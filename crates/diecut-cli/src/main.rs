mod cli;
mod commands;

use clap::Parser;
use cli::{Cli, Commands};

fn main() -> miette::Result<()> {
    match Cli::parse().command {
        Commands::New {
            template,
            output,
            data,
            defaults,
            overwrite,
            no_hooks,
        } => commands::new::run(template, output, data, defaults, overwrite, no_hooks),
        Commands::List => commands::list::run(),
        Commands::Check { path } => commands::check::run(path),
        Commands::Search { query } => commands::search::run(query),
        Commands::Publish { path } => commands::publish::run(path),
        Commands::Migrate {
            path,
            output,
            dry_run,
        } => commands::migrate::run(path, output, dry_run),
    }
}
