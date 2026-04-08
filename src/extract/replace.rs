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

/// Apply replacement rules to a string, longest-match-first, in a single pass.
///
/// All match positions are identified first against the original text, then
/// applied in one pass so that replacement output is never re-scanned by later
/// rules. Uses word-boundary-aware matching to prevent replacing substrings
/// inside longer words (e.g., "app" inside "application").
///
/// Returns the modified string and the number of replacements made.
pub fn apply_replacements(content: &str, rules: &[ReplacementRule]) -> (String, usize) {
    if rules.is_empty() {
        return (content.to_string(), 0);
    }

    // Collect all (start, end, replacement_index) matches across all rules.
    let mut matches: Vec<(usize, usize, usize)> = Vec::new();

    for (rule_idx, rule) in rules.iter().enumerate() {
        if rule.literal.is_empty() {
            continue;
        }
        let literal = &rule.literal;
        let literal_len = literal.len();
        let text_len = content.len();

        if text_len < literal_len {
            continue;
        }

        let mut start = 0;
        while start <= text_len - literal_len {
            match content[start..].find(literal) {
                Some(pos) => {
                    let match_start = start + pos;
                    let match_end = match_start + literal_len;

                    let ok_before = match_start == 0
                        || !is_word_char(content[..match_start].chars().next_back().unwrap());
                    let ok_after = match_end == text_len
                        || !is_word_char(content[match_end..].chars().next().unwrap());

                    if ok_before && ok_after {
                        matches.push((match_start, match_end, rule_idx));
                    }

                    let next = match_start
                        + content[match_start..]
                            .char_indices()
                            .nth(1)
                            .map(|(i, _)| i)
                            .unwrap_or(1);
                    start = next;
                }
                None => break,
            }
        }
    }

    if matches.is_empty() {
        return (content.to_string(), 0);
    }

    // Sort by start position; on tie, prefer the longer match (lower rule index
    // already means longer literal due to build_replacement_rules sorting).
    matches.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));

    // Greedily select non-overlapping matches.
    let mut result = String::with_capacity(content.len());
    let mut total_count = 0;
    let mut cursor = 0;

    for (m_start, m_end, rule_idx) in &matches {
        if *m_start < cursor {
            continue; // overlaps with a previously accepted match
        }
        result.push_str(&content[cursor..*m_start]);
        result.push_str(&rules[*rule_idx].replacement);
        total_count += 1;
        cursor = *m_end;
    }
    result.push_str(&content[cursor..]);

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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::path::Path;

    /// Helper to build a single rule with minimal boilerplate.
    fn rule(literal: &str, replacement: &str) -> ReplacementRule {
        ReplacementRule {
            literal: literal.to_string(),
            replacement: replacement.to_string(),
            variable: "var".to_string(),
            variant: "verbatim".to_string(),
        }
    }

    /// Helper to build and sort a rule set, ready for apply_replacements.
    fn sorted(rules: Vec<ReplacementRule>) -> Vec<ReplacementRule> {
        let mut rules = rules;
        build_replacement_rules(&mut rules);
        rules
    }

    // ── is_word_char ──────────────────────────────────────────────

    #[rstest]
    #[case('a', true)]
    #[case('Z', true)]
    #[case('0', true)]
    #[case('_', true)]
    #[case('-', true)]
    #[case(' ', false)]
    #[case('.', false)]
    #[case('/', false)]
    #[case('{', false)]
    #[case('é', true)] // alphanumeric per char::is_alphanumeric
    fn word_char(#[case] c: char, #[case] expected: bool) {
        assert_eq!(is_word_char(c), expected, "is_word_char({c:?})");
    }

    // ── build_replacement_rules (sorting) ─────────────────────────

    #[test]
    fn sorts_longest_literal_first() {
        let mut rules = vec![
            rule("app", "{{ x }}"),
            rule("my-app", "{{ y }}"),
            rule("a", "{{ z }}"),
        ];
        build_replacement_rules(&mut rules);

        let lengths: Vec<usize> = rules.iter().map(|r| r.literal.len()).collect();
        assert_eq!(lengths, vec![6, 3, 1]);
    }

    // ── apply_replacements: basic ─────────────────────────────────

    #[test]
    fn no_rules_returns_original() {
        let (out, count) = apply_replacements("hello world", &[]);
        assert_eq!(out, "hello world");
        assert_eq!(count, 0);
    }

    #[test]
    fn no_match_returns_original() {
        let rules = sorted(vec![rule("missing", "{{ x }}")]);
        let (out, count) = apply_replacements("hello world", &rules);
        assert_eq!(out, "hello world");
        assert_eq!(count, 0);
    }

    #[test]
    fn simple_replacement() {
        let rules = sorted(vec![rule("my-app", "{{ project_name }}")]);
        let (out, count) = apply_replacements("name = \"my-app\"", &rules);
        assert_eq!(out, "name = \"{{ project_name }}\"");
        assert_eq!(count, 1);
    }

    #[test]
    fn multiple_occurrences() {
        let rules = sorted(vec![rule("foo", "{{ x }}")]);
        let (out, count) = apply_replacements("foo and foo again foo", &rules);
        assert_eq!(out, "{{ x }} and {{ x }} again {{ x }}");
        assert_eq!(count, 3);
    }

    #[test]
    fn empty_content() {
        let rules = sorted(vec![rule("x", "{{ x }}")]);
        let (out, count) = apply_replacements("", &rules);
        assert_eq!(out, "");
        assert_eq!(count, 0);
    }

    #[test]
    fn empty_literal_is_skipped() {
        let rules = vec![rule("", "{{ x }}")];
        let (out, count) = apply_replacements("hello", &rules);
        assert_eq!(out, "hello");
        assert_eq!(count, 0);
    }

    // ── apply_replacements: word boundaries ───────────────────────

    #[test]
    fn no_match_inside_longer_word() {
        let rules = sorted(vec![rule("app", "{{ name }}")]);
        let (out, count) = apply_replacements("the application is great", &rules);
        assert_eq!(out, "the application is great");
        assert_eq!(count, 0);
    }

    #[test]
    fn no_match_with_prefix_attached() {
        let rules = sorted(vec![rule("app", "{{ name }}")]);
        let (out, count) = apply_replacements("myapp works", &rules);
        assert_eq!(out, "myapp works");
        assert_eq!(count, 0);
    }

    #[test]
    fn no_match_with_suffix_attached() {
        let rules = sorted(vec![rule("app", "{{ name }}")]);
        let (out, count) = apply_replacements("apps are great", &rules);
        assert_eq!(out, "apps are great");
        assert_eq!(count, 0);
    }

    #[rstest]
    #[case("app is here", "{{ n }} is here", 1)]
    #[case("use app", "use {{ n }}", 1)]
    #[case("app", "{{ n }}", 1)]
    #[case("(app)", "({{ n }})", 1)]
    #[case("\"app\"", "\"{{ n }}\"", 1)]
    #[case("app.config", "{{ n }}.config", 1)]
    #[case("/app/", "/{{ n }}/", 1)]
    fn boundary_at_non_word_chars(
        #[case] input: &str,
        #[case] expected: &str,
        #[case] expected_count: usize,
    ) {
        let rules = sorted(vec![rule("app", "{{ n }}")]);
        let (out, count) = apply_replacements(input, &rules);
        assert_eq!(out, expected, "input: {input:?}");
        assert_eq!(count, expected_count);
    }

    #[test]
    fn hyphen_is_word_boundary_blocker() {
        // "my-app" contains "app" but hyphen is a word char, so "app" alone
        // should NOT match inside "my-app".
        let rules = sorted(vec![rule("app", "{{ name }}")]);
        let (out, count) = apply_replacements("my-app", &rules);
        assert_eq!(out, "my-app");
        assert_eq!(count, 0);
    }

    #[test]
    fn underscore_is_word_boundary_blocker() {
        let rules = sorted(vec![rule("app", "{{ name }}")]);
        let (out, count) = apply_replacements("my_app", &rules);
        assert_eq!(out, "my_app");
        assert_eq!(count, 0);
    }

    // ── apply_replacements: longest-match-first / overlap ─────────

    #[test]
    fn longest_match_wins() {
        let rules = sorted(vec![
            rule("my-app", "{{ full }}"),
            rule("my", "{{ prefix }}"),
        ]);
        let (out, count) = apply_replacements("name: my-app", &rules);
        assert_eq!(out, "name: {{ full }}");
        assert_eq!(count, 1);
    }

    #[test]
    fn shorter_rule_still_matches_elsewhere() {
        let rules = sorted(vec![
            rule("my-app", "{{ full }}"),
            rule("my", "{{ prefix }}"),
        ]);
        let (out, count) = apply_replacements("my-app by my", &rules);
        assert_eq!(out, "{{ full }} by {{ prefix }}");
        assert_eq!(count, 2);
    }

    #[test]
    fn adjacent_non_overlapping_matches() {
        // Two rules that match at adjacent positions separated by a dot.
        let rules = sorted(vec![rule("foo", "{{ a }}"), rule("bar", "{{ b }}")]);
        let (out, count) = apply_replacements("foo.bar", &rules);
        assert_eq!(out, "{{ a }}.{{ b }}");
        assert_eq!(count, 2);
    }

    // ── apply_replacements: no re-scanning ────────────────────────

    #[test]
    fn replacement_output_is_not_rescanned() {
        // If re-scanning occurred, the "x" in "{{ x }}" could trigger rule 2.
        let rules = sorted(vec![rule("foo", "{{ x }}"), rule("x", "WRONG")]);
        let (out, count) = apply_replacements("foo", &rules);
        assert_eq!(out, "{{ x }}");
        assert_eq!(count, 1);
    }

    // ── apply_replacements: unicode ───────────────────────────────

    #[test]
    fn unicode_content_preserved() {
        // CJK chars are alphanumeric (is_word_char → true), so the literal
        // must appear at a non-word boundary to match.
        let rules = sorted(vec![rule("my-app", "{{ name }}")]);
        let (out, count) = apply_replacements("プロジェクト: my-app です", &rules);
        assert_eq!(out, "プロジェクト: {{ name }} です");
        assert_eq!(count, 1);
    }

    #[test]
    fn cjk_neighbors_block_boundary() {
        // CJK characters are alphanumeric → word chars, so a literal
        // flanked by them is not at a word boundary.
        let rules = sorted(vec![rule("名前", "{{ name }}")]);
        let (out, count) = apply_replacements("私の名前はアプリです", &rules);
        assert_eq!(out, "私の名前はアプリです");
        assert_eq!(count, 0);
    }

    #[test]
    fn multibyte_boundary_respected() {
        // "café" contains "é" which is alphanumeric → word char.
        // Rule for "caf" should NOT match inside "café".
        let rules = sorted(vec![rule("caf", "{{ x }}")]);
        let (out, count) = apply_replacements("café", &rules);
        assert_eq!(out, "café");
        assert_eq!(count, 0);
    }

    // ── apply_path_replacements ───────────────────────────────────

    #[test]
    fn replaces_in_path_components() {
        let rules = sorted(vec![rule("my-app", "{{ name }}")]);
        let path = Path::new("src/my-app/main.rs");
        let result = apply_path_replacements(path, &rules);
        assert_eq!(result, PathBuf::from("src/{{ name }}/main.rs"));
    }

    #[test]
    fn replaces_in_filename() {
        let rules = sorted(vec![rule("my-app", "{{ name }}")]);
        let path = Path::new("my-app.toml");
        let result = apply_path_replacements(path, &rules);
        assert_eq!(result, PathBuf::from("{{ name }}.toml"));
    }

    #[test]
    fn no_match_across_path_separator() {
        // "src/app" should not match "src/app" as a single literal — each
        // component is replaced independently.
        let rules = sorted(vec![rule("src/app", "{{ x }}")]);
        let path = Path::new("src/app/main.rs");
        let result = apply_path_replacements(path, &rules);
        // Should be unchanged because "/" is a component separator, not part
        // of any single component.
        assert_eq!(result, PathBuf::from("src/app/main.rs"));
    }
}
