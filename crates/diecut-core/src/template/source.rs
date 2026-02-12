use std::path::{Path, PathBuf};

use crate::error::{DicecutError, Result};

/// Resolved template source.
pub enum TemplateSource {
    Local(PathBuf),
    Git {
        url: String,
        git_ref: Option<String>,
    },
}

/// Built-in abbreviation prefixes and their expansion targets.
const ABBREVIATIONS: &[(&str, &str, &str)] = &[
    ("gh:", "https://github.com/", ".git"),
    ("gl:", "https://gitlab.com/", ".git"),
    ("bb:", "https://bitbucket.org/", ".git"),
    ("sr:", "https://git.sr.ht/", ""),
];

/// Expand a shorthand abbreviation (e.g. `gh:user/repo`) into a full git URL.
///
/// Returns `Ok(url)` on successful expansion, or `Err` if the prefix is
/// recognized but the remainder is empty/invalid.
fn expand_abbreviation(input: &str) -> Result<String> {
    for &(prefix, base_url, suffix) in ABBREVIATIONS {
        if let Some(rest) = input.strip_prefix(prefix) {
            if rest.is_empty() {
                return Err(DicecutError::InvalidAbbreviation {
                    input: input.to_string(),
                });
            }
            return Ok(format!("{base_url}{rest}{suffix}"));
        }
    }
    // Not an abbreviation at all — caller should handle this.
    unreachable!("expand_abbreviation called on non-abbreviation input");
}

/// Returns `true` when the argument looks like a known abbreviation prefix.
fn is_abbreviation(input: &str) -> bool {
    ABBREVIATIONS
        .iter()
        .any(|&(prefix, _, _)| input.starts_with(prefix))
}

/// Returns `true` when the argument looks like an explicit git URL.
fn is_git_url(input: &str) -> bool {
    input.starts_with("https://")
        || input.starts_with("http://")
        || input.starts_with("git@")
        || input.ends_with(".git")
}

/// Resolve a template argument to a source.
///
/// Detection order:
/// 1. Known abbreviation prefix (`gh:`, `gl:`, `bb:`, `sr:`) -> expand -> Git source
/// 2. Explicit git URL (`https://`, `git@`, or `.git` suffix) -> Git source
/// 3. Otherwise -> local path (existing behavior)
pub fn resolve_source(template_arg: &str) -> Result<TemplateSource> {
    resolve_source_with_ref(template_arg, None)
}

/// Resolve a template argument to a source, with an optional git ref.
pub fn resolve_source_with_ref(
    template_arg: &str,
    git_ref: Option<&str>,
) -> Result<TemplateSource> {
    // 1. Abbreviation expansion
    if is_abbreviation(template_arg) {
        let url = expand_abbreviation(template_arg)?;
        return Ok(TemplateSource::Git {
            url,
            git_ref: git_ref.map(String::from),
        });
    }

    // 2. Explicit git URL
    if is_git_url(template_arg) {
        return Ok(TemplateSource::Git {
            url: template_arg.to_string(),
            git_ref: git_ref.map(String::from),
        });
    }

    // 3. Local path fallback
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

    // ── Abbreviation expansion ──────────────────────────────────────────

    #[test]
    fn expand_github_abbreviation() {
        let url = expand_abbreviation("gh:user/repo").unwrap();
        assert_eq!(url, "https://github.com/user/repo.git");
    }

    #[test]
    fn expand_gitlab_abbreviation() {
        let url = expand_abbreviation("gl:org/project").unwrap();
        assert_eq!(url, "https://gitlab.com/org/project.git");
    }

    #[test]
    fn expand_bitbucket_abbreviation() {
        let url = expand_abbreviation("bb:team/repo").unwrap();
        assert_eq!(url, "https://bitbucket.org/team/repo.git");
    }

    #[test]
    fn expand_sourcehut_abbreviation() {
        let url = expand_abbreviation("sr:~user/repo").unwrap();
        assert_eq!(url, "https://git.sr.ht/~user/repo");
    }

    #[test]
    fn expand_abbreviation_empty_remainder() {
        let result = expand_abbreviation("gh:");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, DicecutError::InvalidAbbreviation { ref input } if input == "gh:"));
    }

    // ── Git URL detection ───────────────────────────────────────────────

    #[test]
    fn detect_https_url() {
        assert!(is_git_url("https://github.com/user/repo.git"));
    }

    #[test]
    fn detect_git_ssh_url() {
        assert!(is_git_url("git@github.com:user/repo.git"));
    }

    #[test]
    fn detect_dot_git_suffix() {
        assert!(is_git_url("something.git"));
    }

    #[test]
    fn not_git_url_for_plain_path() {
        assert!(!is_git_url("./my-template"));
        assert!(!is_git_url("/home/user/templates/foo"));
    }

    // ── resolve_source ──────────────────────────────────────────────────

    #[test]
    fn resolve_abbreviation_to_git_source() {
        let source = resolve_source("gh:user/repo").unwrap();
        match source {
            TemplateSource::Git { url, git_ref } => {
                assert_eq!(url, "https://github.com/user/repo.git");
                assert!(git_ref.is_none());
            }
            _ => panic!("expected Git source"),
        }
    }

    #[test]
    fn resolve_explicit_https_to_git_source() {
        let source = resolve_source("https://example.com/repo.git").unwrap();
        match source {
            TemplateSource::Git { url, git_ref } => {
                assert_eq!(url, "https://example.com/repo.git");
                assert!(git_ref.is_none());
            }
            _ => panic!("expected Git source"),
        }
    }

    #[test]
    fn resolve_git_ssh_to_git_source() {
        let source = resolve_source("git@github.com:user/repo.git").unwrap();
        match source {
            TemplateSource::Git { url, git_ref } => {
                assert_eq!(url, "git@github.com:user/repo.git");
                assert!(git_ref.is_none());
            }
            _ => panic!("expected Git source"),
        }
    }

    // ── resolve_source_with_ref ─────────────────────────────────────────

    #[test]
    fn resolve_with_ref_sets_git_ref() {
        let source = resolve_source_with_ref("gh:user/repo", Some("v1.0")).unwrap();
        match source {
            TemplateSource::Git { url, git_ref } => {
                assert_eq!(url, "https://github.com/user/repo.git");
                assert_eq!(git_ref.as_deref(), Some("v1.0"));
            }
            _ => panic!("expected Git source"),
        }
    }

    #[test]
    fn resolve_with_ref_none_leaves_ref_none() {
        let source = resolve_source_with_ref("gh:user/repo", None).unwrap();
        match source {
            TemplateSource::Git { url, git_ref } => {
                assert_eq!(url, "https://github.com/user/repo.git");
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
}
