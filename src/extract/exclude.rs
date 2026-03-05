use std::path::Path;

const DEFAULT_EXCLUDES: &str = include_str!("default_excludes.txt");

/// Load exclude patterns from a file, or use the built-in defaults.
pub fn load_excludes(override_file: Option<&Path>) -> Vec<String> {
    let text = match override_file {
        Some(path) => {
            std::fs::read_to_string(path).unwrap_or_else(|_| DEFAULT_EXCLUDES.to_string())
        }
        None => DEFAULT_EXCLUDES.to_string(),
    };
    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect()
}

/// Check if a path should be excluded based on the exclude patterns.
pub fn should_exclude(relative_path: &Path, excludes: &[String]) -> bool {
    let path_str = relative_path.to_string_lossy();

    for pattern in excludes {
        let clean = pattern.trim_end_matches('/');

        if let Some(ext) = clean.strip_prefix("*.") {
            if let Some(file_ext) = relative_path.extension() {
                if file_ext.to_string_lossy().eq_ignore_ascii_case(ext) {
                    return true;
                }
            }
            continue;
        }

        for component in relative_path.components() {
            if let std::path::Component::Normal(os_str) = component {
                if os_str.to_string_lossy() == clean {
                    return true;
                }
            }
        }

        if path_str == clean || path_str.starts_with(&format!("{clean}/")) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_defaults() {
        let excludes = load_excludes(None);
        assert!(excludes.contains(&".git".to_string()));
        assert!(excludes.contains(&"target".to_string()));
        assert!(excludes.contains(&".DS_Store".to_string()));
        assert!(!excludes.iter().any(|e| e.starts_with('#')));
    }

    #[test]
    fn test_should_exclude_matches() {
        let excludes = vec![".git".to_string(), "*.pyc".to_string()];
        assert!(should_exclude(Path::new(".git/HEAD"), &excludes));
        assert!(should_exclude(Path::new("pkg/foo.pyc"), &excludes));
        assert!(!should_exclude(Path::new("src/main.rs"), &excludes));
    }

    #[test]
    fn test_override_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("excludes.txt");
        std::fs::write(&file, "# custom\nvendor\n*.log\n").unwrap();

        let excludes = load_excludes(Some(&file));
        assert_eq!(excludes, vec!["vendor", "*.log"]);
    }
}
