use std::collections::BTreeMap;
use std::path::Path;

use tera::Value;

use crate::config::schema::TemplateConfig;
use crate::error::{DicecutError, Result};

/// Write the answers file into the generated project directory.
/// Excludes secret variables.
pub fn write_answers(
    output_dir: &Path,
    config: &TemplateConfig,
    variables: &BTreeMap<String, Value>,
) -> Result<()> {
    let answers_path = output_dir.join(&config.answers.file);

    let mut table = toml::map::Map::new();

    // Record the template source info
    let mut meta = toml::map::Map::new();
    meta.insert(
        "template".to_string(),
        toml::Value::String(config.template.name.clone()),
    );
    if let Some(version) = &config.template.version {
        meta.insert("version".to_string(), toml::Value::String(version.clone()));
    }
    table.insert("_diecut".to_string(), toml::Value::Table(meta));

    // Record variable values (skip secrets)
    let mut vars = toml::map::Map::new();
    for (name, value) in variables {
        if let Some(var_config) = config.variables.get(name) {
            if var_config.secret {
                continue;
            }
        }
        if let Some(toml_val) = tera_value_to_toml(value) {
            vars.insert(name.clone(), toml_val);
        }
    }
    table.insert("variables".to_string(), toml::Value::Table(vars));

    let content = toml::to_string_pretty(&table).map_err(|e| DicecutError::Io {
        context: format!("serializing answers to {}", answers_path.display()),
        source: std::io::Error::other(e),
    })?;

    std::fs::write(&answers_path, content).map_err(|e| DicecutError::Io {
        context: format!("writing answers file {}", answers_path.display()),
        source: e,
    })?;

    Ok(())
}

fn tera_value_to_toml(value: &Value) -> Option<toml::Value> {
    match value {
        Value::String(s) => Some(toml::Value::String(s.clone())),
        Value::Bool(b) => Some(toml::Value::Boolean(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(toml::Value::Integer(i))
            } else {
                n.as_f64().map(toml::Value::Float)
            }
        }
        Value::Array(arr) => {
            let items: Vec<toml::Value> = arr.iter().filter_map(tera_value_to_toml).collect();
            Some(toml::Value::Array(items))
        }
        _ => None,
    }
}
