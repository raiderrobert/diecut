use std::collections::BTreeMap;

use console::style;
use inquire::{Confirm, Select, Text};

use crate::config::schema::DEFAULT_TEMPLATES_SUFFIX;
use crate::error::{DicecutError, Result};

use super::auto_detect::{ConfidenceTier, DetectedCandidate};
use super::conditional::DetectedConditional;
use super::variants::generate_variants;
use super::{ExtractVariable, PlannedExtractFile};

pub fn confirm_variants_interactive(
    variables: Vec<ExtractVariable>,
) -> Result<Vec<ExtractVariable>> {
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

pub fn confirm_excludes_interactive(mut excludes: Vec<String>) -> Result<Vec<String>> {
    eprintln!(
        "\n{} Excludes {}",
        style("──").dim(),
        style("─────────────────────────────────────────────").dim()
    );
    if excludes.is_empty() {
        eprintln!("  No exclude patterns needed for this template.");
    } else {
        eprintln!("  Patterns matching template files:");
        for e in &excludes {
            eprintln!("    {}", e);
        }
    }

    let extra = Text::new("Add extra exclude patterns? (comma-separated, enter to skip)")
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

pub fn confirm_conditionals_interactive(
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

pub fn resolve_candidates_yes(
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
    let mut skipped_freq = 0;

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
        group.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
        let winner = group[0];

        // Skip frequency-analysis candidates in -y mode — too noisy for auto-accept
        if winner.tier == ConfidenceTier::FrequencyAnalysis {
            skipped_freq += 1;
            continue;
        }

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

    if skipped_freq > 0 {
        eprintln!(
            "  {} {} frequency-detected candidate(s) skipped (use interactive mode to review)",
            style("·").dim(),
            skipped_freq
        );
    }

    result
}

pub fn confirm_auto_detected_interactive(
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
        group.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));

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
                    candidate.total_occurrences, candidate.file_count
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

pub fn confirm_files_interactive(files: &[PlannedExtractFile], dropped_count: usize) -> Result<()> {
    let templated: Vec<_> = files.iter().filter(|f| f.has_replacements()).collect();
    let boilerplate: Vec<_> = files
        .iter()
        .filter(|f| !f.has_replacements() && !f.stubbed && !f.is_binary())
        .collect();
    let stubbed: Vec<_> = files.iter().filter(|f| f.stubbed).collect();
    let binary_count = files.iter().filter(|f| f.is_binary()).count();

    eprintln!(
        "\n{} File plan {}",
        style("──").dim(),
        style("──────────────────────────────────────────").dim()
    );

    // Templated files
    eprintln!(
        "\n  {} ({} files, {} suffix):",
        style("Templated").bold(),
        templated.len(),
        DEFAULT_TEMPLATES_SUFFIX
    );
    for file in &templated {
        eprintln!(
            "    {:<50} {} replacements",
            file.template_path.display(),
            file.replacement_count()
        );
    }

    // Boilerplate files
    eprintln!(
        "\n  {} (copied in full, {} files{}):",
        style("Boilerplate").bold(),
        boilerplate.len() + binary_count,
        if binary_count > 0 {
            format!(", {} binary", binary_count)
        } else {
            String::new()
        }
    );
    for file in &boilerplate {
        eprintln!("    {}", file.template_path.display());
    }

    // Stubbed files
    if !stubbed.is_empty() {
        eprintln!(
            "\n  {} (structure only, {} files):",
            style("Stubbed").bold(),
            stubbed.len()
        );
        for file in &stubbed {
            eprintln!("    {}", file.template_path.display());
        }
    }

    // Dropped files
    if dropped_count > 0 {
        eprintln!("\n  {} ({} files):", style("Dropped").bold(), dropped_count);
    }

    let proceed = Confirm::new("Proceed?")
        .with_default(true)
        .prompt()
        .map_err(|_| DicecutError::PromptCancelled)?;

    if !proceed {
        return Err(DicecutError::PromptCancelled);
    }

    Ok(())
}
