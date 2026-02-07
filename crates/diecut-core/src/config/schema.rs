use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::variable::{VariableConfig, VariableType};
use crate::error::{DicecutError, Result};

/// Root config structure deserialized from diecut.toml.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TemplateConfig {
    pub template: TemplateMetadata,

    /// Variables in declaration order (BTreeMap preserves insertion order from TOML).
    #[serde(default)]
    pub variables: BTreeMap<String, VariableConfig>,

    #[serde(default)]
    pub files: FilesConfig,

    #[serde(default)]
    pub hooks: HooksConfig,

    #[serde(default)]
    pub answers: AnswersConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TemplateMetadata {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub min_diecut_version: Option<String>,

    /// Suffix for template files (default: ".tera").
    #[serde(default = "default_templates_suffix")]
    pub templates_suffix: String,
}

fn default_templates_suffix() -> String {
    ".tera".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FilesConfig {
    /// Glob patterns for files to exclude from output.
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Glob patterns for files to copy without rendering.
    #[serde(default)]
    pub copy_without_render: Vec<String>,

    /// Conditional file inclusion rules.
    #[serde(default)]
    pub conditional: Vec<ConditionalFile>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConditionalFile {
    /// Glob pattern matching files affected by this rule.
    pub pattern: String,

    /// Tera expression — if false, matched files are excluded.
    pub when: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct HooksConfig {
    #[serde(default)]
    pub pre_generate: Vec<String>,
    #[serde(default)]
    pub post_generate: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnswersConfig {
    /// Filename for the answers file written into the generated project.
    #[serde(default = "default_answers_file")]
    pub file: String,
}

fn default_answers_file() -> String {
    ".diecut-answers.toml".to_string()
}

impl Default for AnswersConfig {
    fn default() -> Self {
        Self {
            file: default_answers_file(),
        }
    }
}

impl TemplateConfig {
    /// Validate the config for internal consistency.
    pub fn validate(&self) -> Result<()> {
        for (name, var) in &self.variables {
            // select/multiselect must have choices
            if matches!(
                var.var_type,
                VariableType::Select | VariableType::Multiselect
            ) && var.choices.is_none()
            {
                return Err(DicecutError::ConfigInvalidVariable {
                    name: name.clone(),
                    reason: "select/multiselect variables must have 'choices' defined".into(),
                });
            }

            // computed variables shouldn't have a prompt
            if var.computed.is_some() && var.prompt.is_some() {
                return Err(DicecutError::ConfigInvalidVariable {
                    name: name.clone(),
                    reason: "computed variables should not have a 'prompt' field".into(),
                });
            }
        }

        Ok(())
    }
}
