use std::path::Path;

use crate::adapter::TemplateFormat;
use crate::error::{DicecutError, Result};

/// Priority: diecut.toml > cookiecutter.json (so a migrated project uses native).
pub fn detect_format(template_dir: &Path) -> Result<TemplateFormat> {
    if template_dir.join("diecut.toml").exists() {
        return Ok(TemplateFormat::Native);
    }

    if template_dir.join("cookiecutter.json").exists() {
        return Ok(TemplateFormat::Cookiecutter);
    }

    Err(DicecutError::UnsupportedFormat {
        path: template_dir.to_path_buf(),
    })
}
