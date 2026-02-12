use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{DicecutError, Result};

/// User-level configuration loaded from `~/.config/diecut/config.toml`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UserConfig {
    /// Custom abbreviation mappings. Keys are prefixes (e.g. `"company"`),
    /// values are URL templates with `{}` as placeholder (e.g.
    /// `"https://git.company.com/{}.git"`).
    #[serde(default)]
    pub abbreviations: HashMap<String, String>,
}

/// Get the path to the user config file.
fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("diecut").join("config.toml"))
}

/// Load user configuration from the XDG config directory.
///
/// Returns `Ok(None)` if the config file does not exist.
/// Returns `Err` if the file exists but cannot be read or parsed.
pub fn load_user_config() -> Result<Option<UserConfig>> {
    let path = match config_path() {
        Some(p) => p,
        None => return Ok(None),
    };

    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&path).map_err(|e| DicecutError::Io {
        context: format!("reading user config {}", path.display()),
        source: e,
    })?;

    let config: UserConfig =
        toml::from_str(&content).map_err(|e| DicecutError::ConfigParse { source: e })?;

    Ok(Some(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_user_config() {
        let toml_str = r#"
[abbreviations]
company = "https://git.company.com/{}.git"
internal = "https://internal.example.com/repos/{}"
"#;
        let config: UserConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.abbreviations.len(), 2);
        assert_eq!(
            config.abbreviations["company"],
            "https://git.company.com/{}.git"
        );
        assert_eq!(
            config.abbreviations["internal"],
            "https://internal.example.com/repos/{}"
        );
    }

    #[test]
    fn parse_empty_config() {
        let config: UserConfig = toml::from_str("").unwrap();
        assert!(config.abbreviations.is_empty());
    }

    #[test]
    fn parse_config_without_abbreviations() {
        let config: UserConfig = toml::from_str("[abbreviations]").unwrap();
        assert!(config.abbreviations.is_empty());
    }

    #[test]
    fn parse_malformed_config_errors() {
        let result: std::result::Result<UserConfig, _> = toml::from_str("not valid [[ toml");
        assert!(result.is_err());
    }

    #[test]
    fn load_user_config_returns_none_when_no_file() {
        let result = load_user_config();
        assert!(result.is_ok());
    }
}
