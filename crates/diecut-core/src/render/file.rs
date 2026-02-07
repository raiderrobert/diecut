use std::path::Path;

use tera::{Context, Tera};

use crate::error::{DicecutError, Result};

/// Render a single template file's content through Tera.
pub fn render_file_content(tera: &Tera, template_name: &str, context: &Context) -> Result<String> {
    tera.render(template_name, context)
        .map_err(|e| DicecutError::RenderError { source: e })
}

/// Render a path component (filename or dirname) through Tera.
/// This handles template expressions in directory and file names, like `{{project_name}}`.
pub fn render_path_component(component: &str, context: &Context) -> Result<String> {
    let mut tera = Tera::default();
    tera.add_raw_template("__path__", component).map_err(|e| {
        DicecutError::FilenameRenderError {
            filename: component.to_string(),
            source: e,
        }
    })?;

    tera.render("__path__", context)
        .map_err(|e| DicecutError::FilenameRenderError {
            filename: component.to_string(),
            source: e,
        })
}

/// Detect if a file is likely binary by reading the first few kilobytes.
pub fn is_binary_file(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    let check_len = bytes.len().min(8192);
    bytes[..check_len].contains(&0)
}
