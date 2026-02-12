pub mod schema;
pub mod user;
pub mod variable;

use std::path::Path;

use crate::error::{DicecutError, Result};

pub use schema::TemplateConfig;
pub use user::{load_user_config, UserConfig};

/// Load and validate a TemplateConfig from a diecut.toml file.
pub fn load_config(path: &Path) -> Result<TemplateConfig> {
    let config_path = if path.ends_with("diecut.toml") {
        path.to_path_buf()
    } else {
        path.join("diecut.toml")
    };

    if !config_path.exists() {
        return Err(DicecutError::ConfigNotFound { path: config_path });
    }

    let content = std::fs::read_to_string(&config_path).map_err(|e| DicecutError::Io {
        context: format!("reading {}", config_path.display()),
        source: e,
    })?;

    let config: TemplateConfig =
        toml::from_str(&content).map_err(|e| DicecutError::ConfigParse { source: e })?;

    config.validate()?;

    Ok(config)
}
