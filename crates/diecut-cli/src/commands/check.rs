use std::path::Path;

use console::style;
use miette::Result;

use diecut_core::check::check_template;

pub fn run(path: String) -> Result<()> {
    let template_dir = Path::new(&path);

    println!(
        "{} {}",
        style("Checking template at").bold(),
        style(template_dir.display()).cyan()
    );

    let result = check_template(template_dir)?;

    println!(
        "  Format: {}",
        match result.format {
            diecut_core::adapter::TemplateFormat::Native => "native (diecut)",
            diecut_core::adapter::TemplateFormat::Cookiecutter => "cookiecutter",
        }
    );
    println!("  Name: {}", result.template_name);
    println!("  Variables: {}", result.variable_count);

    if !result.warnings.is_empty() {
        println!("\n{}", style("Warnings:").yellow().bold());
        for w in &result.warnings {
            println!("  {} {}", style("⚠").yellow(), w);
        }
    }

    if !result.errors.is_empty() {
        println!("\n{}", style("Errors:").red().bold());
        for e in &result.errors {
            println!("  {} {}", style("✗").red(), e);
        }
        println!(
            "\n{} Template has {} error(s)",
            style("✗").red().bold(),
            result.errors.len()
        );
        std::process::exit(1);
    } else {
        println!("\n{} Template is valid!", style("✓").green().bold());
    }

    Ok(())
}
