pub mod adapter;
pub mod answers;
pub mod check;
pub mod config;
pub mod error;
pub mod hooks;
pub mod prompt;
pub mod ready;
pub mod render;
pub mod template;
pub mod update;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use console::style;
use tera::Value;

use crate::adapter::resolve_template;
use crate::answers::SourceInfo;
use crate::error::{DicecutError, Result};
use crate::prompt::{collect_variables, PromptOptions};
use crate::render::{
    build_context_with_namespace, execute_plan, plan_render, GeneratedProject, GenerationPlan,
};
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
        TemplateSource::Git { url, git_ref } => {
            let (path, commit_sha) = get_or_clone(url, git_ref.as_deref())?;
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

    if !options.no_hooks {
        hooks::run_pre_generate(&resolved.config.hooks, &template_dir, &variables)?;
    }

    let context = build_context_with_namespace(&variables, &resolved.context_namespace);

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
        hooks::run_post_generate(
            &plan.config.hooks,
            &plan.template_dir,
            &plan.output_dir,
            &plan.variables,
        )?;
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
