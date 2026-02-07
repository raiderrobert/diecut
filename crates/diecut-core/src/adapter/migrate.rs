use std::path::{Path, PathBuf};

use regex_lite::Regex;

use crate::adapter::cookiecutter;
use crate::adapter::detect::detect_format;
use crate::adapter::TemplateFormat;
use crate::config::schema::TemplateConfig;
use crate::error::{DicecutError, Result};

/// A planned file operation for migration.
#[derive(Debug, Clone)]
pub enum FileOp {
    /// Move a file from src to dest.
    Move { src: PathBuf, dest: PathBuf },
    /// Create a new file with the given content.
    Create { path: PathBuf, content: String },
    /// Delete a file.
    Delete { path: PathBuf },
    /// Rewrite a file's contents (src path, with description of what changed).
    Rewrite { path: PathBuf, description: String },
}

/// A migration plan that can be previewed (dry-run) or executed.
#[derive(Debug)]
pub struct MigrationPlan {
    pub source_format: TemplateFormat,
    pub operations: Vec<FileOp>,
    pub diecut_toml_content: String,
    pub warnings: Vec<String>,
}

/// Plan a migration from a foreign template format to native diecut format.
pub fn plan_migration(template_dir: &Path) -> Result<MigrationPlan> {
    let format = detect_format(template_dir)?;

    if format == TemplateFormat::Native {
        return Err(DicecutError::Io {
            context: "template is already in native diecut format".into(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "already native"),
        });
    }

    match format {
        TemplateFormat::Cookiecutter => plan_cookiecutter_migration(template_dir),
        TemplateFormat::Native => unreachable!(),
    }
}

fn plan_cookiecutter_migration(template_dir: &Path) -> Result<MigrationPlan> {
    let resolved = cookiecutter::resolve(template_dir)?;
    let mut operations = Vec::new();
    let mut warnings = resolved.warnings;

    // Find the {{cookiecutter.*}} directory
    let cc_dir = find_cookiecutter_dir(template_dir)?;
    let cc_dir_name = cc_dir.file_name().unwrap().to_string_lossy().to_string();

    // Rewrite the directory name: {{cookiecutter.X}} → {{X}}
    let new_dir_name = rewrite_cc_dirname(&cc_dir_name);

    // Plan: create template/ directory and move the content there
    let template_subdir = PathBuf::from("template");
    let new_content_path = template_subdir.join(&new_dir_name);

    // Walk the {{cookiecutter.*}} directory and plan file moves
    for entry in walkdir::WalkDir::new(&cc_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let src_path = entry.path();
        let rel_path = src_path
            .strip_prefix(&cc_dir)
            .expect("entry must be under cc_dir");

        if entry.file_type().is_dir() {
            continue;
        }

        // Rewrite the relative path: remove cookiecutter. prefix from directory components
        let new_rel = rewrite_cc_path(rel_path);

        let dest = new_content_path.join(&new_rel);

        // Check if file contains template syntax that needs .tera suffix
        let needs_tera_suffix = if src_path.is_file() {
            if let Ok(content) = std::fs::read_to_string(src_path) {
                content.contains("{{") || content.contains("{%")
            } else {
                false
            }
        } else {
            false
        };

        let dest = if needs_tera_suffix {
            let mut d = dest.as_os_str().to_os_string();
            d.push(".tera");
            PathBuf::from(d)
        } else {
            dest
        };

        operations.push(FileOp::Move {
            src: src_path.to_path_buf(),
            dest,
        });

        // Check if the file content needs cookiecutter.X → X rewriting
        if src_path.is_file() {
            if let Ok(content) = std::fs::read_to_string(src_path) {
                if content.contains("cookiecutter.") {
                    let new_path = if needs_tera_suffix {
                        let mut d = new_content_path.join(&new_rel).as_os_str().to_os_string();
                        d.push(".tera");
                        PathBuf::from(d)
                    } else {
                        new_content_path.join(&new_rel)
                    };
                    operations.push(FileOp::Rewrite {
                        path: new_path,
                        description: "rewrite cookiecutter.X → X references".to_string(),
                    });
                }
            }
        }
    }

    // Generate diecut.toml content
    let diecut_toml = generate_diecut_toml(&resolved.config);

    // Plan: create diecut.toml
    operations.push(FileOp::Create {
        path: PathBuf::from("diecut.toml"),
        content: diecut_toml.clone(),
    });

    // Plan: delete cookiecutter.json
    operations.push(FileOp::Delete {
        path: PathBuf::from("cookiecutter.json"),
    });

    // Check for hooks directory
    if template_dir.join("hooks").exists() {
        warnings.push(
            "Python hooks cannot be migrated automatically — remove hooks/ and reimplement if needed"
                .to_string(),
        );
    }

    Ok(MigrationPlan {
        source_format: TemplateFormat::Cookiecutter,
        operations,
        diecut_toml_content: diecut_toml,
        warnings,
    })
}

/// Execute a migration plan, writing files to the output directory.
pub fn execute_migration(
    plan: &MigrationPlan,
    template_dir: &Path,
    output_dir: &Path,
) -> Result<()> {
    // Create output directory
    std::fs::create_dir_all(output_dir).map_err(|e| DicecutError::Io {
        context: format!("creating output directory {}", output_dir.display()),
        source: e,
    })?;

    let cc_ref_re = Regex::new(r"cookiecutter\.(\w+)").expect("valid regex");

    for op in &plan.operations {
        match op {
            FileOp::Move { src, dest } => {
                let abs_dest = output_dir.join(dest);
                if let Some(parent) = abs_dest.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| DicecutError::Io {
                        context: format!("creating directory {}", parent.display()),
                        source: e,
                    })?;
                }

                // Copy the file (rewrite content if it contains cookiecutter refs)
                if let Ok(content) = std::fs::read_to_string(src) {
                    let rewritten = cc_ref_re.replace_all(&content, "$1").to_string();
                    std::fs::write(&abs_dest, rewritten).map_err(|e| DicecutError::Io {
                        context: format!("writing {}", abs_dest.display()),
                        source: e,
                    })?;
                } else {
                    // Binary file — just copy
                    std::fs::copy(src, &abs_dest).map_err(|e| DicecutError::Io {
                        context: format!("copying {} to {}", src.display(), abs_dest.display()),
                        source: e,
                    })?;
                }
            }
            FileOp::Create { path, content } => {
                let abs_path = output_dir.join(path);
                if let Some(parent) = abs_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| DicecutError::Io {
                        context: format!("creating directory {}", parent.display()),
                        source: e,
                    })?;
                }
                std::fs::write(&abs_path, content).map_err(|e| DicecutError::Io {
                    context: format!("writing {}", abs_path.display()),
                    source: e,
                })?;
            }
            FileOp::Delete { path } => {
                let abs_path = template_dir.join(path);
                if abs_path.exists() {
                    // In migration to a new output dir, we don't delete from the source
                    // This operation is informational (the source cookiecutter.json is not copied)
                }
            }
            FileOp::Rewrite { .. } => {
                // Rewrites are already handled during the Move step
            }
        }
    }

    Ok(())
}

/// Find the {{cookiecutter.*}} directory in a cookiecutter template.
fn find_cookiecutter_dir(template_dir: &Path) -> Result<PathBuf> {
    let entries = std::fs::read_dir(template_dir).map_err(|e| DicecutError::Io {
        context: format!("reading directory {}", template_dir.display()),
        source: e,
    })?;

    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("{{") && name.contains("cookiecutter.") {
            return Ok(entry.path());
        }
    }

    Err(DicecutError::CookiecutterTemplateDir {
        path: template_dir.to_path_buf(),
    })
}

/// Rewrite a {{cookiecutter.X}} directory name to {{X}}.
fn rewrite_cc_dirname(name: &str) -> String {
    let re = Regex::new(r"\{\{\s*cookiecutter\.(\w+)\s*\}\}").expect("valid regex");
    re.replace_all(name, "{{$1}}").to_string()
}

/// Rewrite path components, removing cookiecutter. prefix.
fn rewrite_cc_path(rel_path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in rel_path.components() {
        let part = component.as_os_str().to_string_lossy();
        let rewritten = rewrite_cc_dirname(&part);
        result.push(rewritten);
    }
    result
}

/// Generate a diecut.toml from a TemplateConfig.
fn generate_diecut_toml(config: &TemplateConfig) -> String {
    // Build the TOML manually for cleaner output
    let mut lines = Vec::new();

    lines.push("[template]".to_string());
    lines.push(format!("name = \"{}\"", config.template.name));
    if let Some(ver) = &config.template.version {
        lines.push(format!("version = \"{}\"", ver));
    }
    if let Some(desc) = &config.template.description {
        lines.push(format!("description = \"{}\"", desc));
    }
    lines.push(String::new());

    // Variables
    for (name, var) in &config.variables {
        lines.push(format!("[variables.{}]", name));
        lines.push(format!("type = \"{}\"", var_type_to_str(&var.var_type)));

        if let Some(prompt) = &var.prompt {
            lines.push(format!("prompt = \"{}\"", prompt));
        }

        if let Some(default) = &var.default {
            lines.push(format!("default = {}", toml_value_to_inline(default)));
        }

        if let Some(choices) = &var.choices {
            let choices_str: Vec<String> = choices.iter().map(|c| format!("\"{}\"", c)).collect();
            lines.push(format!("choices = [{}]", choices_str.join(", ")));
        }

        if let Some(computed) = &var.computed {
            if computed.contains('"') {
                // Use TOML literal string (single quotes) to avoid escaping
                lines.push(format!("computed = '{}'", computed));
            } else {
                lines.push(format!("computed = \"{}\"", computed));
            }
        }

        lines.push(String::new());
    }

    // Files config
    let has_files_config =
        !config.files.exclude.is_empty() || !config.files.copy_without_render.is_empty();

    if has_files_config {
        lines.push("[files]".to_string());

        // Filter out the excludes that were auto-added by the cookiecutter adapter
        let user_excludes: Vec<&String> = config
            .files
            .exclude
            .iter()
            .filter(|e| {
                !matches!(
                    e.as_str(),
                    "cookiecutter.json" | "hooks" | "hooks/**" | ".git" | ".git/**"
                )
            })
            .collect();

        if !user_excludes.is_empty() {
            let vals: Vec<String> = user_excludes.iter().map(|e| format!("\"{}\"", e)).collect();
            lines.push(format!("exclude = [{}]", vals.join(", ")));
        }

        if !config.files.copy_without_render.is_empty() {
            let vals: Vec<String> = config
                .files
                .copy_without_render
                .iter()
                .map(|e| format!("\"{}\"", e))
                .collect();
            lines.push(format!("copy_without_render = [{}]", vals.join(", ")));
        }

        lines.push(String::new());
    }

    lines.join("\n")
}

fn var_type_to_str(vt: &crate::config::variable::VariableType) -> &'static str {
    use crate::config::variable::VariableType;
    match vt {
        VariableType::String => "string",
        VariableType::Bool => "bool",
        VariableType::Int => "int",
        VariableType::Float => "float",
        VariableType::Select => "select",
        VariableType::Multiselect => "multiselect",
    }
}

fn toml_value_to_inline(val: &toml::Value) -> String {
    match val {
        toml::Value::String(s) => format!("\"{}\"", s),
        toml::Value::Integer(n) => n.to_string(),
        toml::Value::Float(f) => f.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(toml_value_to_inline).collect();
            format!("[{}]", items.join(", "))
        }
        other => format!("{}", other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_cc_dirname() {
        assert_eq!(
            rewrite_cc_dirname("{{cookiecutter.project_slug}}"),
            "{{project_slug}}"
        );
        assert_eq!(
            rewrite_cc_dirname("{{ cookiecutter.project_slug }}"),
            "{{project_slug}}"
        );
    }

    #[test]
    fn test_rewrite_cc_path() {
        let path = Path::new("{{cookiecutter.project_slug}}/src/main.py");
        let result = rewrite_cc_path(path);
        assert_eq!(result, Path::new("{{project_slug}}/src/main.py"));
    }

    #[test]
    fn test_find_cookiecutter_dir() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/cookiecutter-basic");
        let dir = find_cookiecutter_dir(&fixture).unwrap();
        assert!(dir
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("cookiecutter.project_slug"));
    }
}
