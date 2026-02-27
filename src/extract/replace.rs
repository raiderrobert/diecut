use std::path::{Path, PathBuf};

/// A single replacement rule: find `literal` and replace with `replacement`.
#[derive(Debug, Clone)]
pub struct ReplacementRule {
    pub literal: String,
    pub replacement: String,
    /// Which variable this rule belongs to (for reporting).
    pub variable: String,
    /// Which variant this rule belongs to (for reporting).
    pub variant: String,
}

/// Build replacement rules from all variables and their confirmed variants.
///
/// Rules are sorted by descending literal length so that longest matches apply first.
/// This prevents shorter overlapping matches from corrupting longer ones.
pub fn build_replacement_rules(rules: &mut [ReplacementRule]) {
    rules.sort_by(|a, b| b.literal.len().cmp(&a.literal.len()));
}

/// Apply replacement rules to a string, longest-match-first.
///
/// Returns the modified string and the number of replacements made.
pub fn apply_replacements(content: &str, rules: &[ReplacementRule]) -> (String, usize) {
    if rules.is_empty() {
        return (content.to_string(), 0);
    }

    let mut result = content.to_string();
    let mut total_count = 0;

    for rule in rules {
        if rule.literal.is_empty() {
            continue;
        }
        let count = result.matches(&rule.literal).count();
        if count > 0 {
            result = result.replace(&rule.literal, &rule.replacement);
            total_count += count;
        }
    }

    (result, total_count)
}

/// Apply replacement rules to path components.
///
/// Returns the new path with template expressions in directory and file names.
pub fn apply_path_replacements(path: &Path, rules: &[ReplacementRule]) -> PathBuf {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::Normal(os_str) => {
                let s = os_str.to_string_lossy();
                let (replaced, _) = apply_replacements(&s, rules);
                components.push(replaced);
            }
            other => {
                components.push(other.as_os_str().to_string_lossy().into_owned());
            }
        }
    }

    components.iter().collect()
}

/// Count occurrences of a literal in a string.
pub fn count_occurrences(content: &str, literal: &str) -> usize {
    if literal.is_empty() {
        return 0;
    }
    content.matches(literal).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rule(literal: &str, replacement: &str) -> ReplacementRule {
        ReplacementRule {
            literal: literal.to_string(),
            replacement: replacement.to_string(),
            variable: "test".to_string(),
            variant: "test".to_string(),
        }
    }

    #[test]
    fn test_apply_replacements_basic() {
        let rules = vec![make_rule("my-app", "{{ project_name }}")];
        let (result, count) = apply_replacements("Welcome to my-app!", &rules);
        assert_eq!(result, "Welcome to {{ project_name }}!");
        assert_eq!(count, 1);
    }

    #[test]
    fn test_apply_replacements_multiple() {
        let rules = vec![make_rule("my-app", "{{ project_name }}")];
        let (result, count) = apply_replacements("my-app is great, use my-app", &rules);
        assert_eq!(
            result,
            "{{ project_name }} is great, use {{ project_name }}"
        );
        assert_eq!(count, 2);
    }

    #[test]
    fn test_longest_match_first() {
        let mut rules = vec![
            make_rule("my", "{{ org }}"),
            make_rule("my-app", "{{ project_name }}"),
        ];
        build_replacement_rules(&mut rules);

        // "my-app" should match before "my"
        assert_eq!(rules[0].literal, "my-app");
        assert_eq!(rules[1].literal, "my");
    }

    #[test]
    fn test_apply_replacements_empty_rules() {
        let (result, count) = apply_replacements("hello world", &[]);
        assert_eq!(result, "hello world");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_apply_path_replacements() {
        let rules = vec![make_rule("my-app", "{{ project_name }}")];
        let path = Path::new("my-app/src/main.rs");
        let result = apply_path_replacements(path, &rules);
        assert_eq!(result, PathBuf::from("{{ project_name }}/src/main.rs"));
    }

    #[test]
    fn test_count_occurrences() {
        assert_eq!(count_occurrences("my-app and my-app", "my-app"), 2);
        assert_eq!(count_occurrences("hello world", "missing"), 0);
        assert_eq!(count_occurrences("anything", ""), 0);
    }
}
