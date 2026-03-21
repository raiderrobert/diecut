use std::path::PathBuf;

use diecut::distill::{self, DistillOptions, DistilledContent};
use miette::Result;

pub fn run(
    projects: Vec<String>,
    vars: Vec<String>,
    output: String,
    depth: usize,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    // Parse --var key=value pairs
    let variables: Vec<(String, String)> = vars
        .iter()
        .map(|v| {
            let parts: Vec<&str> = v.splitn(2, '=').collect();
            if parts.len() != 2 {
                miette::bail!("Invalid --var format '{}': expected KEY=VALUE", v);
            }
            Ok((parts[0].to_string(), parts[1].to_string()))
        })
        .collect::<Result<Vec<_>>>()?;

    // Warn on short values
    for (name, value) in &variables {
        if value.len() < 3 {
            eprintln!(
                "  warning: variable '{}' has short value '{}' — may cause false matches",
                name, value
            );
        }
    }

    // Warn on duplicate values across different variable names
    let mut seen_values: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for (name, value) in &variables {
        if let Some(other_name) = seen_values.get(value.as_str()) {
            eprintln!(
                "  warning: variables '{}' and '{}' have the same value '{}'",
                other_name, name, value
            );
        } else {
            seen_values.insert(value, name);
        }
    }

    let options = DistillOptions {
        projects: projects.iter().map(PathBuf::from).collect(),
        variables,
        output_dir: PathBuf::from(&output),
        max_depth: Some(depth),
        dry_run,
        force,
    };

    let plan = distill::plan_distill(options)?;

    // Print suppressed variable warnings
    for (name, reason) in &plan.suppressed_variables {
        eprintln!("  warning: variable '{}' suppressed — {}", name, reason);
    }

    if dry_run {
        println!("Distill plan (dry run):");
        println!(
            "  Variables: {}",
            plan.active_variables
                .iter()
                .map(|v| v.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!("  Files:");
        for file in &plan.files {
            let suffix = match &file.content {
                DistilledContent::Text {
                    replacement_count, ..
                } => format!(" ({} replacements)", replacement_count),
                DistilledContent::Binary(_) => " (binary)".to_string(),
                DistilledContent::Static(_) => " (static)".to_string(),
            };
            println!("    {}{}", file.template_path.display(), suffix);
        }
        return Ok(());
    }

    distill::execute_distill(&plan)?;
    println!("Template distilled to {}/", output);
    println!(
        "  {} files, {} variables",
        plan.files.len(),
        plan.active_variables.len()
    );

    Ok(())
}
