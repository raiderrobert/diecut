use console::style;
use miette::Result;

use diecut_core::template::{list_cached, CachedTemplate};

pub fn run() -> Result<()> {
    let entries = list_cached()?;

    if entries.is_empty() {
        println!(
            "No cached templates. Use '{}' with a git URL to cache templates.",
            style("diecut new").cyan()
        );
        return Ok(());
    }

    println!(
        "{} ({} template{})\n",
        style("Cached templates").bold(),
        entries.len(),
        if entries.len() == 1 { "" } else { "s" }
    );

    for entry in &entries {
        print_entry(entry);
    }

    Ok(())
}

fn print_entry(entry: &CachedTemplate) {
    let git_ref = entry.metadata.git_ref.as_deref().unwrap_or("default");

    let cached_at = format_timestamp(&entry.metadata.cached_at);

    println!("  {} {}", style("source:").dim(), entry.metadata.url);
    println!("  {}    {}", style("ref:").dim(), git_ref);
    println!("  {} {}", style("cached:").dim(), cached_at);
    println!();
}

fn format_timestamp(unix_ts: &str) -> String {
    let secs: u64 = match unix_ts.parse() {
        Ok(s) => s,
        Err(_) => return unix_ts.to_string(),
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let elapsed = now.saturating_sub(secs);

    if elapsed < 60 {
        "just now".to_string()
    } else if elapsed < 3600 {
        let mins = elapsed / 60;
        format!("{mins} minute{} ago", if mins == 1 { "" } else { "s" })
    } else if elapsed < 86400 {
        let hours = elapsed / 3600;
        format!("{hours} hour{} ago", if hours == 1 { "" } else { "s" })
    } else {
        let days = elapsed / 86400;
        format!("{days} day{} ago", if days == 1 { "" } else { "s" })
    }
}
