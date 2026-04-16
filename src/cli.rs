use clap::{Parser, Subcommand};
use diecut::template::GitProtocol;

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

        /// Protocol for expanding shortcodes (ssh or https).
        /// Defaults to ssh. Override with DIECUT_GIT_PROTOCOL env var.
        #[arg(long, value_enum)]
        protocol: Option<GitProtocol>,
    },

    /// List cached templates
    List,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use diecut::template::GitProtocol;

    #[test]
    fn parses_new_without_protocol() {
        let cli = Cli::parse_from(["diecut", "new", "gh:user/repo"]);
        if let Commands::New { protocol, .. } = cli.command {
            assert!(protocol.is_none());
        } else {
            panic!("expected New");
        }
    }

    #[test]
    fn parses_new_with_protocol_ssh() {
        let cli = Cli::parse_from(["diecut", "new", "gh:user/repo", "--protocol", "ssh"]);
        if let Commands::New { protocol, .. } = cli.command {
            assert_eq!(protocol, Some(GitProtocol::Ssh));
        } else {
            panic!("expected New");
        }
    }

    #[test]
    fn parses_new_with_protocol_https() {
        let cli = Cli::parse_from(["diecut", "new", "gh:user/repo", "--protocol", "https"]);
        if let Commands::New { protocol, .. } = cli.command {
            assert_eq!(protocol, Some(GitProtocol::Https));
        } else {
            panic!("expected New");
        }
    }

    #[test]
    fn rejects_invalid_protocol() {
        let result = Cli::try_parse_from(["diecut", "new", "gh:user/repo", "--protocol", "ftp"]);
        assert!(result.is_err());
    }
}
