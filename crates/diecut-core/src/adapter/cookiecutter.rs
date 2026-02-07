use std::collections::BTreeMap;
use std::path::Path;

use regex_lite::Regex;

use crate::adapter::{ResolvedTemplate, TemplateFormat};
use crate::config::schema::{
    AnswersConfig, FilesConfig, HooksConfig, TemplateConfig, TemplateMetadata,
};
use crate::config::variable::{VariableConfig, VariableType};
use crate::error::{DicecutError, Result};

/// Resolve a cookiecutter template into a `ResolvedTemplate`.
pub fn resolve(template_dir: &Path) -> Result<ResolvedTemplate> {
    let json_path = template_dir.join("cookiecutter.json");
    let content = std::fs::read_to_string(&json_path).map_err(|e| DicecutError::Io {
        context: format!("reading {}", json_path.display()),
        source: e,
    })?;

    let raw: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| DicecutError::ConfigParseCookiecutter { source: e })?;

    let obj = raw
        .as_object()
        .ok_or_else(|| DicecutError::ConfigParseCookiecutter {
            source: serde_json::from_str::<serde_json::Value>("null").unwrap_err(),
        })?;

    let mut variables = BTreeMap::new();
    let mut copy_without_render = Vec::new();
    let mut warnings = Vec::new();
    let mut prompts: BTreeMap<String, String> = BTreeMap::new();

    // First pass: extract special keys
    for (key, value) in obj {
        match key.as_str() {
            "_copy_without_render" => {
                if let Some(arr) = value.as_array() {
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            copy_without_render.push(s.to_string());
                        }
                    }
                }
            }
            "_extensions" => {
                warnings.push(
                    "Jinja2 extensions detected (_extensions) — not supported, ignoring"
                        .to_string(),
                );
            }
            "__prompts__" => {
                if let Some(prompts_obj) = value.as_object() {
                    for (pkey, pval) in prompts_obj {
                        if let Some(s) = pval.as_str() {
                            prompts.insert(pkey.clone(), s.to_string());
                        }
                    }
                }
            }
            _ if key.starts_with('_') && !key.starts_with("__") => {
                // Skip single-underscore private keys (but not double-underscore computed vars)
            }
            _ => {
                let var = json_value_to_variable(key, value, &mut warnings);
                variables.insert(key.clone(), var);
            }
        }
    }

    // Apply __prompts__ to matching variables
    for (name, prompt_text) in &prompts {
        if let Some(var) = variables.get_mut(name) {
            var.prompt = Some(prompt_text.clone());
        }
    }

    // Check for hooks directory
    if template_dir.join("hooks").exists() {
        warnings.push(
            "Python hooks directory detected — hooks are not supported and will be skipped"
                .to_string(),
        );
    }

    // The content_dir is the template root. Only directories matching {{cookiecutter.*}}
    // are actual template content. Everything else at root level gets excluded.
    let content_dir = template_dir.to_path_buf();

    // Build excludes: everything at root except {{cookiecutter.*}} directories
    let mut exclude = vec![
        "cookiecutter.json".to_string(),
        "hooks".to_string(),
        "hooks/**".to_string(),
        ".git".to_string(),
        ".git/**".to_string(),
    ];

    // Exclude all root-level entries that are NOT the {{cookiecutter.*}} template directory
    if let Ok(entries) = std::fs::read_dir(template_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("{{") {
                continue; // This is a template directory, don't exclude
            }
            if name == "cookiecutter.json" || name == "hooks" || name == ".git" {
                continue; // Already excluded above
            }
            exclude.push(name.clone());
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                exclude.push(format!("{name}/**"));
            }
        }
    }

    // Derive a template name from the directory name
    let name = template_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("cookiecutter-template")
        .to_string();

    let config = TemplateConfig {
        template: TemplateMetadata {
            name,
            version: None,
            description: None,
            min_diecut_version: None,
            templates_suffix: String::new(), // cookiecutter renders all files
        },
        variables,
        files: FilesConfig {
            exclude,
            copy_without_render,
            conditional: Vec::new(),
        },
        hooks: HooksConfig::default(),
        answers: AnswersConfig::default(),
    };

    Ok(ResolvedTemplate {
        config,
        content_dir,
        format: TemplateFormat::Cookiecutter,
        render_all: true,
        context_namespace: Some("cookiecutter".to_string()),
        warnings,
    })
}

/// Convert a cookiecutter.json value into a diecut VariableConfig.
fn json_value_to_variable(
    key: &str,
    value: &serde_json::Value,
    warnings: &mut Vec<String>,
) -> VariableConfig {
    match value {
        serde_json::Value::String(s) => {
            // Check if it's a computed expression (contains {{ cookiecutter.* }})
            if is_computed_expression(s) {
                let rewritten = rewrite_cookiecutter_refs(s);
                VariableConfig {
                    var_type: VariableType::String,
                    prompt: None,
                    default: None,
                    choices: None,
                    validation: None,
                    validation_message: None,
                    when: None,
                    computed: Some(rewritten),
                    secret: false,
                }
            } else {
                VariableConfig {
                    var_type: VariableType::String,
                    prompt: Some(key.to_string()),
                    default: Some(toml::Value::String(s.clone())),
                    choices: None,
                    validation: None,
                    validation_message: None,
                    when: None,
                    computed: None,
                    secret: false,
                }
            }
        }
        serde_json::Value::Bool(b) => VariableConfig {
            var_type: VariableType::Bool,
            prompt: Some(key.to_string()),
            default: Some(toml::Value::Boolean(*b)),
            choices: None,
            validation: None,
            validation_message: None,
            when: None,
            computed: None,
            secret: false,
        },
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                VariableConfig {
                    var_type: VariableType::Int,
                    prompt: Some(key.to_string()),
                    default: Some(toml::Value::Integer(i)),
                    choices: None,
                    validation: None,
                    validation_message: None,
                    when: None,
                    computed: None,
                    secret: false,
                }
            } else if let Some(f) = n.as_f64() {
                VariableConfig {
                    var_type: VariableType::Float,
                    prompt: Some(key.to_string()),
                    default: Some(toml::Value::Float(f)),
                    choices: None,
                    validation: None,
                    validation_message: None,
                    when: None,
                    computed: None,
                    secret: false,
                }
            } else {
                warnings.push(format!(
                    "Variable '{key}': unsupported number format, treating as string"
                ));
                VariableConfig {
                    var_type: VariableType::String,
                    prompt: Some(key.to_string()),
                    default: Some(toml::Value::String(n.to_string())),
                    choices: None,
                    validation: None,
                    validation_message: None,
                    when: None,
                    computed: None,
                    secret: false,
                }
            }
        }
        serde_json::Value::Array(arr) => {
            // Array of strings → select type, first item is default
            let choices: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            let default = choices.first().cloned();
            VariableConfig {
                var_type: VariableType::Select,
                prompt: Some(key.to_string()),
                default: default.map(toml::Value::String),
                choices: Some(choices),
                validation: None,
                validation_message: None,
                when: None,
                computed: None,
                secret: false,
            }
        }
        _ => {
            warnings.push(format!(
                "Variable '{key}': unsupported type, treating as string"
            ));
            VariableConfig {
                var_type: VariableType::String,
                prompt: Some(key.to_string()),
                default: Some(toml::Value::String(value.to_string())),
                choices: None,
                validation: None,
                validation_message: None,
                when: None,
                computed: None,
                secret: false,
            }
        }
    }
}

/// Check if a string value contains Jinja2/Tera template expressions referencing cookiecutter.
fn is_computed_expression(s: &str) -> bool {
    s.contains("{{") && s.contains("cookiecutter.")
}

/// Rewrite `cookiecutter.X` references to just `X` in a template expression.
/// Also translates Jinja2 filter/method syntax to Tera syntax.
fn rewrite_cookiecutter_refs(expr: &str) -> String {
    let ns_re = Regex::new(r"cookiecutter\.(\w+)").expect("valid regex");
    let result = ns_re.replace_all(expr, "$1").to_string();
    let result = rewrite_jinja2_method_calls(&result);
    rewrite_jinja2_filters(&result)
}

/// Rewrite Jinja2 string method calls to Tera filter syntax.
/// Jinja2: `var.replace('x', 'y')` → Tera: `var | replace(from="x", to="y")`
fn rewrite_jinja2_method_calls(expr: &str) -> String {
    // Match var.replace('arg1', 'arg2') or var.replace("arg1", "arg2")
    let sq_re =
        Regex::new(r"(\w+)\.replace\(\s*'([^']*)'\s*,\s*'([^']*)'\s*\)").expect("valid regex");
    let result = sq_re
        .replace_all(expr, r#"$1 | replace(from="$2", to="$3")"#)
        .to_string();

    let dq_re =
        Regex::new(r#"(\w+)\.replace\(\s*"([^"]*)"\s*,\s*"([^"]*)"\s*\)"#).expect("valid regex");
    dq_re
        .replace_all(&result, r#"$1 | replace(from="$2", to="$3")"#)
        .to_string()
}

/// Rewrite Jinja2-style filter calls to Tera syntax.
/// Jinja2: `| replace('x', 'y')` → Tera: `| replace(from="x", to="y")`
fn rewrite_jinja2_filters(expr: &str) -> String {
    // Match replace('arg1', 'arg2') with single quotes (as filter, no dot-prefix)
    let sq_re = Regex::new(r"replace\(\s*'([^']*)'\s*,\s*'([^']*)'\s*\)").expect("valid regex");
    let result = sq_re
        .replace_all(expr, r#"replace(from="$1", to="$2")"#)
        .to_string();

    // Match replace("arg1", "arg2") with double quotes
    let dq_re = Regex::new(r#"replace\(\s*"([^"]*)"\s*,\s*"([^"]*)"\s*\)"#).expect("valid regex");
    dq_re
        .replace_all(&result, r#"replace(from="$1", to="$2")"#)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_cookiecutter_refs() {
        assert_eq!(
            rewrite_cookiecutter_refs("{{ cookiecutter.project_name }}"),
            "{{ project_name }}"
        );
        assert_eq!(
            rewrite_cookiecutter_refs("{{ cookiecutter.project_name | slugify }}"),
            "{{ project_name | slugify }}"
        );
        assert_eq!(
            rewrite_cookiecutter_refs(
                "{{ cookiecutter.project_name | lower | replace(' ', '-') }}"
            ),
            "{{ project_name | lower | replace(from=\" \", to=\"-\") }}"
        );
        // Jinja2 string method call: .replace('x', 'y')
        assert_eq!(
            rewrite_cookiecutter_refs("{{ cookiecutter.pypi_package_name.replace('-', '_') }}"),
            r#"{{ pypi_package_name | replace(from="-", to="_") }}"#
        );
        // Multiple variable references
        assert_eq!(
            rewrite_cookiecutter_refs(
                "{{ cookiecutter.github_username }}/{{ cookiecutter.project_slug }}"
            ),
            "{{ github_username }}/{{ project_slug }}"
        );
    }

    #[test]
    fn test_rewrite_jinja2_method_calls() {
        assert_eq!(
            rewrite_jinja2_method_calls("name.replace('-', '_')"),
            r#"name | replace(from="-", to="_")"#
        );
        assert_eq!(
            rewrite_jinja2_method_calls(r#"name.replace("-", "_")"#),
            r#"name | replace(from="-", to="_")"#
        );
        // No method call → no change
        assert_eq!(rewrite_jinja2_method_calls("name | lower"), "name | lower");
    }

    #[test]
    fn test_rewrite_jinja2_filters() {
        assert_eq!(
            rewrite_jinja2_filters("replace(' ', '-')"),
            r#"replace(from=" ", to="-")"#
        );
        assert_eq!(
            rewrite_jinja2_filters(r#"replace("foo", "bar")"#),
            r#"replace(from="foo", to="bar")"#
        );
        // No-op for already-Tera syntax
        assert_eq!(
            rewrite_jinja2_filters(r#"replace(from=" ", to="-")"#),
            r#"replace(from=" ", to="-")"#
        );
    }

    #[test]
    fn test_is_computed_expression() {
        assert!(is_computed_expression("{{ cookiecutter.name | slugify }}"));
        assert!(!is_computed_expression("just a plain string"));
        assert!(!is_computed_expression("{{ some_other_thing }}"));
    }
}
