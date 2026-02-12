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
    },

    /// List cached templates
    List,

    /// Validate a template directory
    Check {
        /// Path to the template to check (default: current directory)
        #[arg(default_value = ".")]
        path: String,
    },

    /// Search for diecut templates on GitHub
    Search {
        /// Search query
        query: String,
    },

    /// Validate a template and show publishing instructions
    Publish {
        /// Path to the template to publish (default: current directory)
        #[arg(default_value = ".")]
        path: String,
    },

    /// Migrate a foreign template (e.g. cookiecutter) to native diecut format
    Migrate {
        /// Path to the template to migrate (default: current directory)
        #[arg(default_value = ".")]
        path: String,

        /// Write migrated template to a new directory instead of modifying in place
        #[arg(short, long)]
        output: Option<String>,

        /// Show planned changes without writing anything
        #[arg(long)]
        dry_run: bool,
    },
}
