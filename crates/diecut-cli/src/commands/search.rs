use console::style;
use miette::Result;

use diecut_core::registry::search_github;

pub fn run(query: String) -> Result<()> {
    println!(
        "{} {}",
        style("Searching templates for").bold(),
        style(&query).cyan()
    );
    println!();

    let entries = search_github(&query)?;

    if entries.is_empty() {
        println!(
            "{}",
            style("No templates found. Try a different search term.").yellow()
        );
        return Ok(());
    }

    for entry in &entries {
        println!(
            "{} by {}",
            style(&entry.name).green().bold(),
            style(&entry.author).cyan()
        );
        if !entry.description.is_empty() {
            println!("  {}", entry.description);
        }
        if !entry.tags.is_empty() {
            let tags: Vec<_> = entry
                .tags
                .iter()
                .filter(|t| *t != "diecut-template")
                .collect();
            if !tags.is_empty() {
                println!(
                    "  Tags: {}",
                    tags.iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }
        println!("  Install: diecut new gh:{}/{}", entry.author, entry.name);
        println!();
    }

    println!(
        "{} Found {} template(s)",
        style("✓").green().bold(),
        entries.len()
    );

    Ok(())
}
