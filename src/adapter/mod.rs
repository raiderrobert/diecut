pub mod native;

use std::path::PathBuf;

use crate::config::schema::TemplateConfig;
use crate::error::Result;

pub struct ResolvedTemplate {
    pub config: TemplateConfig,
    pub content_dir: PathBuf,
    pub warnings: Vec<String>,
}

pub fn resolve_template(template_dir: &std::path::Path) -> Result<ResolvedTemplate> {
    native::resolve(template_dir)
}
