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
        Commands::Update { path, git_ref } => commands::update::run(path, git_ref),
        Commands::Check { path } => commands::check::run(path),
        Commands::Migrate {
            path,
            output,
            dry_run,
        } => commands::migrate::run(path, output, dry_run),
    }
}
