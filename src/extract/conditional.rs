use std::path::Path;

/// A known optional file pattern that can be made conditional in the template.
#[derive(Debug, Clone)]
pub struct ConditionalPattern {
    /// Glob pattern to match files.
    pub pattern: &'static str,
    /// Variable name to control inclusion.
    pub variable: &'static str,
    /// Human-readable description.
    pub description: &'static str,
}

/// Curated list of known optional file patterns.
const KNOWN_PATTERNS: &[ConditionalPattern] = &[
    ConditionalPattern {
        pattern: ".github/**",
        variable: "use_github_actions",
        description: "GitHub Actions CI",
    },
    ConditionalPattern {
        pattern: ".gitlab-ci.yml",
        variable: "use_gitlab_ci",
        description: "GitLab CI",
    },
    ConditionalPattern {
        pattern: "Dockerfile",
        variable: "use_docker",
        description: "Docker support",
    },
    ConditionalPattern {
        pattern: "docker-compose.yml",
        variable: "use_docker",
        description: "Docker support",
    },
    ConditionalPattern {
        pattern: "docker-compose.yaml",
        variable: "use_docker",
        description: "Docker support",
    },
    ConditionalPattern {
        pattern: ".pre-commit-config.yaml",
        variable: "use_pre_commit",
        description: "Pre-commit hooks",
    },
    ConditionalPattern {
        pattern: "Makefile",
        variable: "use_make",
        description: "Make build system",
    },
    ConditionalPattern {
        pattern: "Justfile",
        variable: "use_just",
        description: "Just command runner",
    },
    ConditionalPattern {
        pattern: ".editorconfig",
        variable: "use_editorconfig",
        description: "EditorConfig",
    },
    ConditionalPattern {
        pattern: "renovate.json",
        variable: "use_renovate",
        description: "Renovate dependency updates",
    },
    ConditionalPattern {
        pattern: ".renovaterc",
        variable: "use_renovate",
        description: "Renovate dependency updates",
    },
    ConditionalPattern {
        pattern: ".github/dependabot.yml",
        variable: "use_dependabot",
        description: "Dependabot",
    },
    ConditionalPattern {
        pattern: ".husky/**",
        variable: "use_husky",
        description: "Git hooks (JS)",
    },
];

/// A detected conditional file in the project.
#[derive(Debug, Clone)]
pub struct DetectedConditional {
    /// The pattern that matched.
    pub pattern: String,
    /// The variable name to control this pattern.
    pub variable: String,
    /// Human-readable description.
    pub description: String,
}

/// Detect which known optional file patterns exist in the project.
///
/// Groups by variable name — e.g., multiple Docker files share `use_docker`.
pub fn detect_conditional_files(project_dir: &Path) -> Vec<DetectedConditional> {
    let mut detected = Vec::new();
    let mut seen_variables = std::collections::HashSet::new();

    for known in KNOWN_PATTERNS {
        let exists = if known.pattern.contains("**") {
            // Directory pattern — check if the directory exists
            let dir_part = known.pattern.split("/**").next().unwrap_or(known.pattern);
            project_dir.join(dir_part).exists()
        } else {
            project_dir.join(known.pattern).exists()
        };

        if exists && seen_variables.insert(known.variable) {
            detected.push(DetectedConditional {
                pattern: known.pattern.to_string(),
                variable: known.variable.to_string(),
                description: known.description.to_string(),
            });
        }
    }

    detected
}

/// Get all patterns for a given variable name from the known patterns list.
pub fn patterns_for_variable(variable: &str) -> Vec<&'static str> {
    KNOWN_PATTERNS
        .iter()
        .filter(|p| p.variable == variable)
        .map(|p| p.pattern)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_conditional_files_github() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".github/workflows")).unwrap();

        let detected = detect_conditional_files(dir.path());
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].variable, "use_github_actions");
    }

    #[test]
    fn test_detect_conditional_files_docker() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Dockerfile"), "FROM alpine").unwrap();
        std::fs::write(dir.path().join("docker-compose.yml"), "version: '3'").unwrap();

        let detected = detect_conditional_files(dir.path());
        // Should deduplicate by variable name
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].variable, "use_docker");
    }

    #[test]
    fn test_detect_conditional_files_empty() {
        let dir = tempfile::tempdir().unwrap();
        let detected = detect_conditional_files(dir.path());
        assert!(detected.is_empty());
    }

    #[test]
    fn test_patterns_for_variable() {
        let docker_patterns = patterns_for_variable("use_docker");
        assert!(docker_patterns.contains(&"Dockerfile"));
        assert!(docker_patterns.contains(&"docker-compose.yml"));
    }
}
