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

/// Detect binary files using content_inspector (BOM-aware, null-byte scanning).
///
/// Reads only the first 8KB to avoid unnecessary allocation for large files.
pub fn is_binary_file(path: &Path) -> bool {
    use std::io::Read;

    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };

    let mut buf = [0u8; 8192];
    let Ok(n) = file.take(8192).read(&mut buf) else {
        return false;
    };

    !content_inspector::inspect(&buf[..n]).is_text()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile;

    #[test]
    fn test_is_binary_file_with_text_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "Hello, world!").unwrap();

        assert!(!is_binary_file(&file));
    }

    #[test]
    fn test_is_binary_file_with_binary_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.bin");
        // Create a binary file with null bytes which is definitely binary
        let binary_data: Vec<u8> = (0..256).map(|i| i as u8).collect();
        fs::write(&file, &binary_data).unwrap();

        assert!(is_binary_file(&file));
    }

    #[test]
    fn test_is_binary_file_nonexistent_file() {
        let result = is_binary_file(&std::path::PathBuf::from("/nonexistent/file.txt"));
        assert!(!result);
    }

    #[test]
    fn test_render_path_component() {
        let mut context = Context::new();
        context.insert("project_name", "my-project");

        let result = render_path_component("{{project_name}}", &context).unwrap();
        assert_eq!(result, "my-project");
    }

    #[test]
    fn test_render_path_component_error() {
        let context = Context::new();

        let result = render_path_component("{{invalid_var}}", &context);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, DicecutError::FilenameRenderError { .. }));
        }
    }
}
