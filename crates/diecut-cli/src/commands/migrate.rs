use std::path::Path;

use console::style;
use diecut_core::adapter::migrate::{execute_migration, plan_migration, FileOp};
use miette::Result;

pub fn run(path: String, output: Option<String>, dry_run: bool) -> Result<()> {
    let template_dir = Path::new(&path)
        .canonicalize()
        .map_err(|e| miette::miette!("invalid path '{}': {}", path, e))?;

    let plan = plan_migration(&template_dir)?;

    // Print warnings
    for warning in &plan.warnings {
        eprintln!(
            "{} {}",
            style("warning:").yellow().bold(),
            style(warning).yellow()
        );
    }

    // Print plan summary
    println!(
        "\n{} Migration plan for {} template:",
        style("==>").cyan().bold(),
        style(format!("{:?}", plan.source_format)).green()
    );

    let mut move_count = 0;
    let mut create_count = 0;
    let mut delete_count = 0;
    let mut rewrite_count = 0;

    for op in &plan.operations {
        match op {
            FileOp::Move { src, dest } => {
                println!(
                    "  {} {} → {}",
                    style("move").blue(),
                    src.strip_prefix(&template_dir).unwrap_or(src).display(),
                    dest.display()
                );
                move_count += 1;
            }
            FileOp::Create { path, .. } => {
                println!("  {} {}", style("create").green(), path.display());
                create_count += 1;
            }
            FileOp::Delete { path } => {
                println!("  {} {}", style("delete").red(), path.display());
                delete_count += 1;
            }
            FileOp::Rewrite { path, description } => {
                println!(
                    "  {} {} ({})",
                    style("rewrite").yellow(),
                    path.display(),
                    description
                );
                rewrite_count += 1;
            }
        }
    }

    println!(
        "\nSummary: {} moves, {} creates, {} deletes, {} rewrites",
        move_count, create_count, delete_count, rewrite_count
    );

    if dry_run {
        println!(
            "\n{} Dry run — no changes written.",
            style("ℹ").blue().bold()
        );

        // Show the generated diecut.toml
        println!("\n{} Generated diecut.toml:\n", style("==>").cyan().bold());
        println!("{}", plan.diecut_toml_content);

        return Ok(());
    }

    // Determine output directory
    let output_dir = if let Some(out) = &output {
        Path::new(out).to_path_buf()
    } else {
        template_dir.clone()
    };

    if output_dir == template_dir {
        return Err(miette::miette!(
            "in-place migration not yet supported — please use --output to specify a new directory"
        ));
    }

    execute_migration(&plan, &template_dir, &output_dir)?;

    println!(
        "\n{} Template migrated to {}",
        style("✓").green().bold(),
        style(output_dir.display()).cyan()
    );

    Ok(())
}
