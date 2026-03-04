pub mod config_gen;
pub mod exclude;
pub mod replace;
pub mod scan;
pub mod stub;
pub mod variants;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use console::style;

use crate::config::schema::DEFAULT_TEMPLATES_SUFFIX;
use crate::error::{DicecutError, Result};

use self::config_gen::{
    generate_config_toml, ComputedVariable, ConfigGenOptions, PromptedVariable,
};
use self::exclude::{
    all_default_excludes, detect_copy_without_render, is_copy_without_render,
    relevant_config_excludes,
};
use self::replace::{
    apply_path_replacements, apply_replacements, build_replacement_rules, ReplacementRule,
};
use self::scan::{count_occurrences, scan_project};
use self::stub::{classify_file, generate_stub, FileRole};
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

/// The content of an extracted template file.
#[derive(Debug, Clone)]
pub enum ExtractedContent {
    /// A text file with optional template replacements applied.
    Text {
        content: String,
        replacement_count: usize,
    },
    /// A binary file copied verbatim.
    Binary(Vec<u8>),
}

/// A file that will be part of the extracted template.
#[derive(Debug, Clone)]
pub struct PlannedExtractFile {
    /// Relative path in the output template (may contain template expressions).
    pub template_path: PathBuf,
    /// The file content (text with replacements, or binary bytes).
    pub content: ExtractedContent,
    /// Whether this file was stubbed (content replaced with a minimal placeholder).
    pub stubbed: bool,
}

impl PlannedExtractFile {
    /// Whether this file had template replacements applied.
    pub fn has_replacements(&self) -> bool {
        matches!(&self.content, ExtractedContent::Text { replacement_count, .. } if *replacement_count > 0)
    }

    /// Whether this is a binary file.
    pub fn is_binary(&self) -> bool {
        matches!(&self.content, ExtractedContent::Binary(_))
    }

    /// Number of replacements made (0 for binary files).
    pub fn replacement_count(&self) -> usize {
        match &self.content {
            ExtractedContent::Text {
                replacement_count, ..
            } => *replacement_count,
            ExtractedContent::Binary(_) => 0,
        }
    }
}

/// The full extraction plan, ready to be executed or reviewed.
#[derive(Debug)]
pub struct ExtractionPlan {
    pub output_dir: PathBuf,
    pub files: Vec<PlannedExtractFile>,
    pub config_toml: String,
    pub variables: Vec<ExtractVariable>,
    pub exclude_patterns: Vec<String>,
    pub copy_without_render: Vec<String>,
    pub dropped_count: usize,
    pub dropped_paths: Vec<PathBuf>,
}

/// Options for the extraction process.
pub struct ExtractOptions {
    pub source_dir: PathBuf,
    pub variables: Vec<(String, String)>,
    pub output_dir: Option<PathBuf>,
    pub in_place: bool,
    pub stub_depth: usize,
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

    // Phase 1: All default excludes for scanning (safety — never walks into .git/, node_modules/, etc.)
    let scan_excludes = all_default_excludes();

    // Phase 2: Scan project
    eprintln!(
        "\n{}",
        style(format!("Scanning {}...", source_dir.display())).bold()
    );
    let mut scan_result = scan_project(source_dir, &scan_excludes)?;

    // Drop non-boilerplate files deeper than stub_depth before auto-detect sees them.
    // This prevents frequency analysis from detecting variables that only appear in
    // files that would be dropped anyway.
    let pre_filter_count = scan_result.files.len();
    scan_result.files.retain(|f| {
        let depth = f.relative_path.components().count();
        depth <= options.stub_depth
            || classify_file(&f.relative_path, options.stub_depth) == FileRole::Boilerplate
    });
    let depth_dropped = pre_filter_count - scan_result.files.len();

    eprintln!(
        "  {} files found, {} excluded{}",
        scan_result.files.len(),
        scan_result.excluded_count,
        if depth_dropped > 0 {
            format!(", {} too deep", depth_dropped)
        } else {
            String::new()
        }
    );

    // Validate that at least one --var was provided
    let variables = options.variables.clone();
    if variables.is_empty() {
        return Err(DicecutError::ExtractNoVariables);
    }

    // Phase 3: Generate variants and count occurrences
    let mut extract_variables = Vec::new();

    for (var_name, var_value) in &variables {
        let all_variants = generate_variants(var_name, var_value);

        let mut occurrence_counts = Vec::new();
        for variant in &all_variants {
            let (file_count, total_hits) = count_occurrences(&variant.literal, &scan_result);
            occurrence_counts.push((variant.name.to_string(), file_count, total_hits));
        }

        extract_variables.push(ExtractVariable {
            name: var_name.clone(),
            value: var_value.clone(),
            variants: all_variants,
            occurrence_counts,
        });
    }

    // Phase 4: Auto-accept found variants (keep those with occurrences + verbatim)
    let confirmed_variables: Vec<ExtractVariable> = extract_variables
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
        .collect();

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
    let copy_without_render = detect_copy_without_render(&file_paths);

    // Phase 9: Apply replacements to files
    let mut planned_files = Vec::new();
    let mut dropped_count = depth_dropped;
    let mut dropped_paths = Vec::new();

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
                content: ExtractedContent::Binary(binary_content),
                stubbed: false,
            });
        } else if let Some(ref content) = file.content {
            // Lock files and other copy-without-render files: skip replacement
            if is_copy_without_render(&file.relative_path) {
                planned_files.push(PlannedExtractFile {
                    template_path,
                    content: ExtractedContent::Text {
                        content: content.clone(),
                        replacement_count: 0,
                    },
                    stubbed: false,
                });
                continue;
            }

            let (replaced, count) = apply_replacements(content, &rules);

            if count > 0 {
                // Has template replacements — add .die suffix
                let mut p = template_path.as_os_str().to_string_lossy().to_string();
                p.push_str(DEFAULT_TEMPLATES_SUFFIX);
                planned_files.push(PlannedExtractFile {
                    template_path: PathBuf::from(p),
                    content: ExtractedContent::Text {
                        content: replaced,
                        replacement_count: count,
                    },
                    stubbed: false,
                });
            } else {
                // No replacements — classify as boilerplate, content, or dropped
                match classify_file(&file.relative_path, options.stub_depth) {
                    FileRole::Boilerplate => {
                        planned_files.push(PlannedExtractFile {
                            template_path,
                            content: ExtractedContent::Text {
                                content: replaced,
                                replacement_count: 0,
                            },
                            stubbed: false,
                        });
                    }
                    FileRole::Content => {
                        let stub = generate_stub(&file.relative_path);
                        planned_files.push(PlannedExtractFile {
                            template_path,
                            content: ExtractedContent::Text {
                                content: stub,
                                replacement_count: 0,
                            },
                            stubbed: true,
                        });
                    }
                    FileRole::Dropped => {
                        dropped_count += 1;
                        dropped_paths.push(file.relative_path.clone());
                    }
                }
            }
        }
    }

    // Phase 9.5: Compute config-appropriate excludes from planned template files
    // Only patterns that match files actually in the template are worth writing to diecut.toml
    let template_paths: Vec<PathBuf> = planned_files
        .iter()
        .map(|f| f.template_path.clone())
        .collect();
    let config_excludes = relevant_config_excludes(&template_paths);

    // Generate config
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
        exclude_patterns: config_excludes.clone(),
        copy_without_render: copy_without_render.clone(),
        conditional_entries: vec![],
    });

    Ok(ExtractionPlan {
        output_dir,
        files: planned_files,
        config_toml,
        variables: confirmed_variables,
        exclude_patterns: config_excludes,
        copy_without_render,
        dropped_count,
        dropped_paths,
    })
}

/// Execute an extraction plan: write files and config to the output directory.
pub fn execute_extraction(plan: &ExtractionPlan) -> Result<()> {
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
    let mut stubbed_count = 0;

    for file in &plan.files {
        let dest = template_dir.join(&file.template_path);

        // Ensure parent directory exists
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DicecutError::Io {
                context: format!("creating directory {}", parent.display()),
                source: e,
            })?;
        }

        match &file.content {
            ExtractedContent::Text {
                content,
                replacement_count,
            } => {
                std::fs::write(&dest, content).map_err(|e| DicecutError::Io {
                    context: format!("writing file {}", dest.display()),
                    source: e,
                })?;
                if *replacement_count > 0 {
                    rendered_count += 1;
                } else if file.stubbed {
                    stubbed_count += 1;
                } else {
                    copied_count += 1;
                }
            }
            ExtractedContent::Binary(bytes) => {
                std::fs::write(&dest, bytes).map_err(|e| DicecutError::Io {
                    context: format!("writing binary file {}", dest.display()),
                    source: e,
                })?;
                copied_count += 1;
            }
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
        "  {} files templated, {} files copied, {} files stubbed, {} files dropped",
        rendered_count, copied_count, stubbed_count, plan.dropped_count
    );
    eprintln!("  Review diecut.toml to fine-tune");

    Ok(())
}
