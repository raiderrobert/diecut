use std::path::Path;

use crate::adapter::{ResolvedTemplate, TemplateFormat};
use crate::config::load_config;
use crate::error::Result;

/// Resolve a native diecut template into a `ResolvedTemplate`.
/// This is a thin wrapper around `load_config()` that preserves existing behavior.
pub fn resolve(template_dir: &Path) -> Result<ResolvedTemplate> {
    let config = load_config(template_dir)?;
    let content_dir = template_dir.join("template");

    Ok(ResolvedTemplate {
        config,
        content_dir,
        format: TemplateFormat::Native,
        render_all: false,
        context_namespace: None,
        warnings: Vec::new(),
    })
}
