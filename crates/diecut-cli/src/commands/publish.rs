use std::path::Path;

use console::style;
use miette::Result;

use diecut_core::check::check_template;

pub fn run(path: String) -> Result<()> {
    let template_dir = Path::new(&path);

    println!(
        "{} {}",
        style("Validating template at").bold(),
        style(template_dir.display()).cyan()
    );

    let result = check_template(template_dir)?;

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
            "\n{} Template has {} error(s) — fix them before publishing",
            style("✗").red().bold(),
            result.errors.len()
        );
        std::process::exit(1);
    }

    println!("\n{} Template is valid!\n", style("✓").green().bold());
    println!("To publish your template:");
    println!("  1. Push to a GitHub repository");
    println!(
        "  2. Add the topic '{}' to your repo (Settings → Topics)",
        style("diecut-template").cyan()
    );
    println!("  3. Others can discover it: diecut search <name>");
    println!("  4. Others can use it:     diecut new gh:<owner>/<repo>");

    Ok(())
}
