use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::variable::{VariableConfig, VariableType};
use crate::error::{DicecutError, Result};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TemplateConfig {
    pub template: TemplateMetadata,

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

    #[serde(default = "default_templates_suffix")]
    pub templates_suffix: String,
}

fn default_templates_suffix() -> String {
    ".tera".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FilesConfig {
    #[serde(default)]
    pub exclude: Vec<String>,

    #[serde(default)]
    pub copy_without_render: Vec<String>,

    #[serde(default)]
    pub conditional: Vec<ConditionalFile>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConditionalFile {
    pub pattern: String,
    /// Tera expression — if false, matched files are excluded.
    pub when: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct HooksConfig {
    /// Shell command to run in the output directory after generation.
    #[serde(default)]
    pub post_create: Option<String>,
}

impl HooksConfig {
    pub fn has_hooks(&self) -> bool {
        self.post_create.is_some()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnswersConfig {
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
