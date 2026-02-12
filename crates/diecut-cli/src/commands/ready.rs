use std::path::Path;

use console::style;
use miette::Result;

use diecut_core::ready::check_ready;

pub fn run(path: String) -> Result<()> {
    let template_dir = Path::new(&path);

    println!(
        "{} {}",
        style("Checking distribution readiness for").bold(),
        style(template_dir.display()).cyan()
    );

    let result = check_ready(template_dir)?;

    println!(
        "  Format: {}",
        match result.check.format {
            diecut_core::adapter::TemplateFormat::Native => "native (diecut)",
            diecut_core::adapter::TemplateFormat::Cookiecutter => "cookiecutter",
        }
    );
    println!("  Name: {}", result.check.template_name);
    println!("  Variables: {}", result.check.variable_count);

    if !result.check.warnings.is_empty() {
        println!("\n{}", style("Warnings:").yellow().bold());
        for w in &result.check.warnings {
            println!("  {} {}", style("⚠").yellow(), w);
        }
    }

    if !result.check.errors.is_empty() {
        println!("\n{}", style("Errors:").red().bold());
        for e in &result.check.errors {
            println!("  {} {}", style("✗").red(), e);
        }
    }

    if !result.distribution_warnings.is_empty() {
        println!("\n{}", style("Distribution:").yellow().bold());
        for w in &result.distribution_warnings {
            println!("  {} {}", style("⚠").yellow(), w);
        }
    }

    if result.is_ready() {
        println!(
            "\n{} Template is ready for distribution!",
            style("✓").green().bold()
        );
    } else {
        let total_issues = result.check.errors.len() + result.distribution_warnings.len();
        println!(
            "\n{} Template has {} issue(s) to resolve before distribution",
            style("✗").red().bold(),
            total_issues
        );
        std::process::exit(1);
    }

    Ok(())
}
