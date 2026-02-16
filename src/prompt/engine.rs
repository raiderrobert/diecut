use std::collections::{BTreeMap, HashMap};

use tera::{Context, Tera, Value};

use crate::config::schema::TemplateConfig;
use crate::config::variable::{VariableConfig, VariableType};
use crate::error::{DicecutError, Result};

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
    let mut tera = Tera::default();
    let template_str = format!("{{% if {when_expr} %}}true{{% else %}}false{{% endif %}}");
    tera.add_raw_template("__when__", &template_str)
        .map_err(|e| DicecutError::WhenEvaluation {
            name: name.to_string(),
            source: e,
        })?;

    let mut context = Context::new();
    for (k, v) in values {
        context.insert(k, v);
    }

    let result = tera
        .render("__when__", &context)
        .map_err(|e| DicecutError::WhenEvaluation {
            name: name.to_string(),
            source: e,
        })?;

    Ok(result.trim() == "true")
}

fn evaluate_computed(
    name: &str,
    computed_expr: &str,
    values: &BTreeMap<String, Value>,
) -> Result<Value> {
    let mut tera = Tera::default();
    tera.add_raw_template("__computed__", computed_expr)
        .map_err(|e| DicecutError::ComputedEvaluation {
            name: name.to_string(),
            source: e,
        })?;

    let mut context = Context::new();
    for (k, v) in values {
        context.insert(k, v);
    }

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
            let n: i64 = answer.parse().unwrap();
            Ok(Value::Number(serde_json::Number::from(n)))
        }
        VariableType::Float => {
            let mut prompt = inquire::Text::new(prompt_text);
            let default_str;
            if let Some(toml::Value::Float(f)) = &var.default {
                default_str = f.to_string();
                prompt = prompt.with_default(&default_str);
            }
            prompt = prompt.with_validator(|input: &str| {
                if input.parse::<f64>().is_ok() {
                    Ok(inquire::validator::Validation::Valid)
                } else {
                    Ok(inquire::validator::Validation::Invalid(
                        inquire::validator::ErrorMessage::Custom(
                            "Must be a valid number".to_string(),
                        ),
                    ))
                }
            });
            let answer = prompt.prompt().map_err(|_| DicecutError::PromptCancelled)?;
            let f: f64 = answer.parse().unwrap();
            Ok(serde_json::to_value(f).unwrap())
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
    use super::*;
    use rstest::rstest;

    /// Helper to create a minimal TemplateConfig for testing
    fn minimal_config(variables: BTreeMap<String, VariableConfig>) -> TemplateConfig {
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

    #[test]
    fn test_collect_variables_text_with_default() {
        let mut variables = BTreeMap::new();
        variables.insert(
            "project_name".to_string(),
            VariableConfig {
                var_type: VariableType::String,
                prompt: None,
                default: Some(toml::Value::String("my-project".to_string())),
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

        assert_eq!(result.get("project_name").unwrap(), "my-project");
    }

    #[test]
    fn test_collect_variables_data_override() {
        let mut variables = BTreeMap::new();
        variables.insert(
            "project_name".to_string(),
            VariableConfig {
                var_type: VariableType::String,
                prompt: None,
                default: Some(toml::Value::String("default-name".to_string())),
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
        overrides.insert("project_name".to_string(), "overridden-name".to_string());

        let options = PromptOptions {
            data_overrides: overrides,
            use_defaults: false,
        };

        let result = collect_variables(&config, &options).unwrap();

        assert_eq!(result.get("project_name").unwrap(), "overridden-name");
    }

    #[test]
    fn test_collect_variables_select_with_default() {
        let mut variables = BTreeMap::new();
        variables.insert(
            "license".to_string(),
            VariableConfig {
                var_type: VariableType::Select,
                prompt: None,
                default: Some(toml::Value::String("MIT".to_string())),
                choices: Some(vec![
                    "MIT".to_string(),
                    "Apache-2.0".to_string(),
                    "GPL-3.0".to_string(),
                ]),
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

        assert_eq!(result.get("license").unwrap(), "MIT");
    }

    #[test]
    fn test_collect_variables_boolean() {
        let mut variables = BTreeMap::new();
        variables.insert(
            "use_docker".to_string(),
            VariableConfig {
                var_type: VariableType::Bool,
                prompt: None,
                default: Some(toml::Value::Boolean(true)),
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

        assert_eq!(result.get("use_docker").unwrap(), &Value::Bool(true));
    }

    #[test]
    fn test_collect_variables_computed_slug() {
        let mut variables = BTreeMap::new();
        variables.insert(
            "project_name".to_string(),
            VariableConfig {
                var_type: VariableType::String,
                prompt: None,
                default: Some(toml::Value::String("My Cool Project".to_string())),
                choices: None,
                validation: None,
                validation_message: None,
                when: None,
                computed: None,
                secret: false,
            },
        );
        variables.insert(
            "project_slug".to_string(),
            VariableConfig {
                var_type: VariableType::String,
                prompt: None,
                default: None,
                choices: None,
                validation: None,
                validation_message: None,
                when: None,
                computed: Some("{{ project_name | slugify }}".to_string()),
                secret: false,
            },
        );

        let config = minimal_config(variables);
        let options = PromptOptions {
            data_overrides: HashMap::new(),
            use_defaults: true,
        };

        let result = collect_variables(&config, &options).unwrap();

        assert_eq!(result.get("project_slug").unwrap(), "my-cool-project");
    }

    #[test]
    fn test_collect_variables_multiple() {
        let mut variables = BTreeMap::new();
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

    #[rstest]
    #[case("true", true)]
    #[case("false", false)]
    #[case("1", true)]
    #[case("0", false)]
    fn test_boolean_override_coercion(#[case] input: &str, #[case] expected: bool) {
        let mut variables = BTreeMap::new();
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

    #[test]
    fn test_integer_override() {
        let mut variables = BTreeMap::new();
        variables.insert(
            "port".to_string(),
            VariableConfig {
                var_type: VariableType::Int,
                prompt: None,
                default: Some(toml::Value::Integer(8080)),
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
        overrides.insert("port".to_string(), "3000".to_string());

        let options = PromptOptions {
            data_overrides: overrides,
            use_defaults: false,
        };

        let result = collect_variables(&config, &options).unwrap();

        assert_eq!(
            result.get("port").unwrap(),
            &Value::Number(serde_json::Number::from(3000))
        );
    }

    #[test]
    fn test_float_override() {
        let mut variables = BTreeMap::new();
        variables.insert(
            "threshold".to_string(),
            VariableConfig {
                var_type: VariableType::Float,
                prompt: None,
                default: Some(toml::Value::Float(0.5)),
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
        overrides.insert("threshold".to_string(), "0.75".to_string());

        let options = PromptOptions {
            data_overrides: overrides,
            use_defaults: false,
        };

        let result = collect_variables(&config, &options).unwrap();

        let expected = serde_json::to_value(0.75).unwrap();
        assert_eq!(result.get("threshold").unwrap(), &expected);
    }

    #[test]
    fn test_multiselect_override() {
        let mut variables = BTreeMap::new();
        variables.insert(
            "features".to_string(),
            VariableConfig {
                var_type: VariableType::Multiselect,
                prompt: None,
                default: None,
                choices: Some(vec![
                    "auth".to_string(),
                    "api".to_string(),
                    "db".to_string(),
                ]),
                validation: None,
                validation_message: None,
                when: None,
                computed: None,
                secret: false,
            },
        );

        let config = minimal_config(variables);
        let mut overrides = HashMap::new();
        overrides.insert("features".to_string(), "auth,api".to_string());

        let options = PromptOptions {
            data_overrides: overrides,
            use_defaults: false,
        };

        let result = collect_variables(&config, &options).unwrap();

        let expected = Value::Array(vec![
            Value::String("auth".to_string()),
            Value::String("api".to_string()),
        ]);
        assert_eq!(result.get("features").unwrap(), &expected);
    }

    #[test]
    fn test_multiselect_with_default() {
        let mut variables = BTreeMap::new();
        variables.insert(
            "features".to_string(),
            VariableConfig {
                var_type: VariableType::Multiselect,
                prompt: None,
                default: Some(toml::Value::Array(vec![
                    toml::Value::String("auth".to_string()),
                    toml::Value::String("db".to_string()),
                ])),
                choices: Some(vec![
                    "auth".to_string(),
                    "api".to_string(),
                    "db".to_string(),
                ]),
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

        let expected = Value::Array(vec![
            Value::String("auth".to_string()),
            Value::String("db".to_string()),
        ]);
        assert_eq!(result.get("features").unwrap(), &expected);
    }

    #[test]
    fn test_when_condition_true() {
        let mut variables = BTreeMap::new();
        variables.insert(
            "enable_feature".to_string(),
            VariableConfig {
                var_type: VariableType::Bool,
                prompt: None,
                default: Some(toml::Value::Boolean(true)),
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

        assert_eq!(result.len(), 2);
        assert_eq!(result.get("enable_feature").unwrap(), &Value::Bool(true));
        assert_eq!(result.get("feature_config").unwrap(), "advanced");
    }

    #[test]
    fn test_when_condition_false() {
        let mut variables = BTreeMap::new();
        variables.insert(
            "enable_feature".to_string(),
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

        // feature_config should be skipped because enable_feature is false
        assert_eq!(result.len(), 1);
        assert_eq!(result.get("enable_feature").unwrap(), &Value::Bool(false));
        assert!(result.get("feature_config").is_none());
    }

    #[test]
    fn test_computed_variable_depends_on_another() {
        let mut variables = BTreeMap::new();
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
    fn test_toml_to_tera_value_conversions() {
        // Test string
        let val = toml_to_tera_value(&toml::Value::String("test".to_string()));
        assert_eq!(val, Value::String("test".to_string()));

        // Test integer
        let val = toml_to_tera_value(&toml::Value::Integer(42));
        assert_eq!(val, Value::Number(serde_json::Number::from(42)));

        // Test boolean
        let val = toml_to_tera_value(&toml::Value::Boolean(true));
        assert_eq!(val, Value::Bool(true));

        // Test array
        let arr = toml::Value::Array(vec![
            toml::Value::String("a".to_string()),
            toml::Value::String("b".to_string()),
        ]);
        let val = toml_to_tera_value(&arr);
        assert_eq!(
            val,
            Value::Array(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
            ])
        );
    }

    #[test]
    fn test_integer_with_default() {
        let mut variables = BTreeMap::new();
        variables.insert(
            "port".to_string(),
            VariableConfig {
                var_type: VariableType::Int,
                prompt: None,
                default: Some(toml::Value::Integer(8080)),
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

        assert_eq!(
            result.get("port").unwrap(),
            &Value::Number(serde_json::Number::from(8080))
        );
    }

    #[test]
    fn test_float_with_default() {
        let mut variables = BTreeMap::new();
        variables.insert(
            "threshold".to_string(),
            VariableConfig {
                var_type: VariableType::Float,
                prompt: None,
                default: Some(toml::Value::Float(0.5)),
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

        let expected = serde_json::to_value(0.5).unwrap();
        assert_eq!(result.get("threshold").unwrap(), &expected);
    }

    #[test]
    fn test_boolean_override_yes() {
        let mut variables = BTreeMap::new();
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
        overrides.insert("enabled".to_string(), "yes".to_string());

        let options = PromptOptions {
            data_overrides: overrides,
            use_defaults: false,
        };

        let result = collect_variables(&config, &options).unwrap();

        assert_eq!(result.get("enabled").unwrap(), &Value::Bool(true));
    }

    #[test]
    fn test_invalid_integer_override_falls_back_to_string() {
        let mut variables = BTreeMap::new();
        variables.insert(
            "port".to_string(),
            VariableConfig {
                var_type: VariableType::Int,
                prompt: None,
                default: Some(toml::Value::Integer(8080)),
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
        overrides.insert("port".to_string(), "not-a-number".to_string());

        let options = PromptOptions {
            data_overrides: overrides,
            use_defaults: false,
        };

        let result = collect_variables(&config, &options).unwrap();

        // Invalid integer should fall back to string
        assert_eq!(result.get("port").unwrap(), "not-a-number");
    }

    #[test]
    fn test_invalid_float_override_falls_back_to_string() {
        let mut variables = BTreeMap::new();
        variables.insert(
            "threshold".to_string(),
            VariableConfig {
                var_type: VariableType::Float,
                prompt: None,
                default: Some(toml::Value::Float(0.5)),
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
        overrides.insert("threshold".to_string(), "not-a-float".to_string());

        let options = PromptOptions {
            data_overrides: overrides,
            use_defaults: false,
        };

        let result = collect_variables(&config, &options).unwrap();

        // Invalid float should fall back to string
        assert_eq!(result.get("threshold").unwrap(), "not-a-float");
    }

    #[test]
    fn test_multiselect_override_with_spaces() {
        let mut variables = BTreeMap::new();
        variables.insert(
            "features".to_string(),
            VariableConfig {
                var_type: VariableType::Multiselect,
                prompt: None,
                default: None,
                choices: Some(vec!["auth".to_string(), "api".to_string()]),
                validation: None,
                validation_message: None,
                when: None,
                computed: None,
                secret: false,
            },
        );

        let config = minimal_config(variables);
        let mut overrides = HashMap::new();
        overrides.insert("features".to_string(), " auth , api ".to_string());

        let options = PromptOptions {
            data_overrides: overrides,
            use_defaults: false,
        };

        let result = collect_variables(&config, &options).unwrap();

        // Should trim whitespace from items
        let expected = Value::Array(vec![
            Value::String("auth".to_string()),
            Value::String("api".to_string()),
        ]);
        assert_eq!(result.get("features").unwrap(), &expected);
    }

    #[test]
    fn test_computed_variable_evaluation_error() {
        let mut variables = BTreeMap::new();
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

    #[test]
    fn test_when_condition_undefined_var_is_falsy() {
        let mut variables = BTreeMap::new();
        variables.insert(
            "conditional".to_string(),
            VariableConfig {
                var_type: VariableType::String,
                prompt: None,
                default: Some(toml::Value::String("value".to_string())),
                choices: None,
                validation: None,
                validation_message: None,
                when: Some("undefined_var".to_string()),
                computed: None,
                secret: false,
            },
        );

        let config = minimal_config(variables);
        let options = PromptOptions {
            data_overrides: HashMap::new(),
            use_defaults: true,
        };

        // undefined_var is treated as falsy, so conditional should be skipped
        let result = collect_variables(&config, &options).unwrap();
        assert!(result.get("conditional").is_none());
    }

    #[test]
    fn test_toml_table_conversion() {
        let mut table = toml::map::Map::new();
        table.insert("key".to_string(), toml::Value::String("value".to_string()));
        table.insert("count".to_string(), toml::Value::Integer(42));

        let val = toml_to_tera_value(&toml::Value::Table(table));

        match val {
            Value::Object(map) => {
                assert_eq!(map.get("key").unwrap(), "value");
                assert_eq!(
                    map.get("count").unwrap(),
                    &Value::Number(serde_json::Number::from(42))
                );
            }
            _ => panic!("Expected Value::Object"),
        }
    }

    #[test]
    fn test_toml_datetime_conversion() {
        let datetime_str = "1979-05-27T07:32:00Z";
        let datetime = datetime_str.parse::<toml::value::Datetime>().unwrap();

        let val = toml_to_tera_value(&toml::Value::Datetime(datetime));

        assert_eq!(val, Value::String(datetime_str.to_string()));
    }

    #[test]
    fn test_select_override() {
        let mut variables = BTreeMap::new();
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

        let config = minimal_config(variables);
        let mut overrides = HashMap::new();
        overrides.insert("license".to_string(), "Apache-2.0".to_string());

        let options = PromptOptions {
            data_overrides: overrides,
            use_defaults: false,
        };

        let result = collect_variables(&config, &options).unwrap();

        assert_eq!(result.get("license").unwrap(), "Apache-2.0");
    }
}
