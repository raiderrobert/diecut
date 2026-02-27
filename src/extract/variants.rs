use regex_lite::Regex;

/// A case variant of a variable value, with its literal text and Tera expression.
#[derive(Debug, Clone, PartialEq)]
pub struct CaseVariant {
    pub name: &'static str,
    pub literal: String,
    pub tera_expr: String,
}

/// Split a string value into words for case variant generation.
///
/// Handles kebab-case, snake_case, camelCase, PascalCase, dot.case, and space-separated.
pub fn split_into_words(value: &str) -> Vec<String> {
    if value.contains('-') {
        return value.split('-').map(|s| s.to_lowercase()).collect();
    }
    if value.contains('_') {
        return value.split('_').map(|s| s.to_lowercase()).collect();
    }
    if value.contains('.') {
        return value.split('.').map(|s| s.to_lowercase()).collect();
    }
    if value.contains(' ') {
        return value.split_whitespace().map(|s| s.to_lowercase()).collect();
    }

    // camelCase / PascalCase splitting
    let re = Regex::new(r"[A-Z][a-z]*|[a-z]+|[0-9]+").unwrap();
    let words: Vec<String> = re
        .find_iter(value)
        .map(|m| m.as_str().to_lowercase())
        .collect();

    if words.is_empty() {
        vec![value.to_lowercase()]
    } else {
        words
    }
}

/// Detect if a value is "multi-word" in a way that supports case variants.
///
/// Single words and space-separated phrases skip variant detection.
fn supports_case_variants(value: &str) -> bool {
    let words = split_into_words(value);
    if words.len() < 2 {
        return false;
    }
    // Space-separated values (like author names) skip variant detection
    if value.contains(' ') {
        return false;
    }
    true
}

fn to_kebab(words: &[String]) -> String {
    words.join("-")
}

fn to_snake(words: &[String]) -> String {
    words.join("_")
}

fn to_screaming_snake(words: &[String]) -> String {
    words
        .iter()
        .map(|w| w.to_uppercase())
        .collect::<Vec<_>>()
        .join("_")
}

fn to_screaming_kebab(words: &[String]) -> String {
    words
        .iter()
        .map(|w| w.to_uppercase())
        .collect::<Vec<_>>()
        .join("-")
}

fn to_pascal(words: &[String]) -> String {
    words
        .iter()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    upper + chars.as_str()
                }
                None => String::new(),
            }
        })
        .collect()
}

fn to_camel(words: &[String]) -> String {
    let pascal = to_pascal(words);
    let mut chars = pascal.chars();
    match chars.next() {
        Some(c) => {
            let lower: String = c.to_lowercase().collect();
            lower + chars.as_str()
        }
        None => String::new(),
    }
}

fn to_dot(words: &[String]) -> String {
    words.join(".")
}

/// Detect the canonical separator in the original value.
pub fn detect_separator(value: &str) -> &'static str {
    if value.contains('-') {
        "-"
    } else if value.contains('_') {
        "_"
    } else if value.contains('.') {
        "."
    } else {
        // PascalCase/camelCase — treat as kebab canonical
        "-"
    }
}

/// Check whether a variant is the canonical one (matches the input separator).
///
/// Canonical variants use the bare `{{ var_name }}` expression and do not get
/// a computed variable in diecut.toml.
pub fn is_canonical_variant(variant_name: &str, canonical_sep: &str) -> bool {
    matches!(
        (variant_name, canonical_sep),
        ("kebab", "-") | ("snake", "_") | ("dot", ".")
    )
}

/// Build a Tera expression for a variant, given the variable name and canonical separator.
///
/// Canonical variants use `{{ var_name }}` directly. Non-canonical variants reference
/// their computed variable (e.g., `{{ var_name_snake }}`), which is defined in diecut.toml.
fn tera_expr_for_variant(var_name: &str, variant_name: &str, canonical_sep: &str) -> String {
    if variant_name == "verbatim" || is_canonical_variant(variant_name, canonical_sep) {
        return format!("{{{{ {var_name} }}}}");
    }
    // Non-canonical variants reference their computed variable name
    format!("{{{{ {var_name}_{variant_name} }}}}")
}

/// Generate all case variants for a given variable value.
///
/// Returns the canonical variant first, followed by alternatives.
/// Only returns variants whose literal differs from the canonical form.
/// Single-word values and space-separated phrases return only a verbatim replacement.
pub fn generate_variants(var_name: &str, value: &str) -> Vec<CaseVariant> {
    if !supports_case_variants(value) {
        return vec![CaseVariant {
            name: "verbatim",
            literal: value.to_string(),
            tera_expr: format!("{{{{ {var_name} }}}}"),
        }];
    }

    let words = split_into_words(value);
    let canonical_sep = detect_separator(value);

    let candidates: Vec<(&str, String)> = vec![
        ("kebab", to_kebab(&words)),
        ("snake", to_snake(&words)),
        ("screaming_snake", to_screaming_snake(&words)),
        ("screaming_kebab", to_screaming_kebab(&words)),
        ("pascal", to_pascal(&words)),
        ("camel", to_camel(&words)),
        ("dot", to_dot(&words)),
    ];

    // Deduplicate: some variants produce the same literal (e.g., single-word)
    let mut seen = std::collections::HashSet::new();
    let mut variants = Vec::new();

    for (name, literal) in candidates {
        if seen.insert(literal.clone()) {
            let tera_expr = tera_expr_for_variant(var_name, name, canonical_sep);
            variants.push(CaseVariant {
                name,
                literal,
                tera_expr,
            });
        }
    }

    variants
}

/// Build a computed Tera expression for a named variant variable.
///
/// This is used in diecut.toml for computed variables like `project_name_snake`.
pub fn computed_expression(var_name: &str, variant_name: &str, canonical_sep: &str) -> String {
    match (variant_name, canonical_sep) {
        ("snake", sep) if sep != "_" => {
            format!("{var_name} | replace(from=\"{sep}\", to=\"_\")")
        }
        ("screaming_snake", sep) => {
            if sep == "_" {
                format!("{var_name} | upper")
            } else {
                format!("{var_name} | replace(from=\"{sep}\", to=\"_\") | upper")
            }
        }
        ("screaming_kebab", sep) => {
            if sep == "-" {
                format!("{var_name} | upper")
            } else {
                format!("{var_name} | replace(from=\"{sep}\", to=\"-\") | upper")
            }
        }
        ("pascal", sep) => {
            format!("{var_name} | replace(from=\"{sep}\", to=\" \") | title | replace(from=\" \", to=\"\")")
        }
        ("camel", sep) => {
            format!("{var_name} | camelcase(sep=\"{sep}\")")
        }
        ("kebab", sep) if sep != "-" => {
            format!("{var_name} | replace(from=\"{sep}\", to=\"-\")")
        }
        ("dot", sep) if sep != "." => {
            format!("{var_name} | replace(from=\"{sep}\", to=\".\")")
        }
        _ => var_name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("my-app", vec!["my", "app"])]
    #[case("my_app", vec!["my", "app"])]
    #[case("MyApp", vec!["my", "app"])]
    #[case("myApp", vec!["my", "app"])]
    #[case("my.app", vec!["my", "app"])]
    #[case("my app", vec!["my", "app"])]
    #[case("single", vec!["single"])]
    fn test_split_into_words(#[case] input: &str, #[case] expected: Vec<&str>) {
        assert_eq!(split_into_words(input), expected);
    }

    #[test]
    fn test_generate_variants_kebab() {
        let variants = generate_variants("project_name", "my-app");
        let names: Vec<&str> = variants.iter().map(|v| v.name).collect();
        assert!(names.contains(&"kebab"));
        assert!(names.contains(&"snake"));
        assert!(names.contains(&"pascal"));

        let kebab = variants.iter().find(|v| v.name == "kebab").unwrap();
        assert_eq!(kebab.literal, "my-app");

        let snake = variants.iter().find(|v| v.name == "snake").unwrap();
        assert_eq!(snake.literal, "my_app");

        let pascal = variants.iter().find(|v| v.name == "pascal").unwrap();
        assert_eq!(pascal.literal, "MyApp");
    }

    #[test]
    fn test_generate_variants_single_word() {
        let variants = generate_variants("name", "hello");
        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0].name, "verbatim");
        assert_eq!(variants[0].literal, "hello");
    }

    #[test]
    fn test_generate_variants_space_separated() {
        let variants = generate_variants("author", "Jane Doe");
        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0].name, "verbatim");
        assert_eq!(variants[0].literal, "Jane Doe");
    }

    #[test]
    fn test_generate_variants_screaming_snake() {
        let variants = generate_variants("project_name", "my-app");
        let ss = variants
            .iter()
            .find(|v| v.name == "screaming_snake")
            .unwrap();
        assert_eq!(ss.literal, "MY_APP");
    }

    #[test]
    fn test_tera_expr_kebab_canonical() {
        let expr = tera_expr_for_variant("project_name", "kebab", "-");
        assert_eq!(expr, "{{ project_name }}");
    }

    #[test]
    fn test_tera_expr_snake_from_kebab() {
        let expr = tera_expr_for_variant("project_name", "snake", "-");
        assert_eq!(expr, "{{ project_name_snake }}");
    }
}
