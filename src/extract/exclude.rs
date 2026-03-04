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

/// Return all default exclude patterns for use during scanning.
///
/// All DEFAULT_EXCLUDES are always used during the scan phase because patterns
/// like `node_modules` can appear at any depth (e.g. `docs/node_modules/`).
pub fn all_default_excludes() -> Vec<String> {
    DEFAULT_EXCLUDES.iter().map(|s| s.to_string()).collect()
}

/// Return only the DEFAULT_EXCLUDES patterns that match at least one file in the
/// template output. These are the patterns worth writing to `diecut.toml`'s
/// `[files] exclude` — directory patterns like `.git/` or `node_modules/` that
/// were filtered during scan are omitted since those files never appear in the
/// template.
pub fn relevant_config_excludes(template_files: &[std::path::PathBuf]) -> Vec<String> {
    let all = all_default_excludes();
    all.into_iter()
        .filter(|pattern| {
            template_files
                .iter()
                .any(|f| should_exclude(f, std::slice::from_ref(pattern)))
        })
        .collect()
}

/// Detect which copy-without-render patterns are relevant based on files present.
pub fn detect_copy_without_render(files: &[std::path::PathBuf]) -> Vec<String> {
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

/// Check if a file should be copied without rendering (lock files, binary-like assets).
///
/// These files are included in the template but should never have replacements
/// applied during extraction — they're copied verbatim.
pub fn is_copy_without_render(path: &Path) -> bool {
    for pattern in DEFAULT_COPY_WITHOUT_RENDER {
        if let Some(ext) = pattern.strip_prefix("*.") {
            if let Some(file_ext) = path.extension() {
                if file_ext.to_string_lossy().eq_ignore_ascii_case(ext) {
                    return true;
                }
            }
        } else if let Some(file_name) = path.file_name() {
            if file_name.to_string_lossy() == *pattern {
                return true;
            }
        }
    }
    false
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
    fn test_all_default_excludes() {
        let found = all_default_excludes();
        // All DEFAULT_EXCLUDES are always included
        assert!(found.iter().any(|e| e.contains(".git")));
        assert!(found.iter().any(|e| e == ".DS_Store"));
        assert!(found.iter().any(|e| e == "*.pyc"));
        assert!(found.iter().any(|e| e.contains("node_modules")));
    }

    #[test]
    fn test_relevant_config_excludes_empty_when_no_matches() {
        // Typical template files won't match any DEFAULT_EXCLUDES
        let files = vec![
            PathBuf::from("src/main.rs"),
            PathBuf::from("README.md"),
            PathBuf::from("Cargo.toml"),
        ];
        let relevant = relevant_config_excludes(&files);
        assert!(relevant.is_empty());
    }

    #[test]
    fn test_relevant_config_excludes_finds_matching_patterns() {
        let files = vec![
            PathBuf::from("src/main.py"),
            PathBuf::from("src/__pycache__/main.pyc"),
            PathBuf::from(".DS_Store"),
        ];
        let relevant = relevant_config_excludes(&files);
        assert!(relevant.contains(&"*.pyc".to_string()));
        assert!(relevant.contains(&".DS_Store".to_string()));
        assert!(relevant.contains(&"__pycache__".to_string()));
        // Directory excludes that don't match should not appear
        assert!(!relevant.contains(&".git".to_string()));
        assert!(!relevant.contains(&"node_modules".to_string()));
    }

    #[test]
    fn test_should_exclude_claude_worktrees() {
        let excludes = all_default_excludes();
        assert!(should_exclude(
            Path::new(".claude/worktrees/agent-abc/Cargo.toml"),
            &excludes
        ));
        // .claude/settings.local.json should NOT be excluded
        assert!(!should_exclude(
            Path::new(".claude/settings.local.json"),
            &excludes
        ));
    }

    #[test]
    fn test_should_exclude_astro() {
        let excludes = all_default_excludes();
        assert!(should_exclude(
            Path::new("docs/.astro/data-store.json"),
            &excludes
        ));
        assert!(should_exclude(Path::new(".astro/settings.json"), &excludes));
    }

    #[test]
    fn test_is_copy_without_render() {
        assert!(is_copy_without_render(Path::new("Cargo.lock")));
        assert!(is_copy_without_render(Path::new("pnpm-lock.yaml")));
        assert!(is_copy_without_render(Path::new("package-lock.json")));
        assert!(is_copy_without_render(Path::new("logo.png")));
        assert!(is_copy_without_render(Path::new("deep/nested/file.lock")));
        assert!(!is_copy_without_render(Path::new("src/main.rs")));
        assert!(!is_copy_without_render(Path::new("README.md")));
    }

    #[test]
    fn test_detect_copy_without_render() {
        let files = vec![
            PathBuf::from("logo.png"),
            PathBuf::from("font.woff2"),
            PathBuf::from("README.md"),
        ];
        let found = detect_copy_without_render(&files);
        assert!(found.contains(&"*.png".to_string()));
        assert!(found.contains(&"*.woff2".to_string()));
        assert!(!found.contains(&"*.jpg".to_string()));
    }
}
