use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "diecut",
    about = "A language-agnostic project template generator",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate a new project from a template
    New {
        /// Template source (local path, or in future: git URL / abbreviation)
        template: String,

        /// Output directory
        #[arg(short, long)]
        output: Option<String>,

        /// Set variable values (can be repeated: -d key=value)
        #[arg(short, long = "data", value_name = "KEY=VALUE")]
        data: Vec<String>,

        /// Use default values without prompting
        #[arg(long)]
        defaults: bool,

        /// Overwrite output directory if it exists
        #[arg(long)]
        overwrite: bool,

        /// Skip running hooks
        #[arg(long)]
        no_hooks: bool,

        /// Show what would be generated without writing files
        #[arg(long)]
        dry_run: bool,

        /// Show file contents (with --dry-run) or detailed output
        #[arg(short, long)]
        verbose: bool,
    },

    /// List cached templates
    List,
}
