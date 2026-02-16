pub mod adapter;
pub mod answers;
pub mod config;
pub mod error;
pub mod hooks;
pub mod prompt;
pub mod render;
pub mod template;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use console::style;
use tera::Value;

use crate::adapter::resolve_template;
use crate::answers::SourceInfo;
use crate::error::{DicecutError, Result};
use crate::prompt::{collect_variables, PromptOptions};
use crate::render::{build_context, execute_plan, plan_render, GeneratedProject, GenerationPlan};
use crate::template::{get_or_clone, resolve_source, TemplateSource};

pub struct GenerateOptions {
    pub template: String,
    pub output: Option<String>,
    pub data: Vec<(String, String)>,
    pub defaults: bool,
    pub overwrite: bool,
    pub no_hooks: bool,
}

/// Everything needed to execute a generation that has been planned but not yet written.
pub struct FullGenerationPlan {
    pub render_plan: GenerationPlan,
    pub output_dir: PathBuf,
    pub config: crate::config::schema::TemplateConfig,
    pub variables: BTreeMap<String, Value>,
    pub source_info: SourceInfo,
    pub template_dir: PathBuf,
    pub no_hooks: bool,
}

/// Plan a project generation: resolve template, collect variables, render in memory.
///
/// This performs all preparation (template resolution, variable collection, pre-generate
/// hooks, and rendering) but does **not** write any files to disk.
pub fn plan_generation(options: GenerateOptions) -> Result<FullGenerationPlan> {
    let source = resolve_source(&options.template)?;
    let (template_dir, source_info) = match &source {
        TemplateSource::Local(path) => (
            path.clone(),
            SourceInfo {
                url: None,
                git_ref: None,
                commit_sha: None,
            },
        ),
        TemplateSource::Git {
            url,
            git_ref,
            subpath,
        } => {
            let (path, commit_sha) = get_or_clone(url, git_ref.as_deref())?;
            let path = match subpath {
                Some(sub) => {
                    let joined = path.join(sub);
                    if !joined.exists() {
                        return Err(DicecutError::TemplateDirectoryMissing { path: joined });
                    }
                    joined
                }
                None => path,
            };
            (
                path,
                SourceInfo {
                    url: Some(url.clone()),
                    git_ref: git_ref.clone(),
                    commit_sha,
                },
            )
        }
    };

    let resolved = resolve_template(&template_dir)?;

    for warning in &resolved.warnings {
        eprintln!(
            "{} {}",
            style("warning:").yellow().bold(),
            style(warning).yellow()
        );
    }

    if !options.no_hooks && source_info.url.is_some() && resolved.config.hooks.has_hooks() {
        eprintln!(
            "{} This template contains hooks that will execute code on your machine",
            style("warning:").yellow().bold()
        );
        eprintln!(
            "  source: {}",
            source_info.url.as_deref().unwrap_or("unknown")
        );
        eprintln!("  use --no-hooks to skip hook execution");
    }

    let output_dir = if let Some(out) = &options.output {
        Path::new(out).to_path_buf()
    } else {
        std::env::current_dir().map_err(|e| DicecutError::Io {
            context: "getting current directory".into(),
            source: e,
        })?
    };

    if output_dir.exists() && !options.overwrite {
        // An empty dir is fine
        let has_contents = std::fs::read_dir(&output_dir)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false);
        if has_contents {
            return Err(DicecutError::OutputExists { path: output_dir });
        }
    }

    let prompt_options = PromptOptions {
        data_overrides: options.data.into_iter().collect(),
        use_defaults: options.defaults,
    };
    let variables = collect_variables(&resolved.config, &prompt_options)?;

    let context = build_context(&variables);

    let render_plan = plan_render(&resolved, &variables, &context)?;

    Ok(FullGenerationPlan {
        render_plan,
        output_dir,
        config: resolved.config,
        variables,
        source_info,
        template_dir,
        no_hooks: options.no_hooks,
    })
}

/// Execute a previously planned generation: write files, answers, and run post-generate hooks.
pub fn execute_generation(plan: FullGenerationPlan) -> Result<GeneratedProject> {
    std::fs::create_dir_all(&plan.output_dir).map_err(|e| DicecutError::Io {
        context: format!("creating output directory {}", plan.output_dir.display()),
        source: e,
    })?;

    let result = execute_plan(&plan.render_plan, &plan.output_dir)?;

    answers::write_answers(
        &plan.output_dir,
        &plan.config,
        &plan.variables,
        &plan.source_info,
    )?;

    if !plan.no_hooks {
        hooks::run_post_create(&plan.config.hooks, &plan.output_dir)?;
    }

    println!(
        "\n{} Project generated at {}",
        style("✓").green().bold(),
        style(plan.output_dir.display()).cyan()
    );
    println!(
        "  {} files rendered, {} files copied",
        result.files_created.len(),
        result.files_copied.len()
    );

    Ok(result)
}

/// Generate a project from a template.
pub fn generate(options: GenerateOptions) -> Result<GeneratedProject> {
    let plan = plan_generation(options)?;
    execute_generation(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile;

    fn create_minimal_template(dir: &std::path::Path) {
        let config = r#"
[template]
name = "test-template"
version = "1.0.0"
templates_suffix = ".tera"

[variables.project_name]
type = "string"
default = "my-project"
"#;
        fs::write(dir.join("diecut.toml"), config).unwrap();
        fs::create_dir_all(dir.join("template")).unwrap();
        fs::write(dir.join("template/README.md.tera"), "# {{ project_name }}").unwrap();
    }

    #[test]
    fn test_plan_generation_local_template() {
        let template_dir = tempfile::tempdir().unwrap();
        create_minimal_template(template_dir.path());

        let output_dir = tempfile::tempdir().unwrap();

        let options = GenerateOptions {
            template: template_dir.path().display().to_string(),
            output: Some(output_dir.path().display().to_string()),
            data: vec![("project_name".to_string(), "test-proj".to_string())],
            defaults: false,
            overwrite: false,
            no_hooks: true,
        };

        let plan = plan_generation(options).unwrap();

        assert_eq!(plan.config.template.name, "test-template");
        assert!(plan.render_plan.files.len() > 0);
        assert_eq!(plan.variables.get("project_name").unwrap(), "test-proj");
    }

    #[test]
    fn test_plan_generation_template_missing() {
        let options = GenerateOptions {
            template: "/nonexistent/path/to/template".to_string(),
            output: None,
            data: vec![],
            defaults: true,
            overwrite: false,
            no_hooks: true,
        };

        let result = plan_generation(options);

        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, DicecutError::ConfigNotFound { .. }));
        }
    }

    #[test]
    fn test_plan_generation_output_exists_no_overwrite() {
        let template_dir = tempfile::tempdir().unwrap();
        create_minimal_template(template_dir.path());

        let output_dir = tempfile::tempdir().unwrap();
        fs::write(output_dir.path().join("existing.txt"), "exists").unwrap();

        let options = GenerateOptions {
            template: template_dir.path().display().to_string(),
            output: Some(output_dir.path().display().to_string()),
            data: vec![],
            defaults: true,
            overwrite: false,
            no_hooks: true,
        };

        let result = plan_generation(options);

        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, DicecutError::OutputExists { .. }));
        }
    }

    #[test]
    fn test_plan_generation_output_exists_with_overwrite() {
        let template_dir = tempfile::tempdir().unwrap();
        create_minimal_template(template_dir.path());

        let output_dir = tempfile::tempdir().unwrap();
        fs::write(output_dir.path().join("existing.txt"), "exists").unwrap();

        let options = GenerateOptions {
            template: template_dir.path().display().to_string(),
            output: Some(output_dir.path().display().to_string()),
            data: vec![],
            defaults: true,
            overwrite: true,
            no_hooks: true,
        };

        let plan = plan_generation(options);

        assert!(plan.is_ok());
    }

    #[test]
    fn test_execute_generation_creates_output_dir() {
        let template_dir = tempfile::tempdir().unwrap();
        create_minimal_template(template_dir.path());

        let output_parent = tempfile::tempdir().unwrap();
        let output_path = output_parent.path().join("new_project");

        let options = GenerateOptions {
            template: template_dir.path().display().to_string(),
            output: Some(output_path.display().to_string()),
            data: vec![("project_name".to_string(), "test".to_string())],
            defaults: false,
            overwrite: false,
            no_hooks: true,
        };

        let plan = plan_generation(options).unwrap();

        assert!(
            !output_path.exists(),
            "Output dir should not exist before execution"
        );

        let result = execute_generation(plan);

        assert!(result.is_ok());
        assert!(
            output_path.exists(),
            "Output dir should exist after execution"
        );
    }

    #[test]
    fn test_execute_generation_writes_answers() {
        let template_dir = tempfile::tempdir().unwrap();
        create_minimal_template(template_dir.path());

        let output_dir = tempfile::tempdir().unwrap();

        let options = GenerateOptions {
            template: template_dir.path().display().to_string(),
            output: Some(output_dir.path().display().to_string()),
            data: vec![("project_name".to_string(), "test-project".to_string())],
            defaults: false,
            overwrite: true,
            no_hooks: true,
        };

        let plan = plan_generation(options).unwrap();
        execute_generation(plan).unwrap();

        let answers_file = output_dir.path().join(".diecut-answers.toml");
        assert!(answers_file.exists(), "Answers file should exist");

        let contents = fs::read_to_string(&answers_file).unwrap();
        assert!(contents.contains("project_name"));
        assert!(contents.contains("test-project"));
    }
}
