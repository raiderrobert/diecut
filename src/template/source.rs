use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{DicecutError, Result};

/// Protocol used when expanding built-in shortcodes (`gh:`/`gl:`/`cb:`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum GitProtocol {
    #[default]
    Ssh,
    Https,
}

impl std::str::FromStr for GitProtocol {
    type Err = DicecutError;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "ssh" => Ok(GitProtocol::Ssh),
            "https" => Ok(GitProtocol::Https),
            other => Err(DicecutError::InvalidProtocol {
                value: other.to_string(),
                config_key: "DIECUT_GIT_PROTOCOL",
            }),
        }
    }
}

/// Resolve the git protocol to use for shortcode expansion.
///
/// Precedence: CLI flag > `DIECUT_GIT_PROTOCOL` env var > built-in default (SSH).
pub fn resolve_git_protocol(cli_flag: Option<GitProtocol>) -> Result<GitProtocol> {
    if let Some(p) = cli_flag {
        return Ok(p);
    }
    match std::env::var("DIECUT_GIT_PROTOCOL") {
        Ok(value) => value.parse(),
        Err(_) => Ok(GitProtocol::default()),
    }
}

/// Resolved template source.
pub enum TemplateSource {
    Local(PathBuf),
    Git {
        url: String,
        git_ref: Option<String>,
        /// Subdirectory within the repo that contains the template.
        subpath: Option<String>,
    },
}

/// Options passed to [`resolve_source`].
#[derive(Debug, Default)]
pub struct ResolveOptions<'a> {
    /// Optional git ref (branch, tag, or commit) to check out.
    pub git_ref: Option<&'a str>,
    /// Protocol to use when expanding built-in shortcodes.
    pub protocol: GitProtocol,
    /// Optional user-defined abbreviations to consult before built-in shortcodes.
    pub user_abbreviations: Option<&'a HashMap<String, String>>,
}

/// A built-in shortcode → host mapping.
struct VendorShortcode {
    prefix: &'static str,
    host: &'static str,
}

const SHORTCODES: &[VendorShortcode] = &[
    VendorShortcode {
        prefix: "gh:",
        host: "github.com",
    },
    VendorShortcode {
        prefix: "gl:",
        host: "gitlab.com",
    },
    VendorShortcode {
        prefix: "cb:",
        host: "codeberg.org",
    },
];

/// Build a clone URL for a given host, repo, and protocol.
fn build_url(host: &str, repo: &str, protocol: GitProtocol) -> String {
    match protocol {
        GitProtocol::Ssh => format!("git@{host}:{repo}.git"),
        GitProtocol::Https => format!("https://{host}/{repo}.git"),
    }
}

/// Split an abbreviation remainder like "user/repo/some/path" into
/// the repo part ("user/repo") and an optional subpath ("some/path").
fn split_repo_subpath(rest: &str) -> (&str, Option<&str>) {
    let mut segments = 0;
    let mut split_at = rest.len();
    for (i, c) in rest.char_indices() {
        if c == '/' {
            segments += 1;
            if segments == 2 {
                split_at = i;
                break;
            }
        }
    }
    if split_at < rest.len() {
        let subpath = &rest[split_at + 1..];
        if subpath.is_empty() {
            (&rest[..split_at], None)
        } else {
            (&rest[..split_at], Some(subpath))
        }
    } else {
        (rest, None)
    }
}

#[derive(Debug)]
struct ExpandedSource {
    url: String,
    subpath: Option<String>,
}

/// Expand a built-in shortcode (`gh:`/`gl:`/`cb:`) into a git URL.
fn expand_abbreviation(input: &str, protocol: GitProtocol) -> Result<ExpandedSource> {
    for VendorShortcode { prefix, host } in SHORTCODES {
        if let Some(rest) = input.strip_prefix(prefix) {
            if rest.is_empty() {
                return Err(DicecutError::InvalidAbbreviation {
                    input: input.to_string(),
                });
            }
            let (repo, subpath) = split_repo_subpath(rest);
            return Ok(ExpandedSource {
                url: build_url(host, repo, protocol),
                subpath: subpath.map(String::from),
            });
        }
    }
    Err(DicecutError::InvalidAbbreviation {
        input: input.to_string(),
    })
}

fn expand_user_abbreviation(
    input: &str,
    abbreviations: &HashMap<String, String>,
) -> Option<Result<ExpandedSource>> {
    let (prefix, rest) = input.split_once(':')?;

    let url_template = abbreviations.get(prefix)?;

    if rest.is_empty() {
        return Some(Err(DicecutError::InvalidAbbreviation {
            input: input.to_string(),
        }));
    }

    let (repo, subpath) = split_repo_subpath(rest);
    Some(Ok(ExpandedSource {
        url: url_template.replace("{}", repo),
        subpath: subpath.map(String::from),
    }))
}

/// Check if an input string starts with any built-in shortcode prefix.
fn is_abbreviation(input: &str) -> bool {
    SHORTCODES.iter().any(|s| input.starts_with(s.prefix))
}

fn is_git_url(input: &str) -> bool {
    input.starts_with("https://")
        || input.starts_with("http://")
        || input.starts_with("git@")
        || input.ends_with(".git")
}

/// Resolve a template argument to a [`TemplateSource`].
///
/// Handles user abbreviations, built-in shortcodes, explicit git URLs, and local paths.
pub fn resolve_source(template_arg: &str, opts: ResolveOptions<'_>) -> Result<TemplateSource> {
    if let Some(abbrevs) = opts.user_abbreviations {
        if let Some(result) = expand_user_abbreviation(template_arg, abbrevs) {
            let expanded = result?;
            return Ok(TemplateSource::Git {
                url: expanded.url,
                git_ref: opts.git_ref.map(String::from),
                subpath: expanded.subpath,
            });
        }
    }

    if is_abbreviation(template_arg) {
        let expanded = expand_abbreviation(template_arg, opts.protocol)?;
        return Ok(TemplateSource::Git {
            url: expanded.url,
            git_ref: opts.git_ref.map(String::from),
            subpath: expanded.subpath,
        });
    }

    if is_git_url(template_arg) {
        return Ok(TemplateSource::Git {
            url: template_arg.to_string(),
            git_ref: opts.git_ref.map(String::from),
            subpath: None,
        });
    }

    let path = Path::new(template_arg);
    if path.exists() {
        Ok(TemplateSource::Local(path.canonicalize().map_err(|e| {
            DicecutError::Io {
                context: format!("resolving path {}", path.display()),
                source: e,
            }
        })?))
    } else {
        Err(DicecutError::ConfigNotFound {
            path: path.to_path_buf(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // ── Git URL detection ───────────────────────────────────────────────

    #[rstest]
    #[case("https://github.com/user/repo.git", true)]
    #[case("git@github.com:user/repo.git", true)]
    #[case("something.git", true)]
    #[case("./my-template", false)]
    #[case("/home/user/templates/foo", false)]
    fn is_git_url_cases(#[case] input: &str, #[case] expected: bool) {
        assert_eq!(is_git_url(input), expected);
    }

    // ── resolve_source with ResolveOptions ──────────────────────────────

    #[test]
    fn resolve_source_with_options_local() {
        let dir = env!("CARGO_MANIFEST_DIR");
        let opts = ResolveOptions::default();
        let source = resolve_source(dir, opts).unwrap();
        assert!(matches!(source, TemplateSource::Local(_)));
    }

    #[test]
    fn resolve_source_with_options_shortcode_ssh_default() {
        let opts = ResolveOptions::default();
        let source = resolve_source("gh:user/repo", opts).unwrap();
        match source {
            TemplateSource::Git {
                url,
                git_ref,
                subpath,
            } => {
                assert_eq!(url, "git@github.com:user/repo.git");
                assert!(git_ref.is_none());
                assert!(subpath.is_none());
            }
            _ => panic!("expected Git source"),
        }
    }

    #[test]
    fn resolve_source_with_options_shortcode_https() {
        let opts = ResolveOptions {
            protocol: GitProtocol::Https,
            ..Default::default()
        };
        let source = resolve_source("gh:user/repo", opts).unwrap();
        match source {
            TemplateSource::Git { url, .. } => {
                assert_eq!(url, "https://github.com/user/repo.git");
            }
            _ => panic!("expected Git source"),
        }
    }

    #[test]
    fn resolve_source_with_options_ref() {
        let opts = ResolveOptions {
            git_ref: Some("v1.0"),
            ..Default::default()
        };
        let source = resolve_source("gh:user/repo", opts).unwrap();
        match source {
            TemplateSource::Git { git_ref, .. } => {
                assert_eq!(git_ref.as_deref(), Some("v1.0"));
            }
            _ => panic!("expected Git source"),
        }
    }

    #[test]
    fn resolve_source_with_options_shortcode_subpath() {
        let opts = ResolveOptions::default();
        let source = resolve_source("gh:user/repo/my-template", opts).unwrap();
        match source {
            TemplateSource::Git { url, subpath, .. } => {
                assert_eq!(url, "git@github.com:user/repo.git");
                assert_eq!(subpath.as_deref(), Some("my-template"));
            }
            _ => panic!("expected Git source"),
        }
    }

    #[test]
    fn resolve_source_with_options_plain_url() {
        let opts = ResolveOptions::default();
        let source = resolve_source("https://example.com/repo.git", opts).unwrap();
        match source {
            TemplateSource::Git { url, subpath, .. } => {
                assert_eq!(url, "https://example.com/repo.git");
                assert!(subpath.is_none());
            }
            _ => panic!("expected Git source"),
        }
    }

    #[test]
    fn resolve_source_nonexistent_local_path_errors() {
        let opts = ResolveOptions::default();
        let result = resolve_source("/nonexistent/path/that/does/not/exist", opts);
        assert!(result.is_err());
    }

    // ── Subpath parsing ────────────────────────────────────────────────

    #[rstest]
    #[case("user/repo", "user/repo", None)]
    #[case("user/repo/template-a", "user/repo", Some("template-a"))]
    #[case("user/repo/templates/python", "user/repo", Some("templates/python"))]
    #[case("user/repo/", "user/repo", None)]
    fn split_repo_subpath_cases(
        #[case] input: &str,
        #[case] exp_repo: &str,
        #[case] exp_sub: Option<&str>,
    ) {
        let (repo, sub) = split_repo_subpath(input);
        assert_eq!(repo, exp_repo);
        assert_eq!(sub, exp_sub);
    }

    // ── GitProtocol ────────────────────────────────────────────────────

    #[test]
    fn git_protocol_parses_ssh() {
        let p: GitProtocol = "ssh".parse().unwrap();
        assert_eq!(p, GitProtocol::Ssh);
    }

    #[test]
    fn git_protocol_parses_https() {
        let p: GitProtocol = "https".parse().unwrap();
        assert_eq!(p, GitProtocol::Https);
    }

    #[test]
    fn git_protocol_rejects_unknown() {
        let result: Result<GitProtocol> = "http".parse();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            DicecutError::InvalidProtocol { ref value, .. } if value == "http"
        ));
    }

    #[test]
    fn git_protocol_default_is_ssh() {
        assert_eq!(GitProtocol::default(), GitProtocol::Ssh);
    }

    // ── Shortcode expansion (protocol-aware) ───────────────────────────

    #[rstest]
    #[case("gh:user/repo", GitProtocol::Ssh, "git@github.com:user/repo.git")]
    #[case("gh:user/repo", GitProtocol::Https, "https://github.com/user/repo.git")]
    #[case("gl:org/project", GitProtocol::Ssh, "git@gitlab.com:org/project.git")]
    #[case(
        "gl:org/project",
        GitProtocol::Https,
        "https://gitlab.com/org/project.git"
    )]
    #[case("cb:user/repo", GitProtocol::Ssh, "git@codeberg.org:user/repo.git")]
    #[case(
        "cb:user/repo",
        GitProtocol::Https,
        "https://codeberg.org/user/repo.git"
    )]
    fn expand_shortcode_per_protocol(
        #[case] input: &str,
        #[case] protocol: GitProtocol,
        #[case] expected_url: &str,
    ) {
        let expanded = expand_abbreviation(input, protocol).unwrap();
        assert_eq!(expanded.url, expected_url);
        assert!(expanded.subpath.is_none());
    }

    #[rstest]
    #[case(
        "gh:user/repo/templates/py",
        GitProtocol::Ssh,
        "git@github.com:user/repo.git",
        "templates/py"
    )]
    #[case(
        "gl:org/repo/templates/python",
        GitProtocol::Https,
        "https://gitlab.com/org/repo.git",
        "templates/python"
    )]
    #[case(
        "cb:user/repo/sub",
        GitProtocol::Ssh,
        "git@codeberg.org:user/repo.git",
        "sub"
    )]
    fn expand_shortcode_with_subpath(
        #[case] input: &str,
        #[case] protocol: GitProtocol,
        #[case] expected_url: &str,
        #[case] expected_subpath: &str,
    ) {
        let expanded = expand_abbreviation(input, protocol).unwrap();
        assert_eq!(expanded.url, expected_url);
        assert_eq!(expanded.subpath.as_deref(), Some(expected_subpath));
    }

    #[test]
    fn expand_shortcode_empty_remainder_errors() {
        let result = expand_abbreviation("gh:", GitProtocol::Ssh);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DicecutError::InvalidAbbreviation { ref input } if input == "gh:"
        ));
    }

    #[test]
    fn expand_shortcode_unknown_prefix_errors() {
        let result = expand_abbreviation("xx:user/repo", GitProtocol::Ssh);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod protocol_resolution_tests {
    use super::*;
    use serial_test::serial;

    fn clear_env() {
        std::env::remove_var("DIECUT_GIT_PROTOCOL");
    }

    #[test]
    #[serial]
    fn no_flag_no_env_defaults_to_ssh() {
        clear_env();
        let result = resolve_git_protocol(None).unwrap();
        assert_eq!(result, GitProtocol::Ssh);
    }

    #[test]
    #[serial]
    fn flag_ssh_returns_ssh() {
        clear_env();
        let result = resolve_git_protocol(Some(GitProtocol::Ssh)).unwrap();
        assert_eq!(result, GitProtocol::Ssh);
    }

    #[test]
    #[serial]
    fn flag_https_returns_https() {
        clear_env();
        let result = resolve_git_protocol(Some(GitProtocol::Https)).unwrap();
        assert_eq!(result, GitProtocol::Https);
    }

    #[test]
    #[serial]
    fn env_https_returns_https_when_no_flag() {
        clear_env();
        std::env::set_var("DIECUT_GIT_PROTOCOL", "https");
        let result = resolve_git_protocol(None).unwrap();
        clear_env();
        assert_eq!(result, GitProtocol::Https);
    }

    #[test]
    #[serial]
    fn env_ssh_returns_ssh_when_no_flag() {
        clear_env();
        std::env::set_var("DIECUT_GIT_PROTOCOL", "ssh");
        let result = resolve_git_protocol(None).unwrap();
        clear_env();
        assert_eq!(result, GitProtocol::Ssh);
    }

    #[test]
    #[serial]
    fn flag_overrides_env() {
        clear_env();
        std::env::set_var("DIECUT_GIT_PROTOCOL", "ssh");
        let result = resolve_git_protocol(Some(GitProtocol::Https)).unwrap();
        clear_env();
        assert_eq!(result, GitProtocol::Https);
    }

    #[test]
    #[serial]
    fn invalid_env_value_errors() {
        clear_env();
        std::env::set_var("DIECUT_GIT_PROTOCOL", "tcp");
        let result = resolve_git_protocol(None);
        clear_env();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DicecutError::InvalidProtocol { ref value, .. } if value == "tcp"
        ));
    }

    #[test]
    #[serial]
    fn empty_env_value_errors() {
        clear_env();
        std::env::set_var("DIECUT_GIT_PROTOCOL", "");
        let result = resolve_git_protocol(None);
        clear_env();
        assert!(result.is_err());
    }
}
