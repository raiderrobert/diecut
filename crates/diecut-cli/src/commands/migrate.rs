use std::path::Path;

use console::style;
use diecut_core::adapter::migrate::{execute_migration, plan_migration, FileOp};
use miette::Result;

pub fn run(path: String, output: Option<String>, dry_run: bool) -> Result<()> {
    let template_dir = Path::new(&path)
        .canonicalize()
        .map_err(|e| miette::miette!("invalid path '{}': {}", path, e))?;

    let plan = plan_migration(&template_dir)?;

    for warning in &plan.warnings {
        eprintln!(
            "{} {}",
            style("warning:").yellow().bold(),
            style(warning).yellow()
        );
    }

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

        println!("\n{} Generated diecut.toml:\n", style("==>").cyan().bold());
        println!("{}", plan.diecut_toml_content);

        return Ok(());
    }

    let output_dir = if let Some(out) = &output {
        Path::new(out).to_path_buf()
    } else {
        template_dir.clone()
    };

    if output_dir == template_dir {
        // In-place migration: stage to temp dir, then rename-swap
        let parent = template_dir.parent().ok_or_else(|| {
            miette::miette!(
                "cannot determine parent directory of {}",
                template_dir.display()
            )
        })?;

        let backup_dir = template_dir.with_file_name(format!(
            "{}.pre-migrate",
            template_dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        ));

        if backup_dir.exists() {
            return Err(miette::miette!(
                "backup directory already exists: {} — remove it first or use --output",
                backup_dir.display()
            ));
        }

        let staging = tempfile::tempdir_in(parent)
            .map_err(|e| miette::miette!("failed to create staging directory: {e}"))?;

        execute_migration(&plan, &template_dir, staging.path())?;

        // Atomic swap: rename original → backup, rename staging → original
        std::fs::rename(&template_dir, &backup_dir)
            .map_err(|e| miette::miette!("failed to move original to backup: {e}"))?;

        std::fs::rename(staging.keep(), &template_dir)
            .map_err(|e| miette::miette!("failed to move staged migration into place: {e}"))?;

        println!(
            "\n{} Backed up original to {}",
            style("ℹ").blue().bold(),
            style(backup_dir.display()).cyan()
        );

        println!(
            "\n{} Template migrated in place at {}",
            style("✓").green().bold(),
            style(template_dir.display()).cyan()
        );
    } else {
        execute_migration(&plan, &template_dir, &output_dir)?;

        println!(
            "\n{} Template migrated to {}",
            style("✓").green().bold(),
            style(output_dir.display()).cyan()
        );
    }

    Ok(())
}
