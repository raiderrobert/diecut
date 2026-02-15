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
            dry_run,
            verbose,
        } => commands::new::run(
            template, output, data, defaults, overwrite, no_hooks, dry_run, verbose,
        ),
        Commands::List => commands::list::run(),
        Commands::Update {
            path,
            git_ref,
            dry_run,
            verbose,
        } => commands::update::run(path, git_ref, dry_run, verbose),
        Commands::Check { path } => commands::check::run(path),
        Commands::Ready { path } => commands::ready::run(path),
        Commands::Migrate {
            path,
            output,
            dry_run,
        } => commands::migrate::run(path, output, dry_run),
    }
}
