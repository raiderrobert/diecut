use std::collections::{BTreeMap, HashMap};

use tera::Value;

use crate::config::schema::TemplateConfig;
use crate::config::variable::{VariableConfig, VariableType};
use crate::error::{DicecutError, Result};
use crate::render::build_context;

#[derive(Default)]
pub struct PromptOptions {
    pub data_overrides: HashMap<String, String>,
    pub use_defaults: bool,
}

pub fn collect_variables(
    config: &TemplateConfig,
    options: &PromptOptions,
) -> Result<BTreeMap<String, Value>> {
    let mut values: BTreeMap<String, Value> = BTreeMap::new();

    for (name, var) in &config.variables {
        if var.computed.is_some() {
            continue; // computed vars are handled in second pass
        }

        if let Some(when_expr) = &var.when {
            if !evaluate_when(name, when_expr, &values)? {
                continue; // condition is false, skip
            }
        }

        if let Some(override_val) = options.data_overrides.get(name) {
            let value = parse_override(override_val, var);
            values.insert(name.clone(), value);
            continue;
        }

        if options.use_defaults {
            if let Some(default) = &var.default {
                values.insert(name.clone(), toml_to_tera_value(default));
                continue;
            }
        }

        let value = prompt_variable(name, var)?;
        values.insert(name.clone(), value);
    }

    // Evaluate computed variables iteratively (they may depend on each other)
    let computed_vars: Vec<_> = config
        .variables
        .iter()
        .filter(|(_, v)| v.computed.is_some())
        .map(|(name, var)| (name.clone(), var.computed.clone().unwrap()))
        .collect();

    let mut remaining: Vec<(String, String)> = computed_vars;
    let max_iterations = remaining.len() + 1;
    for _ in 0..max_iterations {
        if remaining.is_empty() {
            break;
        }
        let mut still_pending = Vec::new();
        for (name, computed_expr) in &remaining {
            match evaluate_computed(name, computed_expr, &values) {
                Ok(value) => {
                    values.insert(name.clone(), value);
                }
                Err(_) => {
                    still_pending.push((name.clone(), computed_expr.clone()));
                }
            }
        }
        if still_pending.len() == remaining.len() {
            // No progress — return the first error for diagnostics
            let (name, expr) = &still_pending[0];
            evaluate_computed(name, expr, &values)?;
        }
        remaining = still_pending;
    }

    Ok(values)
}

fn evaluate_when(name: &str, when_expr: &str, values: &BTreeMap<String, Value>) -> Result<bool> {
    let context = build_context(values);
    crate::render::eval_bool_expr(when_expr, &context).map_err(|e| DicecutError::WhenEvaluation {
        name: name.to_string(),
        source: e,
    })
}

fn evaluate_computed(
    name: &str,
    computed_expr: &str,
    values: &BTreeMap<String, Value>,
) -> Result<Value> {
    let mut tera = tera::Tera::default();
    tera.add_raw_template("__computed__", computed_expr)
        .map_err(|e| DicecutError::ComputedEvaluation {
            name: name.to_string(),
            source: e,
        })?;

    let context = build_context(values);

    let result =
        tera.render("__computed__", &context)
            .map_err(|e| DicecutError::ComputedEvaluation {
                name: name.to_string(),
                source: e,
            })?;

    Ok(Value::String(result))
}

fn prompt_variable(name: &str, var: &VariableConfig) -> Result<Value> {
    let prompt_text = var.prompt.as_deref().unwrap_or(name);

    match var.var_type {
        VariableType::String => {
            let mut prompt = inquire::Text::new(prompt_text);
            if let Some(toml::Value::String(default)) = &var.default {
                prompt = prompt.with_default(default);
            }
            if let Some(pattern) = &var.validation {
                let pattern = pattern.clone();
                let msg = var
                    .validation_message
                    .clone()
                    .unwrap_or_else(|| format!("Must match pattern: {pattern}"));
                prompt = prompt.with_validator(move |input: &str| {
                    let re = regex_lite::Regex::new(&pattern)
                        .map_err(|e| inquire::CustomUserError::from(e.to_string()))?;
                    if re.is_match(input) {
                        Ok(inquire::validator::Validation::Valid)
                    } else {
                        Ok(inquire::validator::Validation::Invalid(
                            inquire::validator::ErrorMessage::Custom(msg.clone()),
                        ))
                    }
                });
            }
            let answer = prompt.prompt().map_err(|_| DicecutError::PromptCancelled)?;
            Ok(Value::String(answer))
        }
        VariableType::Bool => {
            let default = match &var.default {
                Some(toml::Value::Boolean(b)) => *b,
                _ => false,
            };
            let answer = inquire::Confirm::new(prompt_text)
                .with_default(default)
                .prompt()
                .map_err(|_| DicecutError::PromptCancelled)?;
            Ok(Value::Bool(answer))
        }
        VariableType::Int => {
            let mut prompt = inquire::Text::new(prompt_text);
            let default_str;
            if let Some(toml::Value::Integer(n)) = &var.default {
                default_str = n.to_string();
                prompt = prompt.with_default(&default_str);
            }
            prompt = prompt.with_validator(|input: &str| {
                if input.parse::<i64>().is_ok() {
                    Ok(inquire::validator::Validation::Valid)
                } else {
                    Ok(inquire::validator::Validation::Invalid(
                        inquire::validator::ErrorMessage::Custom(
                            "Must be a valid integer".to_string(),
                        ),
                    ))
                }
            });
            let answer = prompt.prompt().map_err(|_| DicecutError::PromptCancelled)?;
            let n: i64 = answer.parse().map_err(|_| DicecutError::ValidationFailed {
                name: name.to_string(),
                message: "expected a valid integer".into(),
            })?;
            Ok(Value::Number(serde_json::Number::from(n)))
        }
        VariableType::Float => {
            let mut prompt = inquire::Text::new(prompt_text);
            let default_str;
            if let Some(toml::Value::Float(f)) = &var.default {
                default_str = f.to_string();
                prompt = prompt.with_default(&default_str);
            }
            prompt = prompt.with_validator(|input: &str| match input.parse::<f64>() {
                Ok(f) if f.is_finite() => Ok(inquire::validator::Validation::Valid),
                _ => Ok(inquire::validator::Validation::Invalid(
                    inquire::validator::ErrorMessage::Custom("Must be a finite number".to_string()),
                )),
            });
            let answer = prompt.prompt().map_err(|_| DicecutError::PromptCancelled)?;
            let f: f64 = answer.parse().map_err(|_| DicecutError::ValidationFailed {
                name: name.to_string(),
                message: "expected a valid float".into(),
            })?;
            Ok(
                serde_json::to_value(f).map_err(|e| DicecutError::ValidationFailed {
                    name: name.to_string(),
                    message: e.to_string(),
                })?,
            )
        }
        VariableType::Select => {
            let choices = var.choices.as_ref().expect("select must have choices");
            let mut prompt = inquire::Select::new(prompt_text, choices.clone());
            if let Some(toml::Value::String(default)) = &var.default {
                if let Some(idx) = choices.iter().position(|c| c == default) {
                    prompt = prompt.with_starting_cursor(idx);
                }
            }
            let answer = prompt.prompt().map_err(|_| DicecutError::PromptCancelled)?;
            Ok(Value::String(answer))
        }
        VariableType::Multiselect => {
            let choices = var.choices.as_ref().expect("multiselect must have choices");
            let mut prompt = inquire::MultiSelect::new(prompt_text, choices.clone());
            let default_indices: Vec<usize> =
                if let Some(toml::Value::Array(defaults)) = &var.default {
                    defaults
                        .iter()
                        .filter_map(|v| {
                            if let toml::Value::String(s) = v {
                                choices.iter().position(|c| c == s)
                            } else {
                                None
                            }
                        })
                        .collect()
                } else {
                    Vec::new()
                };
            if !default_indices.is_empty() {
                prompt = prompt.with_default(&default_indices);
            }
            let answers = prompt.prompt().map_err(|_| DicecutError::PromptCancelled)?;
            let arr: Vec<Value> = answers.into_iter().map(Value::String).collect();
            Ok(Value::Array(arr))
        }
    }
}

fn parse_override(value: &str, var: &VariableConfig) -> Value {
    match var.var_type {
        VariableType::Bool => Value::Bool(value == "true" || value == "1" || value == "yes"),
        VariableType::Int => value
            .parse::<i64>()
            .map(|n| Value::Number(serde_json::Number::from(n)))
            .unwrap_or(Value::String(value.to_string())),
        VariableType::Float => value
            .parse::<f64>()
            .ok()
            .and_then(|f| serde_json::to_value(f).ok())
            .unwrap_or(Value::String(value.to_string())),
        VariableType::Multiselect => {
            let items: Vec<Value> = value
                .split(',')
                .map(|s| Value::String(s.trim().to_string()))
                .collect();
            Value::Array(items)
        }
        _ => Value::String(value.to_string()),
    }
}

fn toml_to_tera_value(val: &toml::Value) -> Value {
    match val {
        toml::Value::String(s) => Value::String(s.clone()),
        toml::Value::Integer(n) => Value::Number(serde_json::Number::from(*n)),
        toml::Value::Float(f) => serde_json::to_value(f).unwrap_or(Value::Null),
        toml::Value::Boolean(b) => Value::Bool(*b),
        toml::Value::Array(arr) => Value::Array(arr.iter().map(toml_to_tera_value).collect()),
        toml::Value::Table(t) => {
            let map: serde_json::Map<String, Value> = t
                .iter()
                .map(|(k, v)| (k.clone(), toml_to_tera_value(v)))
                .collect();
            Value::Object(map)
        }
        toml::Value::Datetime(d) => Value::String(d.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;

    use super::*;
    use rstest::rstest;

    /// Helper to create a minimal TemplateConfig for testing
    fn minimal_config(variables: IndexMap<String, VariableConfig>) -> TemplateConfig {
        TemplateConfig {
            template: crate::config::schema::TemplateMetadata {
                name: "test".to_string(),
                version: None,
                description: None,
                min_diecut_version: None,
                templates_suffix: ".tera".to_string(),
            },
            variables,
            files: Default::default(),
            hooks: Default::default(),
            answers: Default::default(),
        }
    }

    /// Integration test: verify basic variable collection with defaults and multiple types
    #[test]
    fn test_collect_variables_multiple() {
        let mut variables = IndexMap::new();
        variables.insert(
            "name".to_string(),
            VariableConfig {
                var_type: VariableType::String,
                prompt: None,
                default: Some(toml::Value::String("test".to_string())),
                choices: None,
                validation: None,
                validation_message: None,
                when: None,
                computed: None,
                secret: false,
            },
        );
        variables.insert(
            "license".to_string(),
            VariableConfig {
                var_type: VariableType::Select,
                prompt: None,
                default: Some(toml::Value::String("MIT".to_string())),
                choices: Some(vec!["MIT".to_string(), "Apache-2.0".to_string()]),
                validation: None,
                validation_message: None,
                when: None,
                computed: None,
                secret: false,
            },
        );
        variables.insert(
            "use_ci".to_string(),
            VariableConfig {
                var_type: VariableType::Bool,
                prompt: None,
                default: Some(toml::Value::Boolean(false)),
                choices: None,
                validation: None,
                validation_message: None,
                when: None,
                computed: None,
                secret: false,
            },
        );

        let config = minimal_config(variables);
        let options = PromptOptions {
            data_overrides: HashMap::new(),
            use_defaults: true,
        };

        let result = collect_variables(&config, &options).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result.get("name").unwrap(), "test");
        assert_eq!(result.get("license").unwrap(), "MIT");
        assert_eq!(result.get("use_ci").unwrap(), &Value::Bool(false));
    }

    /// Integration test: verify override type coercion for boolean values
    #[rstest]
    #[case("true", true)]
    #[case("false", false)]
    #[case("1", true)]
    #[case("0", false)]
    fn test_boolean_override_coercion(#[case] input: &str, #[case] expected: bool) {
        let mut variables = IndexMap::new();
        variables.insert(
            "enabled".to_string(),
            VariableConfig {
                var_type: VariableType::Bool,
                prompt: None,
                default: Some(toml::Value::Boolean(false)),
                choices: None,
                validation: None,
                validation_message: None,
                when: None,
                computed: None,
                secret: false,
            },
        );

        let config = minimal_config(variables);
        let mut overrides = HashMap::new();
        overrides.insert("enabled".to_string(), input.to_string());

        let options = PromptOptions {
            data_overrides: overrides,
            use_defaults: false,
        };

        let result = collect_variables(&config, &options).unwrap();

        assert_eq!(result.get("enabled").unwrap(), &Value::Bool(expected));
    }

    /// Integration test: verify when conditions evaluate correctly
    #[rstest]
    #[case(true, 2, Some("advanced"))] // condition true, both vars collected
    #[case(false, 1, None)] // condition false, dependent var skipped
    fn test_when_condition(
        #[case] condition: bool,
        #[case] expected_len: usize,
        #[case] expected_value: Option<&str>,
    ) {
        let mut variables = IndexMap::new();
        variables.insert(
            "enable_feature".to_string(),
            VariableConfig {
                var_type: VariableType::Bool,
                prompt: None,
                default: Some(toml::Value::Boolean(condition)),
                choices: None,
                validation: None,
                validation_message: None,
                when: None,
                computed: None,
                secret: false,
            },
        );
        variables.insert(
            "feature_config".to_string(),
            VariableConfig {
                var_type: VariableType::String,
                prompt: None,
                default: Some(toml::Value::String("advanced".to_string())),
                choices: None,
                validation: None,
                validation_message: None,
                when: Some("enable_feature".to_string()),
                computed: None,
                secret: false,
            },
        );

        let config = minimal_config(variables);
        let options = PromptOptions {
            data_overrides: HashMap::new(),
            use_defaults: true,
        };

        let result = collect_variables(&config, &options).unwrap();

        assert_eq!(result.len(), expected_len);
        assert_eq!(
            result.get("enable_feature").unwrap(),
            &Value::Bool(condition)
        );
        assert_eq!(
            result.get("feature_config").map(|v| v.as_str().unwrap()),
            expected_value
        );
    }

    /// Integration test: verify computed variables can reference other variables
    #[test]
    fn test_computed_variable_depends_on_another() {
        let mut variables = IndexMap::new();
        variables.insert(
            "author".to_string(),
            VariableConfig {
                var_type: VariableType::String,
                prompt: None,
                default: Some(toml::Value::String("John Doe".to_string())),
                choices: None,
                validation: None,
                validation_message: None,
                when: None,
                computed: None,
                secret: false,
            },
        );
        variables.insert(
            "author_email".to_string(),
            VariableConfig {
                var_type: VariableType::String,
                prompt: None,
                default: Some(toml::Value::String("john@example.com".to_string())),
                choices: None,
                validation: None,
                validation_message: None,
                when: None,
                computed: None,
                secret: false,
            },
        );
        variables.insert(
            "full_author".to_string(),
            VariableConfig {
                var_type: VariableType::String,
                prompt: None,
                default: None,
                choices: None,
                validation: None,
                validation_message: None,
                when: None,
                computed: Some("{{ author }} <{{ author_email }}>".to_string()),
                secret: false,
            },
        );

        let config = minimal_config(variables);
        let options = PromptOptions {
            data_overrides: HashMap::new(),
            use_defaults: true,
        };

        let result = collect_variables(&config, &options).unwrap();

        assert_eq!(
            result.get("full_author").unwrap(),
            "John Doe <john@example.com>"
        );
    }

    #[test]
    fn test_float_validator_rejects_nonfinite() {
        // Verify the validator logic: only finite floats are valid
        let is_valid =
            |input: &str| -> bool { matches!(input.parse::<f64>(), Ok(f) if f.is_finite()) };
        assert!(!is_valid("inf"));
        assert!(!is_valid("-inf"));
        assert!(!is_valid("nan"));
        assert!(!is_valid("not-a-number"));
        assert!(is_valid("3.14"));
        assert!(is_valid("-1.0"));
        assert!(is_valid("0"));
    }

    #[test]
    fn test_serde_json_silently_corrupts_infinity() {
        // serde_json::to_value(f64::INFINITY) does not panic or error;
        // it silently produces Value::Null, which is data corruption.
        // This is why the validator must reject non-finite floats before they reach to_value.
        let result = serde_json::to_value(f64::INFINITY).unwrap();
        assert_eq!(result, serde_json::Value::Null);
    }

    /// Integration test: verify Tera errors in computed variables are caught and wrapped
    #[test]
    fn test_computed_variable_evaluation_error() {
        let mut variables = IndexMap::new();
        variables.insert(
            "broken".to_string(),
            VariableConfig {
                var_type: VariableType::String,
                prompt: None,
                default: None,
                choices: None,
                validation: None,
                validation_message: None,
                when: None,
                computed: Some("{{ undefined_var }}".to_string()),
                secret: false,
            },
        );

        let config = minimal_config(variables);
        let options = PromptOptions {
            data_overrides: HashMap::new(),
            use_defaults: true,
        };

        // Should error because undefined_var doesn't exist
        let result = collect_variables(&config, &options);
        assert!(result.is_err());
    }
}
