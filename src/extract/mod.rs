pub mod exclude;
pub mod replace;
pub mod scan;

use std::path::{Path, PathBuf};

use console::style;

use crate::config::schema::DEFAULT_TEMPLATES_SUFFIX;
use crate::error::{DicecutError, Result};

use self::exclude::load_excludes;
use self::replace::{
    apply_path_replacements, apply_replacements, build_replacement_rules, ReplacementRule,
};
use self::scan::scan_project;

/// A variable with its value.
#[derive(Debug, Clone)]
pub struct ExtractVariable {
    pub name: String,
    pub value: String,
}

/// The content of an extracted template file.
#[derive(Debug, Clone)]
pub enum ExtractedContent {
    /// A text file with optional template replacements applied.
    Text {
        content: String,
        replacement_count: usize,
    },
    /// A binary file copied verbatim.
    Binary(Vec<u8>),
}

/// A file that will be part of the extracted template.
#[derive(Debug, Clone)]
pub struct PlannedExtractFile {
    /// Relative path in the output template (may contain template expressions).
    pub template_path: PathBuf,
    /// The file content (text with replacements, or binary bytes).
    pub content: ExtractedContent,
}

impl PlannedExtractFile {
    /// Whether this file had template replacements applied.
    pub fn has_replacements(&self) -> bool {
        matches!(&self.content, ExtractedContent::Text { replacement_count, .. } if *replacement_count > 0)
    }

    /// Whether this is a binary file.
    pub fn is_binary(&self) -> bool {
        matches!(&self.content, ExtractedContent::Binary(_))
    }

    /// Number of replacements made (0 for binary files).
    pub fn replacement_count(&self) -> usize {
        match &self.content {
            ExtractedContent::Text {
                replacement_count, ..
            } => *replacement_count,
            ExtractedContent::Binary(_) => 0,
        }
    }
}

/// The full extraction plan, ready to be executed or reviewed.
#[derive(Debug)]
pub struct ExtractionPlan {
    pub output_dir: PathBuf,
    pub files: Vec<PlannedExtractFile>,
    pub config_toml: String,
    pub variables: Vec<ExtractVariable>,
}

/// Options for the extraction process.
pub struct ExtractOptions {
    pub source_dir: PathBuf,
    pub variables: Vec<(String, String)>,
    pub output_dir: Option<PathBuf>,
    pub in_place: bool,
    pub exclude_file: Option<PathBuf>,
}

/// Plan an extraction: scan the project, build replacement rules, apply replacements.
pub fn plan_extraction(options: &ExtractOptions) -> Result<ExtractionPlan> {
    let source_dir = &options.source_dir;

    if !source_dir.exists() {
        return Err(DicecutError::ExtractSourceNotFound {
            path: source_dir.clone(),
        });
    }

    // Check if this is already a template
    if source_dir.join("diecut.toml").exists() {
        return Err(DicecutError::ExtractAlreadyTemplate {
            path: source_dir.clone(),
        });
    }

    let output_dir = if options.in_place {
        source_dir.clone()
    } else if let Some(ref out) = options.output_dir {
        out.clone()
    } else {
        // Default: source dir name + "-template"
        let dir_name = source_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "template".to_string());
        source_dir
            .parent()
            .unwrap_or(Path::new("."))
            .join(format!("{dir_name}-template"))
    };

    if !options.in_place && output_dir.exists() {
        return Err(DicecutError::ExtractOutputExists {
            path: output_dir.clone(),
        });
    }

    // Scan project
    let scan_excludes = load_excludes(options.exclude_file.as_deref());
    eprintln!(
        "\n{}",
        style(format!("Scanning {}...", source_dir.display())).bold()
    );
    let scan_result = scan_project(source_dir, &scan_excludes)?;

    eprintln!(
        "  {} files found, {} excluded",
        scan_result.files.len(),
        scan_result.excluded_count,
    );

    // Validate that at least one --var was provided
    let variables = options.variables.clone();
    if variables.is_empty() {
        return Err(DicecutError::ExtractNoVariables);
    }

    // Build extract variables (verbatim only)
    let extract_variables: Vec<ExtractVariable> = variables
        .iter()
        .map(|(name, value)| ExtractVariable {
            name: name.clone(),
            value: value.clone(),
        })
        .collect();

    // Build replacement rules — one rule per variable, verbatim only
    let mut rules: Vec<ReplacementRule> = extract_variables
        .iter()
        .map(|var| ReplacementRule {
            literal: var.value.clone(),
            replacement: format!("{{{{ {} }}}}", var.name),
            variable: var.name.clone(),
            variant: "verbatim".to_string(),
        })
        .collect();
    build_replacement_rules(&mut rules);

    // Apply replacements to files
    let mut planned_files = Vec::new();

    for file in &scan_result.files {
        let template_path = apply_path_replacements(&file.relative_path, &rules);

        if file.is_binary {
            let binary_content =
                std::fs::read(&file.absolute_path).map_err(|e| DicecutError::Io {
                    context: format!("reading binary file {}", file.absolute_path.display()),
                    source: e,
                })?;
            planned_files.push(PlannedExtractFile {
                template_path,
                content: ExtractedContent::Binary(binary_content),
            });
        } else if let Some(ref content) = file.content {
            let (replaced, count) = apply_replacements(content, &rules);

            if count > 0 {
                // Has template replacements — add .die suffix
                let mut p = template_path.as_os_str().to_string_lossy().to_string();
                p.push_str(DEFAULT_TEMPLATES_SUFFIX);
                planned_files.push(PlannedExtractFile {
                    template_path: PathBuf::from(p),
                    content: ExtractedContent::Text {
                        content: replaced,
                        replacement_count: count,
                    },
                });
            } else {
                // No replacements — copy verbatim
                planned_files.push(PlannedExtractFile {
                    template_path,
                    content: ExtractedContent::Text {
                        content: replaced,
                        replacement_count: 0,
                    },
                });
            }
        }
    }

    // Generate minimal config TOML inline
    let template_name = source_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "template".to_string());

    let config_toml = generate_minimal_config(&template_name, &extract_variables);

    Ok(ExtractionPlan {
        output_dir,
        files: planned_files,
        config_toml,
        variables: extract_variables,
    })
}

/// Execute an extraction plan: write files and config to the output directory.
pub fn execute_extraction(plan: &ExtractionPlan) -> Result<()> {
    let output_dir = &plan.output_dir;
    let template_dir = output_dir.join("template");

    // Create output structure
    std::fs::create_dir_all(&template_dir).map_err(|e| DicecutError::Io {
        context: format!("creating template directory {}", template_dir.display()),
        source: e,
    })?;

    // Write template files
    let mut rendered_count = 0;
    let mut copied_count = 0;

    for file in &plan.files {
        let dest = template_dir.join(&file.template_path);

        // Ensure parent directory exists
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DicecutError::Io {
                context: format!("creating directory {}", parent.display()),
                source: e,
            })?;
        }

        match &file.content {
            ExtractedContent::Text {
                content,
                replacement_count,
            } => {
                std::fs::write(&dest, content).map_err(|e| DicecutError::Io {
                    context: format!("writing file {}", dest.display()),
                    source: e,
                })?;
                if *replacement_count > 0 {
                    rendered_count += 1;
                } else {
                    copied_count += 1;
                }
            }
            ExtractedContent::Binary(bytes) => {
                std::fs::write(&dest, bytes).map_err(|e| DicecutError::Io {
                    context: format!("writing binary file {}", dest.display()),
                    source: e,
                })?;
                copied_count += 1;
            }
        }
    }

    // Write diecut.toml
    let config_path = output_dir.join("diecut.toml");
    std::fs::write(&config_path, &plan.config_toml).map_err(|e| DicecutError::Io {
        context: format!("writing {}", config_path.display()),
        source: e,
    })?;

    // Summary
    eprintln!(
        "\n{} Template extracted to {}",
        style("✓").green().bold(),
        style(output_dir.display()).cyan()
    );
    eprintln!(
        "  {} variables, {} files templated, {} files copied",
        plan.variables.len(),
        rendered_count,
        copied_count,
    );
    eprintln!("  Review diecut.toml to fine-tune");

    Ok(())
}

fn generate_minimal_config(template_name: &str, variables: &[ExtractVariable]) -> String {
    let escape = |s: &str| toml::Value::String(s.to_string()).to_string();
    let mut out = String::new();

    out.push_str(&format!("[template]\nname = {}\n", escape(template_name)));
    out.push_str("version = \"1.0.0\"\n\n");

    for var in variables {
        out.push_str(&format!("[variables.{}]\n", var.name));
        out.push_str(&format!(
            "type = \"string\"\nprompt = {}\n",
            escape(&var.name.replace(['_', '-'], " "))
        ));
        out.push_str(&format!("default = {}\n\n", escape(&var.value)));
    }

    out.push_str("[files]\n# exclude = []\n# copy_without_render = []\n\n");
    out.push_str("# [hooks]\n# post_create = \"echo 'Project created!'\"\n");
    out
}
