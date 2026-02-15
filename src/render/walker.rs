use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use tera::{Context, Tera, Value};
use walkdir::WalkDir;

use crate::adapter::ResolvedTemplate;
use crate::config::schema::FilesConfig;
use crate::error::{DicecutError, Result};
use crate::render::file::{is_binary_file, render_path_component};

pub struct GeneratedProject {
    pub output_dir: PathBuf,
    pub files_created: Vec<PathBuf>,
    pub files_copied: Vec<PathBuf>,
}

/// A file that would be created during generation.
pub struct PlannedFile {
    /// Path relative to the output directory.
    pub relative_path: PathBuf,
    /// The file content (rendered template or copied binary).
    pub content: Vec<u8>,
    /// Whether this file was copied verbatim (true) or rendered from a template (false).
    pub is_copy: bool,
}

/// The result of planning a generation without writing to disk.
pub struct GenerationPlan {
    pub files: Vec<PlannedFile>,
}

/// Walk the template directory and collect rendered/copied files into memory without writing.
pub fn plan_render(
    resolved: &ResolvedTemplate,
    variables: &BTreeMap<String, Value>,
    context: &Context,
) -> Result<GenerationPlan> {
    let content_dir = &resolved.content_dir;
    if !content_dir.exists() {
        return Err(DicecutError::TemplateDirectoryMissing {
            path: content_dir.clone(),
        });
    }

    let config = &resolved.config;
    let suffix = &config.template.templates_suffix;
    let exclude_set = build_glob_set(&config.files.exclude)?;
    let copy_set = build_glob_set(&config.files.copy_without_render)?;
    let conditional_excludes = evaluate_conditional_files(&config.files, variables)?;

    let mut files = Vec::new();

    for entry in WalkDir::new(content_dir)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let src_path = entry.path();
        let rel_path = src_path
            .strip_prefix(content_dir)
            .expect("entry must be under content_dir");

        let rel_str = rel_path.to_string_lossy();

        if exclude_set.is_match(rel_str.as_ref()) {
            continue;
        }

        let rendered_rel = render_relative_path(rel_path, context, suffix)?;
        let rendered_str = rendered_rel.to_string_lossy();

        if conditional_excludes.is_match(rendered_str.as_ref()) {
            continue;
        }

        if entry.file_type().is_dir() {
            continue;
        }

        let should_copy = copy_set.is_match(rendered_str.as_ref())
            || is_binary_file(src_path)
            || (!suffix.is_empty() && !src_path.to_string_lossy().ends_with(suffix));

        if should_copy {
            let content = std::fs::read(src_path).map_err(|e| DicecutError::Io {
                context: format!("reading {}", src_path.display()),
                source: e,
            })?;
            files.push(PlannedFile {
                relative_path: rendered_rel,
                content,
                is_copy: true,
            });
        } else {
            let content = std::fs::read_to_string(src_path).map_err(|e| DicecutError::Io {
                context: format!("reading {}", src_path.display()),
                source: e,
            })?;

            let mut tera = Tera::default();
            let template_name = rel_str.to_string();
            let parse_result = tera.add_raw_template(&template_name, &content);
            let render_result = parse_result.and_then(|_| tera.render(&template_name, context));

            match render_result {
                Ok(rendered) => {
                    files.push(PlannedFile {
                        relative_path: rendered_rel,
                        content: rendered.into_bytes(),
                        is_copy: false,
                    });
                }
                Err(e) => {
                    return Err(DicecutError::RenderError {
                        file: rel_str.to_string(),
                        source: e,
                    });
                }
            }
        }
    }

    Ok(GenerationPlan { files })
}

/// Write the files from a generation plan to disk.
pub fn execute_plan(plan: &GenerationPlan, output_dir: &Path) -> Result<GeneratedProject> {
    let mut files_created = Vec::new();
    let mut files_copied = Vec::new();

    for file in &plan.files {
        let dest_path = output_dir.join(&file.relative_path);
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DicecutError::Io {
                context: format!("creating directory {}", parent.display()),
                source: e,
            })?;
        }
        std::fs::write(&dest_path, &file.content).map_err(|e| DicecutError::Io {
            context: format!("writing {}", dest_path.display()),
            source: e,
        })?;
        if file.is_copy {
            files_copied.push(file.relative_path.clone());
        } else {
            files_created.push(file.relative_path.clone());
        }
    }

    Ok(GeneratedProject {
        output_dir: output_dir.to_path_buf(),
        files_created,
        files_copied,
    })
}

/// Walk the template directory, render files, and write output.
pub fn walk_and_render(
    resolved: &ResolvedTemplate,
    output_dir: &Path,
    variables: &BTreeMap<String, Value>,
    context: &Context,
) -> Result<GeneratedProject> {
    let plan = plan_render(resolved, variables, context)?;
    execute_plan(&plan, output_dir)
}

/// Render each component of a relative path through Tera, and strip the template suffix.
fn render_relative_path(rel_path: &Path, context: &Context, suffix: &str) -> Result<PathBuf> {
    let mut rendered = PathBuf::new();
    for component in rel_path.components() {
        let part = component.as_os_str().to_string_lossy();
        let mut rendered_part = render_path_component(&part, context)?;

        // Strip template suffix from the final component (filename)
        if !suffix.is_empty() && rendered_part.ends_with(suffix) {
            rendered_part.truncate(rendered_part.len() - suffix.len());
        }

        rendered.push(rendered_part);
    }
    Ok(rendered)
}

fn build_glob_set(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).map_err(|e| DicecutError::GlobPattern {
            pattern: pattern.clone(),
            source: e,
        })?;
        builder.add(glob);
    }
    builder.build().map_err(|e| DicecutError::GlobPattern {
        pattern: "<combined>".into(),
        source: e,
    })
}

/// Evaluate [[files.conditional]] rules and return a GlobSet of files to exclude.
fn evaluate_conditional_files(
    files_config: &FilesConfig,
    variables: &BTreeMap<String, Value>,
) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();

    for cond in &files_config.conditional {
        let should_include = evaluate_when_expr(&cond.when, variables)?;
        if !should_include {
            // Condition is false → exclude files matching this pattern
            let glob = Glob::new(&cond.pattern).map_err(|e| DicecutError::GlobPattern {
                pattern: cond.pattern.clone(),
                source: e,
            })?;
            builder.add(glob);
        }
    }

    builder.build().map_err(|e| DicecutError::GlobPattern {
        pattern: "<conditional>".into(),
        source: e,
    })
}

fn evaluate_when_expr(when_expr: &str, variables: &BTreeMap<String, Value>) -> Result<bool> {
    let mut tera = Tera::default();
    let template_str = format!("{{% if {when_expr} %}}true{{% else %}}false{{% endif %}}");
    tera.add_raw_template("__when__", &template_str)
        .map_err(|e| DicecutError::RenderError {
            file: format!("(when expression: {when_expr})"),
            source: e,
        })?;

    let mut context = Context::new();
    for (k, v) in variables {
        context.insert(k, v);
    }

    let result = tera
        .render("__when__", &context)
        .map_err(|e| DicecutError::RenderError {
            file: format!("(when expression: {when_expr})"),
            source: e,
        })?;

    Ok(result.trim() == "true")
}
