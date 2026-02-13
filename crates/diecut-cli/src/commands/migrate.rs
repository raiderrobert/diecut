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
        // In-place migration: back up the original first
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

        // Copy original to backup
        copy_dir_all(&template_dir, &backup_dir)
            .map_err(|e| miette::miette!("failed to create backup: {e}"))?;

        println!(
            "\n{} Backed up original to {}",
            style("ℹ").blue().bold(),
            style(backup_dir.display()).cyan()
        );

        // Execute migration to a temporary directory, then swap
        let staging = tempfile::tempdir()
            .map_err(|e| miette::miette!("failed to create staging directory: {e}"))?;

        execute_migration(&plan, &template_dir, staging.path())?;

        // Remove original contents and move staged result in
        for entry in std::fs::read_dir(&template_dir)
            .map_err(|e| miette::miette!("reading template directory: {e}"))?
        {
            let entry = entry.map_err(|e| miette::miette!("reading directory entry: {e}"))?;
            let path = entry.path();
            if path.is_dir() {
                std::fs::remove_dir_all(&path)
                    .map_err(|e| miette::miette!("removing {}: {e}", path.display()))?;
            } else {
                std::fs::remove_file(&path)
                    .map_err(|e| miette::miette!("removing {}: {e}", path.display()))?;
            }
        }

        for entry in std::fs::read_dir(staging.path())
            .map_err(|e| miette::miette!("reading staging directory: {e}"))?
        {
            let entry = entry.map_err(|e| miette::miette!("reading directory entry: {e}"))?;
            let dest = template_dir.join(entry.file_name());
            std::fs::rename(entry.path(), &dest).or_else(|_| {
                if entry.path().is_dir() {
                    copy_dir_all(&entry.path(), &dest)?;
                    std::fs::remove_dir_all(entry.path())
                        .map_err(|e| miette::miette!("cleaning staging: {e}"))
                } else {
                    std::fs::copy(entry.path(), &dest)
                        .map(|_| ())
                        .map_err(|e| miette::miette!("copying to destination: {e}"))
                }
            })?;
        }

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

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)
        .map_err(|e| miette::miette!("creating directory {}: {e}", dst.display()))?;

    for entry in std::fs::read_dir(src)
        .map_err(|e| miette::miette!("reading directory {}: {e}", src.display()))?
    {
        let entry = entry.map_err(|e| miette::miette!("reading directory entry: {e}"))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| {
                miette::miette!(
                    "copying {} to {}: {e}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        }
    }

    Ok(())
}
