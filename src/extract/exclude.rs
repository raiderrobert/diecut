use std::path::Path;

/// Default directories and files to exclude from template extraction.
const DEFAULT_EXCLUDES: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    ".DS_Store",
    "Thumbs.db",
    "__pycache__",
    "*.pyc",
    ".tox",
    ".nox",
    ".mypy_cache",
    ".ruff_cache",
    ".pytest_cache",
    "target",
    ".venv",
    ".env",
    "dist",
    "build",
    ".next",
    ".nuxt",
    ".output",
    ".turbo",
    ".worktrees",
    ".claude/worktrees",
    ".astro",
    ".diecut-answers.toml",
];

/// Return all default exclude patterns for use during scanning.
///
/// All DEFAULT_EXCLUDES are always used during the scan phase because patterns
/// like `node_modules` can appear at any depth (e.g. `docs/node_modules/`).
pub fn all_default_excludes() -> Vec<String> {
    DEFAULT_EXCLUDES.iter().map(|s| s.to_string()).collect()
}

/// Check if a path should be excluded based on the exclude patterns.
pub fn should_exclude(relative_path: &Path, excludes: &[String]) -> bool {
    let path_str = relative_path.to_string_lossy();

    for pattern in excludes {
        let clean = pattern.trim_end_matches('/');

        if clean.contains('*') {
            // Glob-style matching: *.pyc matches any .pyc file
            if let Some(ext) = clean.strip_prefix("*.") {
                if let Some(file_ext) = relative_path.extension() {
                    if file_ext.to_string_lossy().eq_ignore_ascii_case(ext) {
                        return true;
                    }
                }
            }
            continue;
        }

        // Exact directory/file match at any level
        for component in relative_path.components() {
            if let std::path::Component::Normal(os_str) = component {
                if os_str.to_string_lossy() == clean {
                    return true;
                }
            }
        }

        // Full path match
        if path_str == clean || path_str.starts_with(&format!("{clean}/")) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_should_exclude_git() {
        let excludes = vec![".git/".to_string()];
        assert!(should_exclude(Path::new(".git/config"), &excludes));
        assert!(should_exclude(Path::new(".git/HEAD"), &excludes));
    }

    #[test]
    fn test_should_exclude_node_modules() {
        let excludes = vec!["node_modules".to_string()];
        assert!(should_exclude(
            Path::new("node_modules/express/index.js"),
            &excludes
        ));
    }

    #[test]
    fn test_should_exclude_glob() {
        let excludes = vec!["*.pyc".to_string()];
        assert!(should_exclude(
            Path::new("module/__pycache__/foo.pyc"),
            &excludes
        ));
        assert!(!should_exclude(Path::new("module/foo.py"), &excludes));
    }

    #[test]
    fn test_should_not_exclude_normal_file() {
        let excludes = vec![".git/".to_string(), "node_modules".to_string()];
        assert!(!should_exclude(Path::new("src/main.rs"), &excludes));
        assert!(!should_exclude(Path::new("README.md"), &excludes));
    }
}
