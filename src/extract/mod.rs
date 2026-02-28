pub mod auto_detect;
pub mod conditional;
pub mod config_gen;
pub mod exclude;
pub mod replace;
pub mod scan;
pub mod variants;

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use console::style;
use inquire::{Confirm, Select, Text};

use crate::config::schema::DEFAULT_TEMPLATES_SUFFIX;
use crate::error::{DicecutError, Result};

use self::conditional::{detect_conditional_files, patterns_for_variable, DetectedConditional};
use self::config_gen::{
    generate_config_toml, ComputedVariable, ConditionalEntry, ConfigGenOptions, PromptedVariable,
};
use self::exclude::{detect_copy_without_render, detect_excludes};
use self::replace::{
    apply_path_replacements, apply_replacements, build_replacement_rules, ReplacementRule,
};
use self::auto_detect::{auto_detect, DetectedCandidate};
use self::scan::{scan_project, ScannedFile};
use self::variants::{
    computed_expression, detect_separator, generate_variants, is_canonical_variant, CaseVariant,
};

/// A variable with its value and confirmed case variants.
#[derive(Debug, Clone)]
pub struct ExtractVariable {
    pub name: String,
    pub value: String,
    pub variants: Vec<CaseVariant>,
    /// Per-variant occurrence counts: (variant_name, file_count, total_hits).
    pub occurrence_counts: Vec<(String, usize, usize)>,
}

/// A file that will be part of the extracted template.
#[derive(Debug, Clone)]
pub struct PlannedExtractFile {
    /// Relative path in the output template (may contain template expressions).
    pub template_path: PathBuf,
    /// Content (with replacements applied), or None for binary files.
    pub content: Option<String>,
    /// Original bytes for binary files.
    pub binary_content: Option<Vec<u8>>,
    /// Whether this file had template replacements applied.
    pub has_replacements: bool,
    /// Number of replacements made.
    pub replacement_count: usize,
    /// Whether this is a binary file.
    pub is_binary: bool,
}

/// The full extraction plan, ready to be executed or reviewed.
#[derive(Debug)]
pub struct ExtractionPlan {
    pub output_dir: PathBuf,
    pub files: Vec<PlannedExtractFile>,
    pub config_toml: String,
    pub variables: Vec<ExtractVariable>,
    pub conditional_entries: Vec<ConditionalEntry>,
    pub exclude_patterns: Vec<String>,
    pub copy_without_render: Vec<String>,
}

/// Options for the extraction process.
pub struct ExtractOptions {
    pub source_dir: PathBuf,
    pub variables: Vec<(String, String)>,
    pub output_dir: Option<PathBuf>,
    pub in_place: bool,
    pub yes: bool,
    pub min_confidence: f64,
    pub dry_run: bool,
}

/// Plan an extraction: scan the project, detect variants, build replacement rules.
pub fn plan_extraction(options: &ExtractOptions) -> Result<ExtractionPlan> {
    let source_dir = &options.source_dir;

    if !source_dir.exists() {
        return Err(DicecutError::ExtractSourceNotFound {
            path: source_dir.clone(),
        });
    }

    // Check if this is already a template
    if source_dir.join("diecut.toml").exists() {
        return Err(DicecutError::ExtractAlreadyTemplate {
            path: source_dir.clone(),
        });
    }

    let output_dir = if options.in_place {
        source_dir.clone()
    } else if let Some(ref out) = options.output_dir {
        out.clone()
    } else {
        // Default: source dir name + "-template"
        let dir_name = source_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "template".to_string());
        source_dir
            .parent()
            .unwrap_or(Path::new("."))
            .join(format!("{dir_name}-template"))
    };

    if !options.in_place && output_dir.exists() {
        return Err(DicecutError::ExtractOutputExists {
            path: output_dir.clone(),
        });
    }

    // Phase 1: Detect excludes
    let mut excludes = detect_excludes(source_dir);

    // Phase 2: Scan project
    eprintln!(
        "\n{}",
        style(format!("Scanning {}...", source_dir.display())).bold()
    );
    let scan_result = scan_project(source_dir, &excludes)?;
    eprintln!(
        "  {} files found, {} excluded",
        scan_result.files.len(),
        scan_result.excluded_count
    );

    // Phase 2.5: Auto-detect variables (always runs), merge with explicit --var entries
    let variables = {
        let explicit_vars = options.variables.clone();
        let detect_result = auto_detect(source_dir, &scan_result);

        // Filter candidates below min_confidence threshold
        let candidates: Vec<_> = detect_result
            .candidates
            .into_iter()
            .filter(|c| c.confidence >= options.min_confidence)
            .collect();

        if candidates.is_empty() && explicit_vars.is_empty() {
            return Err(DicecutError::ExtractNoVariables);
        }

        // Resolve auto-detected candidates (merge with explicit vars)
        let auto_vars = if candidates.is_empty() {
            vec![]
        } else if options.yes {
            resolve_candidates_yes(&candidates, &explicit_vars)
        } else {
            confirm_auto_detected_interactive(candidates, &explicit_vars)?
        };

        // Merge: explicit vars first (pre-accepted), then auto-detected additions
        let mut merged = explicit_vars;
        for (name, value) in auto_vars {
            if !merged.iter().any(|(n, _)| n == &name) {
                merged.push((name, value));
            }
        }

        if merged.is_empty() {
            return Err(DicecutError::ExtractNoVariables);
        }

        merged
    };

    // Phase 3: Generate variants and count occurrences
    let mut extract_variables = Vec::new();

    for (var_name, var_value) in &variables {
        let all_variants = generate_variants(var_name, var_value);

        let mut occurrence_counts = Vec::new();
        for variant in &all_variants {
            let (file_count, total_hits) =
                count_variant_occurrences(&variant.literal, &scan_result.files);
            occurrence_counts.push((variant.name.to_string(), file_count, total_hits));
        }

        extract_variables.push(ExtractVariable {
            name: var_name.clone(),
            value: var_value.clone(),
            variants: all_variants,
            occurrence_counts,
        });
    }

    // Phase 4: Interactive variant confirmation
    let confirmed_variables = if options.yes {
        // Batch mode: auto-accept all found variants
        extract_variables
            .into_iter()
            .map(|mut var| {
                var.variants.retain(|v| {
                    var.occurrence_counts
                        .iter()
                        .any(|(name, _, hits)| name == v.name && *hits > 0)
                        || v.name == "verbatim"
                });
                // Always keep at least the verbatim/canonical variant
                if var.variants.is_empty() {
                    let all = generate_variants(&var.name, &var.value);
                    if let Some(first) = all.into_iter().next() {
                        var.variants.push(first);
                    }
                }
                var
            })
            .collect()
    } else {
        confirm_variants_interactive(extract_variables)?
    };

    // Phase 5: Interactive exclude confirmation
    if !options.yes {
        excludes = confirm_excludes_interactive(excludes)?;
    }

    // Phase 6: Detect conditional files
    let detected_conditionals = if options.yes {
        vec![] // Batch mode: no conditional files
    } else {
        let detected = detect_conditional_files(source_dir);
        if detected.is_empty() {
            vec![]
        } else {
            confirm_conditionals_interactive(detected)?
        }
    };

    // Phase 7: Build replacement rules
    let mut rules = Vec::new();
    for var in &confirmed_variables {
        for variant in &var.variants {
            rules.push(ReplacementRule {
                literal: variant.literal.clone(),
                replacement: variant.tera_expr.clone(),
                variable: var.name.clone(),
                variant: variant.name.to_string(),
            });
        }
    }
    build_replacement_rules(&mut rules);

    // Phase 8: Detect copy_without_render patterns
    let file_paths: Vec<PathBuf> = scan_result
        .files
        .iter()
        .map(|f| f.relative_path.clone())
        .collect();
    let copy_without_render = detect_copy_without_render(source_dir, &file_paths);

    // Phase 9: Apply replacements to files
    let mut planned_files = Vec::new();

    for file in &scan_result.files {
        let template_path = apply_path_replacements(&file.relative_path, &rules);

        if file.is_binary {
            let binary_content =
                std::fs::read(&file.absolute_path).map_err(|e| DicecutError::Io {
                    context: format!("reading binary file {}", file.absolute_path.display()),
                    source: e,
                })?;
            planned_files.push(PlannedExtractFile {
                template_path,
                content: None,
                binary_content: Some(binary_content),
                has_replacements: false,
                replacement_count: 0,
                is_binary: true,
            });
        } else if let Some(ref content) = file.content {
            let (replaced, count) = apply_replacements(content, &rules);
            let has_replacements = count > 0;

            // Add .die suffix if file has template replacements
            let final_path = if has_replacements {
                let mut p = template_path.as_os_str().to_string_lossy().to_string();
                p.push_str(DEFAULT_TEMPLATES_SUFFIX);
                PathBuf::from(p)
            } else {
                template_path
            };

            planned_files.push(PlannedExtractFile {
                template_path: final_path,
                content: Some(replaced),
                binary_content: None,
                has_replacements,
                replacement_count: count,
                is_binary: false,
            });
        }
    }

    // Phase 10: Interactive file confirmation
    if !options.yes {
        confirm_files_interactive(&planned_files)?;
    }

    // Phase 11: Build conditional entries
    let conditional_entries: Vec<ConditionalEntry> = detected_conditionals
        .iter()
        .map(|d| {
            let patterns = patterns_for_variable(&d.variable)
                .into_iter()
                .map(|p| p.to_string())
                .collect();
            ConditionalEntry {
                patterns,
                variable: d.variable.clone(),
                description: d.description.clone(),
            }
        })
        .collect();

    // Phase 12: Generate config
    let canonical_seps: HashMap<String, &str> = confirmed_variables
        .iter()
        .map(|v| (v.name.clone(), detect_separator(&v.value)))
        .collect();

    let prompted_vars: Vec<PromptedVariable> = confirmed_variables
        .iter()
        .map(|v| PromptedVariable {
            name: v.name.clone(),
            default_value: v.value.clone(),
            prompt: v.name.replace(['_', '-'], " "),
        })
        .collect();

    let mut computed_vars = Vec::new();
    for var in &confirmed_variables {
        let canonical_sep = canonical_seps.get(&var.name).copied().unwrap_or("-");
        for variant in &var.variants {
            // Skip the canonical variant (it uses the variable directly)
            if variant.name == "verbatim" {
                continue;
            }
            // Skip the variant that matches the canonical separator
            if is_canonical_variant(variant.name, canonical_sep) {
                continue;
            }

            let computed_name = format!("{}_{}", var.name, variant.name);
            let expression = computed_expression(&var.name, variant.name, canonical_sep);
            // Don't add if expression is just the variable name
            if expression != var.name {
                computed_vars.push(ComputedVariable {
                    name: computed_name,
                    expression,
                });
            }
        }
    }

    let config_toml = generate_config_toml(&ConfigGenOptions {
        template_name: source_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "template".to_string()),
        prompted_variables: prompted_vars,
        computed_variables: computed_vars,
        exclude_patterns: excludes.clone(),
        copy_without_render: copy_without_render.clone(),
        conditional_entries: conditional_entries.clone(),
    });

    Ok(ExtractionPlan {
        output_dir,
        files: planned_files,
        config_toml,
        variables: confirmed_variables,
        conditional_entries,
        exclude_patterns: excludes,
        copy_without_render,
    })
}

/// Execute an extraction plan: write files and config to the output directory.
pub fn execute_extraction(plan: &ExtractionPlan, _in_place: bool) -> Result<()> {
    let output_dir = &plan.output_dir;
    let template_dir = output_dir.join("template");

    // Create output structure
    std::fs::create_dir_all(&template_dir).map_err(|e| DicecutError::Io {
        context: format!("creating template directory {}", template_dir.display()),
        source: e,
    })?;

    // Write template files
    let mut rendered_count = 0;
    let mut copied_count = 0;

    for file in &plan.files {
        let dest = template_dir.join(&file.template_path);

        // Ensure parent directory exists
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DicecutError::Io {
                context: format!("creating directory {}", parent.display()),
                source: e,
            })?;
        }

        if let Some(ref content) = file.content {
            std::fs::write(&dest, content).map_err(|e| DicecutError::Io {
                context: format!("writing file {}", dest.display()),
                source: e,
            })?;
            if file.has_replacements {
                rendered_count += 1;
            } else {
                copied_count += 1;
            }
        } else if let Some(ref bytes) = file.binary_content {
            std::fs::write(&dest, bytes).map_err(|e| DicecutError::Io {
                context: format!("writing binary file {}", dest.display()),
                source: e,
            })?;
            copied_count += 1;
        }
    }

    // Write diecut.toml
    let config_path = output_dir.join("diecut.toml");
    std::fs::write(&config_path, &plan.config_toml).map_err(|e| DicecutError::Io {
        context: format!("writing {}", config_path.display()),
        source: e,
    })?;

    // Summary
    let prompted_count = plan.variables.len();
    let computed_count = plan
        .variables
        .iter()
        .flat_map(|v| &v.variants)
        .filter(|variant| {
            variant.name != "verbatim"
                && !matches!(
                    (
                        variant.name,
                        detect_separator(
                            plan.variables
                                .iter()
                                .find(|v2| v2.variants.contains(variant))
                                .map(|v2| v2.value.as_str())
                                .unwrap_or("")
                        )
                    ),
                    ("kebab", "-") | ("snake", "_") | ("dot", ".")
                )
        })
        .count();

    eprintln!(
        "\n{} Template extracted to {}",
        style("✓").green().bold(),
        style(output_dir.display()).cyan()
    );
    eprintln!(
        "  {} variables ({} prompted, {} computed)",
        prompted_count + computed_count,
        prompted_count,
        computed_count
    );
    eprintln!(
        "  {} files templated, {} files copied",
        rendered_count, copied_count
    );
    if !plan.conditional_entries.is_empty() {
        eprintln!(
            "  {} conditional patterns added",
            plan.conditional_entries.len()
        );
    }
    eprintln!("  Review diecut.toml to fine-tune");

    Ok(())
}

// ── Interactive helpers ──────────────────────────────────────────────────

fn count_variant_occurrences(literal: &str, files: &[ScannedFile]) -> (usize, usize) {
    let mut file_count = 0;
    let mut total_hits = 0;

    for file in files {
        if let Some(ref content) = file.content {
            let hits = content.matches(literal).count();
            if hits > 0 {
                file_count += 1;
                total_hits += hits;
            }
        }
    }

    // Also check path components
    for file in files {
        let path_str = file.relative_path.to_string_lossy();
        let hits = path_str.matches(literal).count();
        if hits > 0 {
            // Don't double-count file_count if already counted from content
            total_hits += hits;
        }
    }

    (file_count, total_hits)
}

fn confirm_variants_interactive(variables: Vec<ExtractVariable>) -> Result<Vec<ExtractVariable>> {
    let mut confirmed = Vec::new();

    for mut var in variables {
        eprintln!(
            "\n{} {} = {:?} {}",
            style("──").dim(),
            style(&var.name).bold(),
            var.value,
            style("──────────────────────────────────────").dim()
        );

        if var.variants.len() == 1 && var.variants[0].name == "verbatim" {
            // Simple value — just show occurrence count
            let (file_count, total_hits) = var
                .occurrence_counts
                .first()
                .map(|(_, fc, th)| (*fc, *th))
                .unwrap_or((0, 0));
            if total_hits > 0 {
                eprintln!(
                    "  Found in {} files ({} occurrences)",
                    file_count, total_hits
                );
            } else {
                eprintln!(
                    "  {} Value not found in any file (will still be added to config)",
                    style("⚠").yellow()
                );
            }
            confirmed.push(var);
            continue;
        }

        // Show detected variants with counts
        eprintln!("  Detected case variants:");
        let mut found_any = false;
        for (i, variant) in var.variants.iter().enumerate() {
            let (_, file_count, total_hits) = &var.occurrence_counts[i];
            let mark = if *total_hits > 0 {
                found_any = true;
                style("✓").green().to_string()
            } else {
                style("✗").dim().to_string()
            };
            let hits_str = if *total_hits > 0 {
                format!(
                    "{} {} across {} {}",
                    total_hits,
                    if *total_hits == 1 { "hit" } else { "hits" },
                    file_count,
                    if *file_count == 1 { "file" } else { "files" }
                )
            } else {
                "not found".to_string()
            };
            eprintln!(
                "    {} {:<16} {:<20} {}",
                mark,
                variant.literal,
                variant.name,
                style(&hits_str).dim()
            );
        }

        if !found_any {
            eprintln!(
                "  {} No occurrences found for any variant (will still be added to config)",
                style("⚠").yellow()
            );
            // Keep just the first variant
            var.variants.truncate(1);
            confirmed.push(var);
            continue;
        }

        let keep = Confirm::new("Keep detected variants?")
            .with_default(true)
            .prompt()
            .map_err(|_| DicecutError::PromptCancelled)?;

        if keep {
            // Remove variants with zero occurrences
            let counts = var.occurrence_counts.clone();
            var.variants.retain(|v| {
                counts
                    .iter()
                    .any(|(name, _, hits)| name == v.name && *hits > 0)
            });
            if var.variants.is_empty() {
                let all = generate_variants(&var.name, &var.value);
                if let Some(first) = all.into_iter().next() {
                    var.variants.push(first);
                }
            }
        } else {
            // Keep only the canonical variant
            var.variants.truncate(1);
        }

        confirmed.push(var);
    }

    Ok(confirmed)
}

fn confirm_excludes_interactive(mut excludes: Vec<String>) -> Result<Vec<String>> {
    eprintln!(
        "\n{} Excludes {}",
        style("──").dim(),
        style("─────────────────────────────────────────────").dim()
    );
    eprintln!("  Auto-detected:");
    for e in &excludes {
        eprintln!("    {}", e);
    }

    let extra = Text::new("Add any others? (comma-separated, enter to accept)")
        .with_default("")
        .prompt()
        .map_err(|_| DicecutError::PromptCancelled)?;

    if !extra.is_empty() {
        for pattern in extra.split(',') {
            let trimmed = pattern.trim().to_string();
            if !trimmed.is_empty() {
                excludes.push(trimmed);
            }
        }
    }

    Ok(excludes)
}

fn confirm_conditionals_interactive(
    detected: Vec<DetectedConditional>,
) -> Result<Vec<DetectedConditional>> {
    eprintln!(
        "\n{} Conditional files {}",
        style("──").dim(),
        style("────────────────────────────────────").dim()
    );
    eprintln!("  These look optional. Make them conditional?");

    let mut confirmed = Vec::new();
    for cond in detected {
        let prompt = format!("  {} → {}", cond.pattern, cond.variable);
        let include = Confirm::new(&prompt)
            .with_default(false)
            .prompt()
            .map_err(|_| DicecutError::PromptCancelled)?;

        if include {
            confirmed.push(cond);
        }
    }

    Ok(confirmed)
}

fn resolve_candidates_yes(
    candidates: &[DetectedCandidate],
    explicit_vars: &[(String, String)],
) -> Vec<(String, String)> {
    eprintln!(
        "\n{} Auto-detected variables {}",
        style("──").dim(),
        style("──────────────────────────────────").dim()
    );

    // Group candidates by suggested_name
    let mut groups: BTreeMap<String, Vec<&DetectedCandidate>> = BTreeMap::new();
    for c in candidates {
        groups.entry(c.suggested_name.clone()).or_default().push(c);
    }

    let mut result = Vec::new();

    for (name, mut group) in groups {
        // Skip names already covered by explicit --var
        if explicit_vars.iter().any(|(n, _)| n == &name) {
            eprintln!(
                "  {} {} (explicit --var, skipping auto-detect)",
                style("·").dim(),
                style(&name).dim()
            );
            continue;
        }

        // For name collisions, pick highest confidence
        group.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        let winner = group[0];

        eprintln!(
            "  {} {} = {:?} ({:.0}% confidence, {})",
            style("✓").green(),
            style(&winner.suggested_name).bold(),
            winner.value,
            winner.confidence * 100.0,
            winner.tier
        );
        eprintln!("    {}", style(&winner.reason).dim());

        if group.len() > 1 {
            eprintln!(
                "    {} {} other candidates for this name (picked highest confidence)",
                style("⚠").yellow(),
                group.len() - 1
            );
        }

        result.push((winner.suggested_name.clone(), winner.value.clone()));
    }

    result
}

fn confirm_auto_detected_interactive(
    candidates: Vec<DetectedCandidate>,
    explicit_vars: &[(String, String)],
) -> Result<Vec<(String, String)>> {
    eprintln!(
        "\n{} Auto-detected variables {}",
        style("──").dim(),
        style("──────────────────────────────────").dim()
    );

    // Group candidates by suggested_name
    let mut groups: BTreeMap<String, Vec<DetectedCandidate>> = BTreeMap::new();
    for c in candidates {
        groups.entry(c.suggested_name.clone()).or_default().push(c);
    }

    let mut accepted = Vec::new();

    for (name, mut group) in groups {
        // Skip names already covered by explicit --var
        if explicit_vars.iter().any(|(n, _)| n == &name) {
            eprintln!(
                "\n  {} {} (provided via --var, skipping)",
                style("·").dim(),
                style(&name).dim()
            );
            continue;
        }

        // Sort by confidence descending
        group.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        if group.len() == 1 {
            // Single candidate — simple confirm
            let candidate = &group[0];
            eprintln!(
                "\n  {} = {:?} ({:.0}% confidence, {})",
                style(&candidate.suggested_name).bold(),
                candidate.value,
                candidate.confidence * 100.0,
                candidate.tier
            );
            eprintln!("    {}", style(&candidate.reason).dim());
            if candidate.total_occurrences > 0 {
                eprintln!(
                    "    {} occurrences across {} files",
                    candidate.total_occurrences,
                    candidate.file_count
                );
            }

            let accept = Confirm::new(&format!("Accept \"{}\"?", candidate.suggested_name))
                .with_default(true)
                .prompt()
                .map_err(|_| DicecutError::PromptCancelled)?;

            if accept {
                accepted.push((candidate.suggested_name.clone(), candidate.value.clone()));
            }
        } else {
            // Name collision — show selection prompt
            eprintln!(
                "\n  {} Multiple candidates for {}:",
                style("⚠").yellow(),
                style(&name).bold()
            );

            let mut options: Vec<String> = group
                .iter()
                .map(|c| {
                    format!(
                        "{:?} ({:.0}% confidence, {})",
                        c.value,
                        c.confidence * 100.0,
                        c.tier
                    )
                })
                .collect();
            options.push("Skip".to_string());

            let selection = Select::new(&format!("Which value for \"{}\"?", name), options)
                .prompt()
                .map_err(|_| DicecutError::PromptCancelled)?;

            if selection != "Skip" {
                // Find the matching candidate
                if let Some(chosen) = group.iter().find(|c| {
                    format!(
                        "{:?} ({:.0}% confidence, {})",
                        c.value,
                        c.confidence * 100.0,
                        c.tier
                    ) == selection
                }) {
                    accepted.push((chosen.suggested_name.clone(), chosen.value.clone()));
                }
            }
        }
    }

    Ok(accepted)
}

fn confirm_files_interactive(files: &[PlannedExtractFile]) -> Result<()> {
    let templated: Vec<_> = files.iter().filter(|f| f.has_replacements).collect();
    let copied: Vec<_> = files.iter().filter(|f| !f.has_replacements).collect();
    let binary_count = files.iter().filter(|f| f.is_binary).count();

    eprintln!(
        "\n{} Files to template {}",
        style("──").dim(),
        style("────────────────────────────────────").dim()
    );
    eprintln!(
        "  Will get {} suffix (template replacements made):",
        DEFAULT_TEMPLATES_SUFFIX
    );
    for file in &templated {
        eprintln!(
            "    {:<40} {} replacements",
            file.template_path.display(),
            file.replacement_count
        );
    }

    eprintln!(
        "\n  Copied verbatim: {} files (including {} binary)",
        copied.len(),
        binary_count
    );

    let proceed = Confirm::new("Proceed?")
        .with_default(true)
        .prompt()
        .map_err(|_| DicecutError::PromptCancelled)?;

    if !proceed {
        return Err(DicecutError::PromptCancelled);
    }

    Ok(())
}
