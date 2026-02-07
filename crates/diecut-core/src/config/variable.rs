use serde::{Deserialize, Serialize};

/// The type of a template variable.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VariableType {
    String,
    Bool,
    Int,
    Float,
    Select,
    Multiselect,
}

/// Configuration for a single template variable.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VariableConfig {
    #[serde(rename = "type")]
    pub var_type: VariableType,

    /// Prompt text shown to the user.
    pub prompt: Option<String>,

    /// Default value (type depends on var_type).
    pub default: Option<toml::Value>,

    /// Available choices for select/multiselect types.
    pub choices: Option<Vec<String>>,

    /// Regex pattern for validation (string/int/float types).
    pub validation: Option<String>,

    /// Message shown when validation fails.
    pub validation_message: Option<String>,

    /// Tera expression evaluated to determine if this variable should be prompted.
    /// If it evaluates to false, the variable is skipped.
    pub when: Option<String>,

    /// Tera expression used to compute the value (variable is never prompted).
    pub computed: Option<String>,

    /// If true, the value won't be saved to the answers file.
    #[serde(default)]
    pub secret: bool,
}

impl VariableConfig {
    /// Returns whether this variable should be prompted (not computed).
    pub fn is_prompted(&self) -> bool {
        self.computed.is_none()
    }
}
