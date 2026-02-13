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

/// Walk the template directory, render files, and write output.
pub fn walk_and_render(
    resolved: &ResolvedTemplate,
    output_dir: &Path,
    variables: &BTreeMap<String, Value>,
    context: &Context,
) -> Result<GeneratedProject> {
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

    let mut files_created = Vec::new();
    let mut files_copied = Vec::new();

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

        let dest_path = output_dir.join(&rendered_rel);

        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&dest_path).map_err(|e| DicecutError::Io {
                context: format!("creating directory {}", dest_path.display()),
                source: e,
            })?;
            continue;
        }

        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DicecutError::Io {
                context: format!("creating directory {}", parent.display()),
                source: e,
            })?;
        }

        let should_copy = copy_set.is_match(rendered_str.as_ref())
            || is_binary_file(src_path)
            || (!resolved.render_all
                && !suffix.is_empty()
                && !src_path.to_string_lossy().ends_with(suffix));

        if should_copy {
            std::fs::copy(src_path, &dest_path).map_err(|e| DicecutError::Io {
                context: format!("copying {} to {}", src_path.display(), dest_path.display()),
                source: e,
            })?;
            files_copied.push(rendered_rel);
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
                    std::fs::write(&dest_path, rendered).map_err(|e| DicecutError::Io {
                        context: format!("writing {}", dest_path.display()),
                        source: e,
                    })?;
                    files_created.push(rendered_rel);
                }
                Err(e) if resolved.render_all => {
                    // Foreign templates may contain unsupported syntax (e.g. Jinja2
                    // extensions). Fall back to copying verbatim with a warning.
                    eprintln!(
                        "warning: failed to render {}, copying verbatim: {}",
                        rel_str, e
                    );
                    std::fs::write(&dest_path, &content).map_err(|e| DicecutError::Io {
                        context: format!("writing {}", dest_path.display()),
                        source: e,
                    })?;
                    files_copied.push(rendered_rel);
                }
                Err(e) => {
                    return Err(DicecutError::RenderError { source: e });
                }
            }
        }
    }

    Ok(GeneratedProject {
        output_dir: output_dir.to_path_buf(),
        files_created,
        files_copied,
    })
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
        .map_err(|e| DicecutError::RenderError { source: e })?;

    let mut context = Context::new();
    for (k, v) in variables {
        context.insert(k, v);
    }

    let result = tera
        .render("__when__", &context)
        .map_err(|e| DicecutError::RenderError { source: e })?;

    Ok(result.trim() == "true")
}
