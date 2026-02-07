use std::path::{Path, PathBuf};

use crate::error::{DicecutError, Result};

/// Resolved template source — for M1, only local paths.
pub enum TemplateSource {
    Local(PathBuf),
}

/// Resolve a template argument to a source.
/// For M1, only local paths are supported.
pub fn resolve_source(template_arg: &str) -> Result<TemplateSource> {
    let path = Path::new(template_arg);
    if path.exists() {
        Ok(TemplateSource::Local(path.canonicalize().map_err(|e| {
            DicecutError::Io {
                context: format!("resolving path {}", path.display()),
                source: e,
            }
        })?))
    } else {
        Err(DicecutError::ConfigNotFound {
            path: path.to_path_buf(),
        })
    }
}
