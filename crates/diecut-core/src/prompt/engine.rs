use std::collections::{BTreeMap, HashMap};

use tera::{Context, Tera, Value};

use crate::config::schema::TemplateConfig;
use crate::config::variable::{VariableConfig, VariableType};
use crate::error::{DicecutError, Result};

/// Options controlling how variables are collected.
#[derive(Default)]
pub struct PromptOptions {
    /// Pre-supplied key=value overrides (from --data flags).
    pub data_overrides: HashMap<String, String>,
    /// If true, use defaults without prompting.
    pub use_defaults: bool,
}

/// Collect all variable values by prompting the user (or using overrides/defaults).
/// Returns a map of variable name → Tera Value.
pub fn collect_variables(
    config: &TemplateConfig,
    options: &PromptOptions,
) -> Result<BTreeMap<String, Value>> {
    let mut values: BTreeMap<String, Value> = BTreeMap::new();

    // First pass: collect prompted variables
    for (name, var) in &config.variables {
        if var.computed.is_some() {
            continue; // computed vars are handled in second pass
        }

        // Evaluate `when` condition if present
        if let Some(when_expr) = &var.when {
            if !evaluate_when(name, when_expr, &values)? {
                continue; // condition is false, skip
            }
        }

        // Check for --data override
        if let Some(override_val) = options.data_overrides.get(name) {
            let value = parse_override(override_val, var);
            values.insert(name.clone(), value);
            continue;
        }

        // Use defaults if --defaults flag is set
        if options.use_defaults {
            if let Some(default) = &var.default {
                values.insert(name.clone(), toml_to_tera_value(default));
                continue;
            }
        }

        // Interactive prompt
        let value = prompt_variable(name, var)?;
        values.insert(name.clone(), value);
    }

    // Second pass: evaluate computed variables iteratively.
    // Some computed variables depend on others, so we keep trying until all are resolved.
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
