use std::path::Path;

use console::style;
use diecut_core::update::{update_project, UpdateOptions};
use miette::Result;

pub fn run(path: String, git_ref: Option<String>) -> Result<()> {
    let project_path = Path::new(&path).to_path_buf();

    let project_path = if project_path.is_relative() {
        std::env::current_dir()
            .map_err(|e| miette::miette!("failed to get current directory: {e}"))?
            .join(&project_path)
    } else {
        project_path
    };

    if !project_path.exists() {
        return Err(miette::miette!(
            "Project directory does not exist: {}",
            project_path.display()
        ));
    }

    println!(
        "{} Updating project at {}",
        style("...").cyan().bold(),
        style(project_path.display()).cyan()
    );

    let options = UpdateOptions {
        template_source: None,
        git_ref,
    };

    let report = update_project(&project_path, options)?;

    if !report.has_changes() {
        println!(
            "\n{} Project is already up to date",
            style("✓").green().bold()
        );
        return Ok(());
    }

    println!(
        "\n{} Update complete: {}",
        style("✓").green().bold(),
        report
    );

    if !report.files_updated.is_empty() {
        println!("\n  {} Updated:", style("↻").cyan());
        for f in &report.files_updated {
            println!("    {}", f.display());
        }
    }

    if !report.files_added.is_empty() {
        println!("\n  {} Added:", style("+").green());
        for f in &report.files_added {
            println!("    {}", f.display());
        }
    }

    if !report.files_removed.is_empty() {
        println!(
            "\n  {} Marked for removal (review manually):",
            style("-").red()
        );
        for f in &report.files_removed {
            println!("    {}", f.display());
        }
    }

    if !report.conflicts.is_empty() {
        println!(
            "\n  {} Conflicts (see .rej files):",
            style("!").yellow().bold()
        );
        for f in &report.conflicts {
            println!("    {}", f.display());
        }
    }

    Ok(())
}
