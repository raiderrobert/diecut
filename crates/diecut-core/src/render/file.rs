use std::path::Path;

use tera::{Context, Tera};

use crate::error::{DicecutError, Result};

pub fn render_file_content(tera: &Tera, template_name: &str, context: &Context) -> Result<String> {
    tera.render(template_name, context)
        .map_err(|e| DicecutError::RenderError { source: e })
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

/// Detect binary files by checking for null bytes in the first 8KB.
///
/// Only reads the first 8KB rather than the entire file, avoiding
/// unnecessary memory allocation for large binary assets.
pub fn is_binary_file(path: &Path) -> bool {
    use std::io::Read;

    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };

    let mut buf = [0u8; 8192];
    let Ok(n) = file.take(8192).read(&mut buf) else {
        return false;
    };

    buf[..n].contains(&0)
}
