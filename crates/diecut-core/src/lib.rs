pub mod adapter;
pub mod answers;
pub mod check;
pub mod config;
pub mod error;
pub mod hooks;
pub mod prompt;
pub mod render;
pub mod template;

use std::path::Path;

use console::style;

use crate::adapter::resolve_template;
use crate::error::{DicecutError, Result};
use crate::prompt::{collect_variables, PromptOptions};
use crate::render::{build_context_with_namespace, walk_and_render, GeneratedProject};
use crate::template::{get_or_clone, resolve_source, TemplateSource};

/// Options for the `generate` operation.
pub struct GenerateOptions {
    /// The template source (path, URL, or abbreviation).
    pub template: String,
    /// Output directory. If None, uses the current directory.
    pub output: Option<String>,
    /// Pre-supplied key=value pairs.
    pub data: Vec<(String, String)>,
    /// Use default values without prompting.
    pub defaults: bool,
    /// Overwrite output directory if it exists.
    pub overwrite: bool,
}

/// Main entry point: generate a project from a template.
pub fn generate(options: GenerateOptions) -> Result<GeneratedProject> {
    // 1. Resolve source
    let source = resolve_source(&options.template)?;
    let template_dir = match &source {
        TemplateSource::Local(path) => path.clone(),
        TemplateSource::Git { url, git_ref } => {
            get_or_clone(url, git_ref.as_deref())?
        }
    };

    // 2. Resolve template (auto-detect format, parse config)
    let resolved = resolve_template(&template_dir)?;

    // Print any adapter warnings
    for warning in &resolved.warnings {
        eprintln!(
            "{} {}",
            style("warning:").yellow().bold(),
            style(warning).yellow()
        );
    }

    // 3. Determine output directory
    let output_dir = if let Some(out) = &options.output {
        Path::new(out).to_path_buf()
    } else {
        std::env::current_dir().map_err(|e| DicecutError::Io {
            context: "getting current directory".into(),
            source: e,
        })?
    };

    // Check overwrite
    if output_dir.exists() && !options.overwrite {
        // Check if it has contents (an empty dir is fine)
        let has_contents = std::fs::read_dir(&output_dir)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false);
        if has_contents {
            return Err(DicecutError::OutputExists { path: output_dir });
        }
    }

    // 4. Collect variables
    let prompt_options = PromptOptions {
        data_overrides: options.data.into_iter().collect(),
        use_defaults: options.defaults,
    };
    let variables = collect_variables(&resolved.config, &prompt_options)?;

    // 5. Build Tera context (with optional namespace for foreign formats)
    let context = build_context_with_namespace(&variables, &resolved.context_namespace);

    // 6. Create output directory
    std::fs::create_dir_all(&output_dir).map_err(|e| DicecutError::Io {
        context: format!("creating output directory {}", output_dir.display()),
        source: e,
    })?;

    // 7. Walk and render
    let result = walk_and_render(&resolved, &output_dir, &variables, &context)?;

    // 8. Write answers file
    answers::write_answers(&output_dir, &resolved.config, &variables)?;

    // 9. Print summary
    println!(
        "\n{} Project generated at {}",
        style("✓").green().bold(),
        style(output_dir.display()).cyan()
    );
    println!(
        "  {} files rendered, {} files copied",
        result.files_created.len(),
        result.files_copied.len()
    );

    Ok(result)
}
