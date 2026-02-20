use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum VariableType {
    #[default]
    String,
    Bool,
    Int,
    Float,
    Select,
    Multiselect,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct VariableConfig {
    #[serde(rename = "type")]
    pub var_type: VariableType,
    pub prompt: Option<String>,
    pub default: Option<toml::Value>,
    pub choices: Option<Vec<String>>,
    pub validation: Option<String>,
    pub validation_message: Option<String>,
    /// If false, this variable is skipped during prompting.
    pub when: Option<String>,
    /// Tera expression — computed variables are never prompted.
    pub computed: Option<String>,
    #[serde(default)]
    pub secret: bool,
}

impl VariableConfig {
    pub fn is_prompted(&self) -> bool {
        self.computed.is_none()
    }
}
