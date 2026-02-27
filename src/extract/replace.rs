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

/// Whether a character is "word-like" for the purpose of boundary detection.
///
/// Alphanumeric, underscore, and hyphen are all considered word characters
/// because they appear as separators in identifiers (kebab-case, snake_case).
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

/// Replace `literal` in `text` only at word boundaries.
///
/// A match is at a word boundary when the characters immediately before and
/// after the match are not word-like (alphanumeric, `_`, or `-`), or the
/// match is at the start/end of the string.
///
/// Multi-word literals (containing a separator like `-`, `_`, or `.`) always
/// use boundary-aware replacement since false positives are unlikely but still
/// possible in paths and compound tokens.
fn replace_whole_word(text: &str, literal: &str, replacement: &str) -> (String, usize) {
    let literal_len = literal.len();
    let text_len = text.len();

    if literal_len == 0 || text_len < literal_len {
        return (text.to_string(), 0);
    }

    let mut result = String::with_capacity(text.len());
    let mut count = 0;
    let mut start = 0;

    while start <= text_len - literal_len {
        match text[start..].find(literal) {
            Some(pos) => {
                let match_start = start + pos;
                let match_end = match_start + literal_len;

                let ok_before = match_start == 0
                    || !is_word_char(text[..match_start].chars().next_back().unwrap());
                let ok_after = match_end == text_len
                    || !is_word_char(text[match_end..].chars().next().unwrap());

                if ok_before && ok_after {
                    result.push_str(&text[start..match_start]);
                    result.push_str(replacement);
                    count += 1;
                    start = match_end;
                } else {
                    // Not a word boundary — advance past the start of this match
                    let next = match_start
                        + text[match_start..]
                            .char_indices()
                            .nth(1)
                            .map(|(i, _)| i)
                            .unwrap_or(1);
                    result.push_str(&text[start..next]);
                    start = next;
                }
            }
            None => break,
        }
    }

    result.push_str(&text[start..]);
    (result, count)
}

/// Apply replacement rules to a string, longest-match-first.
///
/// Uses word-boundary-aware matching to prevent replacing substrings
/// inside longer words (e.g., "app" inside "application").
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
        let (replaced, count) = replace_whole_word(&result, &rule.literal, &rule.replacement);
        if count > 0 {
            result = replaced;
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

    #[test]
    fn test_no_substring_collision_suffix() {
        let rules = vec![make_rule("app", "{{ name }}")];
        let (result, count) = apply_replacements("application startup", &rules);
        assert_eq!(result, "application startup");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_no_substring_collision_prefix() {
        let rules = vec![make_rule("app", "{{ name }}")];
        let (result, count) = apply_replacements("webapp is cool", &rules);
        assert_eq!(result, "webapp is cool");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_standalone_match_with_punctuation() {
        let rules = vec![make_rule("app", "{{ name }}")];
        let (result, count) = apply_replacements("run app. start app!", &rules);
        assert_eq!(result, "run {{ name }}. start {{ name }}!");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_match_at_string_boundaries() {
        let rules = vec![make_rule("app", "{{ name }}")];
        let (result, count) = apply_replacements("app", &rules);
        assert_eq!(result, "{{ name }}");
        assert_eq!(count, 1);
    }

    #[test]
    fn test_compound_literal_still_matches() {
        // Multi-word literals like "my-app" should still match inside strings
        let rules = vec![make_rule("my-app", "{{ name }}")];
        let (result, count) = apply_replacements("name = \"my-app\"", &rules);
        assert_eq!(result, "name = \"{{ name }}\"");
        assert_eq!(count, 1);
    }
}
