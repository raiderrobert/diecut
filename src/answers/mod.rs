use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use serde::{Deserialize, Serialize};
use tera::Value;

use crate::config::schema::TemplateConfig;
use crate::error::{DicecutError, Result};

pub struct SourceInfo {
    pub url: Option<String>,
    pub git_ref: Option<String>,
    pub commit_sha: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedAnswers {
    pub template_source: String,
    pub template_ref: Option<String>,
    pub commit_sha: Option<String>,
    pub diecut_version: String,
    pub answers: HashMap<String, toml::Value>,
}

pub fn load_answers(project_path: &Path) -> Result<SavedAnswers> {
    let answers_path = project_path.join(".diecut-answers.toml");
    if !answers_path.exists() {
        return Err(DicecutError::NoAnswerFile {
            path: project_path.to_path_buf(),
        });
    }

    let content = std::fs::read_to_string(&answers_path).map_err(|e| DicecutError::Io {
        context: format!("reading answers file {}", answers_path.display()),
        source: e,
    })?;

    let table: toml::Value =
        toml::from_str(&content).map_err(|e| DicecutError::AnswerFileParseError {
            path: answers_path.clone(),
            source: e,
        })?;

    let empty_table = toml::map::Map::new();
    let diecut_section = table.get("_diecut").and_then(toml::Value::as_table);
    let meta = diecut_section.unwrap_or(&empty_table);

    let get_str = |key: &str| -> Option<&str> { meta.get(key).and_then(toml::Value::as_str) };

    let template_source = get_str("template_source")
        .or_else(|| get_str("template"))
        .unwrap_or("")
        .to_string();

    let template_ref = get_str("template_ref").map(String::from);

    let commit_sha = get_str("commit_sha").map(String::from);

    let diecut_version = get_str("diecut_version").unwrap_or("0.0.0").to_string();

    let vars_table = table
        .get("variables")
        .and_then(toml::Value::as_table)
        .cloned()
        .unwrap_or_default();

    let answers: HashMap<String, toml::Value> = vars_table.into_iter().collect();

    Ok(SavedAnswers {
        template_source,
        template_ref,
        commit_sha,
        diecut_version,
        answers,
    })
}

/// Excludes secret variables. Includes template source metadata for `diecut update`.
pub fn write_answers(
    output_dir: &Path,
    config: &TemplateConfig,
    variables: &BTreeMap<String, Value>,
    source_info: &SourceInfo,
) -> Result<()> {
    write_answers_with_source(
        output_dir,
        config,
        variables,
        source_info.url.as_deref(),
        source_info.git_ref.as_deref(),
        source_info.commit_sha.as_deref(),
    )
}

pub fn write_answers_with_source(
    output_dir: &Path,
    config: &TemplateConfig,
    variables: &BTreeMap<String, Value>,
    template_source: Option<&str>,
    template_ref: Option<&str>,
    commit_sha: Option<&str>,
) -> Result<()> {
    let answers_path = output_dir.join(&config.answers.file);

    let mut table = toml::map::Map::new();

    let mut meta = toml::map::Map::new();
    meta.insert(
        "template".to_string(),
        toml::Value::String(config.template.name.clone()),
    );
    if let Some(version) = &config.template.version {
        meta.insert("version".to_string(), toml::Value::String(version.clone()));
    }
    if let Some(source) = template_source {
        meta.insert(
            "template_source".to_string(),
            toml::Value::String(source.to_string()),
        );
    }
    if let Some(git_ref) = template_ref {
        meta.insert(
            "template_ref".to_string(),
            toml::Value::String(git_ref.to_string()),
        );
    }
    if let Some(sha) = commit_sha {
        meta.insert(
            "commit_sha".to_string(),
            toml::Value::String(sha.to_string()),
        );
    }
    meta.insert(
        "diecut_version".to_string(),
        toml::Value::String(env!("CARGO_PKG_VERSION").to_string()),
    );
    table.insert("_diecut".to_string(), toml::Value::Table(meta));

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_write_answers_basic() {
        let output_dir = tempfile::tempdir().unwrap();

        let config = crate::config::schema::TemplateConfig {
            template: crate::config::schema::TemplateMetadata {
                name: "test-template".to_string(),
                version: Some("1.0.0".to_string()),
                description: None,
                min_diecut_version: None,
                templates_suffix: ".tera".to_string(),
            },
            variables: BTreeMap::new(),
            files: crate::config::schema::FilesConfig::default(),
            hooks: crate::config::schema::HooksConfig { post_create: None },
            answers: crate::config::schema::AnswersConfig::default(),
        };

        let mut variables = BTreeMap::new();
        variables.insert(
            "project_name".to_string(),
            Value::String("my-project".to_string()),
        );
        variables.insert("author".to_string(), Value::String("Jane Doe".to_string()));

        let source_info = SourceInfo {
            url: None,
            git_ref: None,
            commit_sha: None,
        };

        let result = write_answers(output_dir.path(), &config, &variables, &source_info);

        assert!(result.is_ok());

        let answers_file = output_dir.path().join(".diecut-answers.toml");
        assert!(answers_file.exists());

        let content = fs::read_to_string(&answers_file).unwrap();
        assert!(content.contains("project_name"));
        assert!(content.contains("my-project"));
        assert!(content.contains("author"));
        assert!(content.contains("Jane Doe"));
    }

    #[test]
    fn test_write_answers_includes_template_metadata() {
        let output_dir = tempfile::tempdir().unwrap();

        let config = crate::config::schema::TemplateConfig {
            template: crate::config::schema::TemplateMetadata {
                name: "my-template".to_string(),
                version: Some("2.1.0".to_string()),
                description: Some("A test template".to_string()),
                min_diecut_version: None,
                templates_suffix: ".tera".to_string(),
            },
            variables: BTreeMap::new(),
            files: crate::config::schema::FilesConfig::default(),
            hooks: crate::config::schema::HooksConfig { post_create: None },
            answers: crate::config::schema::AnswersConfig::default(),
        };

        let variables = BTreeMap::new();
        let source_info = SourceInfo {
            url: None,
            git_ref: None,
            commit_sha: None,
        };

        write_answers(output_dir.path(), &config, &variables, &source_info).unwrap();

        let answers_file = output_dir.path().join(".diecut-answers.toml");
        let content = fs::read_to_string(&answers_file).unwrap();

        assert!(content.contains("my-template"));
        assert!(content.contains("2.1.0"));
    }

    #[test]
    fn test_write_answers_includes_git_source() {
        let output_dir = tempfile::tempdir().unwrap();

        let config = crate::config::schema::TemplateConfig {
            template: crate::config::schema::TemplateMetadata {
                name: "test".to_string(),
                version: None,
                description: None,
                min_diecut_version: None,
                templates_suffix: ".tera".to_string(),
            },
            variables: BTreeMap::new(),
            files: crate::config::schema::FilesConfig::default(),
            hooks: crate::config::schema::HooksConfig { post_create: None },
            answers: crate::config::schema::AnswersConfig::default(),
        };

        let variables = BTreeMap::new();
        let source_info = SourceInfo {
            url: Some("https://github.com/user/repo.git".to_string()),
            git_ref: Some("main".to_string()),
            commit_sha: Some("abc123def456".to_string()),
        };

        write_answers(output_dir.path(), &config, &variables, &source_info).unwrap();

        let answers_file = output_dir.path().join(".diecut-answers.toml");
        let content = fs::read_to_string(&answers_file).unwrap();

        assert!(content.contains("https://github.com/user/repo.git"));
        assert!(content.contains("main"));
        assert!(content.contains("abc123def456"));
    }
}
