pub mod cookiecutter;
pub mod detect;
pub mod migrate;
pub mod native;

use std::path::PathBuf;

use crate::config::schema::TemplateConfig;
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateFormat {
    Native,
    Cookiecutter,
}

/// Adapter output consumed by the generation pipeline regardless of source format.
pub struct ResolvedTemplate {
    pub config: TemplateConfig,
    pub content_dir: PathBuf,
    pub format: TemplateFormat,
    /// If true, render all text files (no suffix gating).
    pub render_all: bool,
    /// Nest variables under this key for rendering (e.g. "cookiecutter").
    pub context_namespace: Option<String>,
    pub warnings: Vec<String>,
}

pub fn resolve_template(template_dir: &std::path::Path) -> Result<ResolvedTemplate> {
    let format = detect::detect_format(template_dir)?;
    match format {
        TemplateFormat::Native => native::resolve(template_dir),
        TemplateFormat::Cookiecutter => cookiecutter::resolve(template_dir),
    }
}
