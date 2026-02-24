use std::path::Path;

use walkdir::WalkDir;

use crate::adapter::ResolvedTemplate;
use crate::config::load_config;
use crate::config::schema::{DEFAULT_TEMPLATES_SUFFIX, DEPRECATED_TERA_SUFFIX};
use crate::error::Result;

pub fn resolve(template_dir: &Path) -> Result<ResolvedTemplate> {
    let mut config = load_config(template_dir)?;
    let content_dir = template_dir.join("template");
    let mut warnings = Vec::new();

    if config.template.templates_suffix.is_none() {
        if content_dir.exists() && has_files_with_suffix(&content_dir, DEPRECATED_TERA_SUFFIX) {
            config.template.templates_suffix = Some(DEPRECATED_TERA_SUFFIX.to_string());
            warnings.push(format!(
                "This template uses {DEPRECATED_TERA_SUFFIX} file extensions. \
                 The default template suffix is now \"{DEFAULT_TEMPLATES_SUFFIX}\". \
                 Consider renaming your template files from \"{DEPRECATED_TERA_SUFFIX}\" to \"{DEFAULT_TEMPLATES_SUFFIX}\". \
                 To suppress this warning, set templates_suffix = \"{DEPRECATED_TERA_SUFFIX}\" in diecut.toml."
            ));
        } else {
            config.template.templates_suffix = Some(DEFAULT_TEMPLATES_SUFFIX.to_string());
        }
    }

    Ok(ResolvedTemplate {
        config,
        content_dir,
        warnings,
    })
}

fn has_files_with_suffix(dir: &Path, suffix: &str) -> bool {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .any(|e| e.path().to_string_lossy().ends_with(suffix))
}
