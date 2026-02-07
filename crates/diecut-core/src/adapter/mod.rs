pub mod cookiecutter;
pub mod detect;
pub mod migrate;
pub mod native;

use std::path::PathBuf;

use crate::config::schema::TemplateConfig;
use crate::error::Result;

/// Supported template formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateFormat {
    Native,
    Cookiecutter,
}

/// The universal contract between format-specific parsing and the generation pipeline.
/// Every adapter produces one of these; the pipeline consumes it without knowing the source format.
pub struct ResolvedTemplate {
    /// Normalized config (always diecut's internal model).
    pub config: TemplateConfig,
    /// Where template files live on disk.
    pub content_dir: PathBuf,
    /// Which format was detected.
    pub format: TemplateFormat,
    /// If true, render all text files (no suffix gating).
    pub render_all: bool,
    /// Namespace to nest variables under for template rendering (e.g. "cookiecutter").
    pub context_namespace: Option<String>,
    /// Non-fatal warnings (e.g. "Python hooks detected, not supported").
    pub warnings: Vec<String>,
}

/// Detect the template format and resolve it into a `ResolvedTemplate`.
pub fn resolve_template(template_dir: &std::path::Path) -> Result<ResolvedTemplate> {
    let format = detect::detect_format(template_dir)?;
    match format {
        TemplateFormat::Native => native::resolve(template_dir),
        TemplateFormat::Cookiecutter => cookiecutter::resolve(template_dir),
    }
}
