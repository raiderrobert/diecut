use std::path::Path;

/// Default directories and files to exclude from template extraction.
const DEFAULT_EXCLUDES: &[&str] = &[
    ".git",
    ".git/",
    ".hg",
    ".svn",
    "node_modules",
    "node_modules/",
    ".DS_Store",
    "Thumbs.db",
    "__pycache__",
    "__pycache__/",
    "*.pyc",
    ".tox",
    ".nox",
    ".mypy_cache",
    ".ruff_cache",
    ".pytest_cache",
    "target",
    "target/",
    ".venv",
    ".env",
    "dist",
    "build",
    ".next",
    ".nuxt",
    ".output",
    ".turbo",
    ".worktrees",
    ".diecut-answers.toml",
];

/// Patterns for files that should be copied without rendering (binary-like or problematic).
const DEFAULT_COPY_WITHOUT_RENDER: &[&str] = &[
    "*.png",
    "*.jpg",
    "*.jpeg",
    "*.gif",
    "*.ico",
    "*.svg",
    "*.webp",
    "*.woff",
    "*.woff2",
    "*.ttf",
    "*.eot",
    "*.otf",
    "*.zip",
    "*.tar",
    "*.gz",
    "*.bz2",
    "*.xz",
    "*.pdf",
    "*.lock",
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "Cargo.lock",
    "Gemfile.lock",
    "poetry.lock",
    "composer.lock",
];

/// Detect which default exclude patterns actually exist in the project.
///
/// All DEFAULT_EXCLUDES are always included because patterns like `node_modules`
/// can appear at any depth (e.g. `docs/node_modules/`), not just the project root.
pub fn detect_excludes(_project_dir: &Path) -> Vec<String> {
    DEFAULT_EXCLUDES.iter().map(|s| s.to_string()).collect()
}

/// Detect which copy-without-render patterns are relevant based on files present.
pub fn detect_copy_without_render(
    _project_dir: &Path,
    files: &[std::path::PathBuf],
) -> Vec<String> {
    let mut found = Vec::new();

    for pattern in DEFAULT_COPY_WITHOUT_RENDER {
        if pattern.starts_with('*') {
            // Extension pattern — check if any file matches
            let ext = pattern.trim_start_matches("*.");
            if files.iter().any(|f| {
                f.extension()
                    .map(|e| e.to_string_lossy().eq_ignore_ascii_case(ext))
                    .unwrap_or(false)
            }) {
                found.push(pattern.to_string());
            }
        } else {
            // Exact filename — check if present
            if files.iter().any(|f| {
                f.file_name()
                    .map(|n| n.to_string_lossy() == *pattern)
                    .unwrap_or(false)
            }) {
                found.push(pattern.to_string());
            }
        }
    }

    found
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
    use std::path::PathBuf;

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

    #[test]
    fn test_detect_excludes() {
        let dir = tempfile::tempdir().unwrap();

        let found = detect_excludes(dir.path());
        // All DEFAULT_EXCLUDES are always included regardless of what exists on disk
        assert!(found.iter().any(|e| e.contains(".git")));
        assert!(found.iter().any(|e| e == ".DS_Store"));
        assert!(found.iter().any(|e| e == "*.pyc"));
        assert!(found.iter().any(|e| e.contains("node_modules")));
    }

    #[test]
    fn test_detect_copy_without_render() {
        let files = vec![
            PathBuf::from("logo.png"),
            PathBuf::from("font.woff2"),
            PathBuf::from("README.md"),
        ];
        let found = detect_copy_without_render(Path::new("."), &files);
        assert!(found.contains(&"*.png".to_string()));
        assert!(found.contains(&"*.woff2".to_string()));
        assert!(!found.contains(&"*.jpg".to_string()));
    }
}
