use std::path::Path;

use tera::{Context, Tera};

use crate::error::{DicecutError, Result};

pub fn render_file_content(tera: &Tera, template_name: &str, context: &Context) -> Result<String> {
    tera.render(template_name, context)
        .map_err(|e| DicecutError::RenderError {
            file: template_name.to_string(),
            source: e,
        })
}

/// Render template expressions in a path component (e.g. `{{project_name}}`).
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

pub fn is_binary_file(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    let check_len = bytes.len().min(8192);
    bytes[..check_len].contains(&0)
}
