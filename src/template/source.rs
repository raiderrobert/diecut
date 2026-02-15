use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{DicecutError, Result};

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

/// Built-in abbreviation prefixes and their expansion targets.
const ABBREVIATIONS: &[(&str, &str, &str)] = &[
    ("gh:", "https://github.com/", ".git"),
    ("gl:", "https://gitlab.com/", ".git"),
    ("bb:", "https://bitbucket.org/", ".git"),
];

fn detect_github_protocol() -> String {
    Command::new("gh")
        .args(["config", "get", "git_protocol", "-h", "github.com"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if stdout == "ssh" {
                    return Some("ssh".to_string());
                }
            }
            None
        })
        .unwrap_or_else(|| "https".to_string())
}

fn build_github_url(rest: &str, protocol: &str) -> String {
    if protocol == "ssh" {
        format!("git@github.com:{rest}.git")
    } else {
        format!("https://github.com/{rest}.git")
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

fn expand_abbreviation(input: &str) -> Result<ExpandedSource> {
    // Special case: GitHub abbreviation with protocol detection
    if let Some(rest) = input.strip_prefix("gh:") {
        if rest.is_empty() {
            return Err(DicecutError::InvalidAbbreviation {
                input: input.to_string(),
            });
        }
        let (repo, subpath) = split_repo_subpath(rest);
        let protocol = detect_github_protocol();
        return Ok(ExpandedSource {
            url: build_github_url(repo, &protocol),
            subpath: subpath.map(String::from),
        });
    }

    // All other abbreviations use static expansion
    for &(prefix, base_url, suffix) in ABBREVIATIONS {
        if let Some(rest) = input.strip_prefix(prefix) {
            if rest.is_empty() {
                return Err(DicecutError::InvalidAbbreviation {
                    input: input.to_string(),
                });
            }
            let (repo, subpath) = split_repo_subpath(rest);
            return Ok(ExpandedSource {
                url: format!("{base_url}{repo}{suffix}"),
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

fn is_abbreviation(input: &str) -> bool {
    ABBREVIATIONS
        .iter()
        .any(|&(prefix, _, _)| input.starts_with(prefix))
}

fn is_git_url(input: &str) -> bool {
    input.starts_with("https://")
        || input.starts_with("http://")
        || input.starts_with("git@")
        || input.ends_with(".git")
}

/// Resolve a template argument to a source: abbreviation -> git URL -> local path.
pub fn resolve_source(template_arg: &str) -> Result<TemplateSource> {
    resolve_source_with_ref(template_arg, None)
}

pub fn resolve_source_with_ref(
    template_arg: &str,
    git_ref: Option<&str>,
) -> Result<TemplateSource> {
    resolve_source_full(template_arg, git_ref, None)
}

pub fn resolve_source_full(
    template_arg: &str,
    git_ref: Option<&str>,
    user_abbreviations: Option<&HashMap<String, String>>,
) -> Result<TemplateSource> {
    if let Some(abbrevs) = user_abbreviations {
        if let Some(result) = expand_user_abbreviation(template_arg, abbrevs) {
            let expanded = result?;
            return Ok(TemplateSource::Git {
                url: expanded.url,
                git_ref: git_ref.map(String::from),
                subpath: expanded.subpath,
            });
        }
    }

    if is_abbreviation(template_arg) {
        let expanded = expand_abbreviation(template_arg)?;
        return Ok(TemplateSource::Git {
            url: expanded.url,
            git_ref: git_ref.map(String::from),
            subpath: expanded.subpath,
        });
    }

    if is_git_url(template_arg) {
        return Ok(TemplateSource::Git {
            url: template_arg.to_string(),
            git_ref: git_ref.map(String::from),
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

    // ── Abbreviation expansion ──────────────────────────────────────────

    #[test]
    fn build_github_url_ssh() {
        let url = build_github_url("user/repo", "ssh");
        assert_eq!(url, "git@github.com:user/repo.git");
    }

    #[test]
    fn build_github_url_https() {
        let url = build_github_url("user/repo", "https");
        assert_eq!(url, "https://github.com/user/repo.git");
    }

    #[test]
    fn expand_github_abbreviation() {
        let expanded = expand_abbreviation("gh:user/repo").unwrap();
        assert!(
            expanded.url == "https://github.com/user/repo.git"
                || expanded.url == "git@github.com:user/repo.git",
            "unexpected URL: {}",
            expanded.url
        );
        assert!(expanded.subpath.is_none());
    }

    #[rstest]
    #[case("gl:org/project", "https://gitlab.com/org/project.git")]
    #[case("bb:team/repo", "https://bitbucket.org/team/repo.git")]
    fn expand_abbreviation_cases(#[case] input: &str, #[case] expected_url: &str) {
        let expanded = expand_abbreviation(input).unwrap();
        assert_eq!(expanded.url, expected_url);
        assert!(expanded.subpath.is_none());
    }

    #[test]
    fn expand_abbreviation_empty_remainder() {
        let result = expand_abbreviation("gh:");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, DicecutError::InvalidAbbreviation { ref input } if input == "gh:"));
    }

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

    // ── resolve_source ──────────────────────────────────────────────────

    #[test]
    fn resolve_abbreviation_to_git_source() {
        let source = resolve_source("gh:user/repo").unwrap();
        match source {
            TemplateSource::Git {
                url,
                git_ref,
                subpath,
            } => {
                assert!(
                    url == "https://github.com/user/repo.git"
                        || url == "git@github.com:user/repo.git",
                    "unexpected URL: {url}"
                );
                assert!(git_ref.is_none());
                assert!(subpath.is_none());
            }
            _ => panic!("expected Git source"),
        }
    }

    #[test]
    fn resolve_explicit_https_to_git_source() {
        let source = resolve_source("https://example.com/repo.git").unwrap();
        match source {
            TemplateSource::Git {
                url,
                git_ref,
                subpath,
            } => {
                assert_eq!(url, "https://example.com/repo.git");
                assert!(git_ref.is_none());
                assert!(subpath.is_none());
            }
            _ => panic!("expected Git source"),
        }
    }

    #[test]
    fn resolve_git_ssh_to_git_source() {
        let source = resolve_source("git@github.com:user/repo.git").unwrap();
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

    // ── resolve_source_with_ref ─────────────────────────────────────────

    #[test]
    fn resolve_with_ref_sets_git_ref() {
        let source = resolve_source_with_ref("gh:user/repo", Some("v1.0")).unwrap();
        match source {
            TemplateSource::Git { url, git_ref, .. } => {
                assert!(
                    url == "https://github.com/user/repo.git"
                        || url == "git@github.com:user/repo.git",
                    "unexpected URL: {url}"
                );
                assert_eq!(git_ref.as_deref(), Some("v1.0"));
            }
            _ => panic!("expected Git source"),
        }
    }

    #[test]
    fn resolve_with_ref_none_leaves_ref_none() {
        let source = resolve_source_with_ref("gh:user/repo", None).unwrap();
        match source {
            TemplateSource::Git { url, git_ref, .. } => {
                assert!(
                    url == "https://github.com/user/repo.git"
                        || url == "git@github.com:user/repo.git",
                    "unexpected URL: {url}"
                );
                assert!(git_ref.is_none());
            }
            _ => panic!("expected Git source"),
        }
    }

    // ── Local path fallback ─────────────────────────────────────────────

    #[test]
    fn resolve_existing_local_path() {
        // Use the cargo manifest dir which is guaranteed to exist.
        let dir = env!("CARGO_MANIFEST_DIR");
        let source = resolve_source(dir).unwrap();
        match source {
            TemplateSource::Local(path) => {
                assert!(path.exists());
            }
            _ => panic!("expected Local source"),
        }
    }

    #[test]
    fn resolve_nonexistent_local_path_errors() {
        let result = resolve_source("/nonexistent/path/that/does/not/exist");
        assert!(result.is_err());
    }

    // ── User abbreviations ──────────────────────────────────────────────

    #[test]
    fn user_abbreviation_expands_correctly() {
        let mut abbrevs = HashMap::new();
        abbrevs.insert(
            "company".to_string(),
            "https://git.company.com/{}.git".to_string(),
        );
        let source = resolve_source_full("company:team/project", None, Some(&abbrevs)).unwrap();
        match source {
            TemplateSource::Git {
                url,
                git_ref,
                subpath,
            } => {
                assert_eq!(url, "https://git.company.com/team/project.git");
                assert!(git_ref.is_none());
                assert!(subpath.is_none());
            }
            _ => panic!("expected Git source"),
        }
    }

    #[test]
    fn user_abbreviation_with_ref() {
        let mut abbrevs = HashMap::new();
        abbrevs.insert(
            "corp".to_string(),
            "https://git.corp.com/{}.git".to_string(),
        );
        let source = resolve_source_full("corp:myrepo", Some("v2.0"), Some(&abbrevs)).unwrap();
        match source {
            TemplateSource::Git { url, git_ref, .. } => {
                assert_eq!(url, "https://git.corp.com/myrepo.git");
                assert_eq!(git_ref.as_deref(), Some("v2.0"));
            }
            _ => panic!("expected Git source"),
        }
    }

    #[test]
    fn user_abbreviation_takes_priority_over_builtin() {
        let mut abbrevs = HashMap::new();
        abbrevs.insert(
            "gh".to_string(),
            "https://custom-github.example.com/{}.git".to_string(),
        );
        let source = resolve_source_full("gh:user/repo", None, Some(&abbrevs)).unwrap();
        match source {
            TemplateSource::Git { url, .. } => {
                assert_eq!(url, "https://custom-github.example.com/user/repo.git");
            }
            _ => panic!("expected Git source"),
        }
    }

    #[test]
    fn user_abbreviation_empty_remainder_errors() {
        let mut abbrevs = HashMap::new();
        abbrevs.insert(
            "company".to_string(),
            "https://git.company.com/{}.git".to_string(),
        );
        let result = resolve_source_full("company:", None, Some(&abbrevs));
        assert!(result.is_err());
    }

    #[test]
    fn unknown_user_abbreviation_falls_through_to_builtin() {
        let mut abbrevs = HashMap::new();
        abbrevs.insert(
            "company".to_string(),
            "https://git.company.com/{}.git".to_string(),
        );
        let source = resolve_source_full("gh:user/repo", None, Some(&abbrevs)).unwrap();
        match source {
            TemplateSource::Git { url, .. } => {
                assert!(
                    url == "https://github.com/user/repo.git"
                        || url == "git@github.com:user/repo.git",
                    "unexpected URL: {url}"
                );
            }
            _ => panic!("expected Git source"),
        }
    }

    #[test]
    fn no_user_abbreviations_behaves_as_before() {
        let source = resolve_source_full("gh:user/repo", None, None).unwrap();
        match source {
            TemplateSource::Git { url, .. } => {
                assert!(
                    url == "https://github.com/user/repo.git"
                        || url == "git@github.com:user/repo.git",
                    "unexpected URL: {url}"
                );
            }
            _ => panic!("expected Git source"),
        }
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

    #[test]
    fn resolve_abbreviation_with_subpath() {
        let source = resolve_source("gh:user/repo/my-template").unwrap();
        match source {
            TemplateSource::Git {
                url,
                subpath,
                git_ref,
            } => {
                assert!(
                    url == "https://github.com/user/repo.git"
                        || url == "git@github.com:user/repo.git",
                    "unexpected URL: {url}"
                );
                assert_eq!(subpath.as_deref(), Some("my-template"));
                assert!(git_ref.is_none());
            }
            _ => panic!("expected Git source"),
        }
    }

    #[test]
    fn resolve_abbreviation_with_nested_subpath() {
        let source = resolve_source("gl:org/repo/templates/python").unwrap();
        match source {
            TemplateSource::Git { url, subpath, .. } => {
                assert_eq!(url, "https://gitlab.com/org/repo.git");
                assert_eq!(subpath.as_deref(), Some("templates/python"));
            }
            _ => panic!("expected Git source"),
        }
    }

    #[test]
    fn user_abbreviation_with_subpath() {
        let mut abbrevs = HashMap::new();
        abbrevs.insert(
            "company".to_string(),
            "https://git.company.com/{}.git".to_string(),
        );
        let source =
            resolve_source_full("company:team/project/subdir", None, Some(&abbrevs)).unwrap();
        match source {
            TemplateSource::Git { url, subpath, .. } => {
                assert_eq!(url, "https://git.company.com/team/project.git");
                assert_eq!(subpath.as_deref(), Some("subdir"));
            }
            _ => panic!("expected Git source"),
        }
    }
}
