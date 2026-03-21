pub mod intersect;
pub mod validate;

use std::path::PathBuf;

use regex_lite::Regex;

use crate::error::{DicecutError, Result};
use crate::extract::config_gen::{
    generate_config_toml, ComputedVariable, ConfigGenOptions, PromptedVariable,
};
use crate::extract::exclude::all_default_excludes;
use crate::extract::replace::{
    apply_path_replacements, apply_replacements, build_replacement_rules, ReplacementRule,
};
use crate::extract::scan::scan_project;
use crate::extract::variants::{
    computed_expression, detect_separator, generate_variants, is_canonical_variant,
};

use self::intersect::intersect_scans;
use self::validate::{is_variable_active, DistillVariable};

// ── Public API types ─────────────────────────────────────────────────────────

pub struct DistillOptions {
    pub projects: Vec<PathBuf>,
    pub variables: Vec<(String, String)>,
    pub output_dir: PathBuf,
    pub max_depth: Option<usize>,
    pub dry_run: bool,
    pub force: bool,
}

pub enum DistilledContent {
    Text {
        content: String,
        replacement_count: usize,
    },
    Binary(Vec<u8>),
    Static(String),
}

pub struct DistilledFile {
    pub template_path: PathBuf,
    pub content: DistilledContent,
}

pub struct DistillPlan {
    pub output_dir: PathBuf,
    pub files: Vec<DistilledFile>,
    pub config_toml: String,
    pub active_variables: Vec<DistillVariable>,
    pub suppressed_variables: Vec<(String, String)>, // (name, reason)
    pub dry_run: bool,
}

// ── Main entry points ─────────────────────────────────────────────────────────

/// Build a distill plan without writing any files.
pub fn plan_distill(options: DistillOptions) -> Result<DistillPlan> {
    // Phase 0: validate inputs
    validate_inputs(&options)?;

    // Phase 1: expand variables into replacement rules
    let (distill_vars, mut rules) = expand_variables(&options.variables);

    // Phase 2: scan all projects
    let excludes = all_default_excludes();
    let mut scans = Vec::with_capacity(options.projects.len());
    for project in &options.projects {
        let scan = scan_project(project, &excludes, options.max_depth)?;
        scans.push(scan);
    }

    // Phase 3: intersect
    let aligned = intersect_scans(&scans);
    if aligned.is_empty() {
        return Err(DicecutError::DistillNoCommonFiles(options.projects.len()));
    }

    // Phase 4: cross-validate — determine which variables are active
    let mut active_variables: Vec<DistillVariable> = Vec::new();
    let mut suppressed_variables: Vec<(String, String)> = Vec::new();

    for var in distill_vars {
        if is_variable_active(&var, &aligned) {
            active_variables.push(var);
        } else {
            let reason = format!(
                "'{}' does not vary across projects or does not appear in shared files",
                var.value_in_p0
            );
            suppressed_variables.push((var.name, reason));
        }
    }

    // Keep only rules for active variables
    rules.retain(|r| active_variables.iter().any(|v| v.name == r.variable));

    // Phase 5: process files
    let files = process_files(&aligned, &rules);

    // Phase 6: generate diecut.toml
    let config_toml = generate_config(&active_variables, &options.output_dir);

    Ok(DistillPlan {
        output_dir: options.output_dir,
        files,
        config_toml,
        active_variables,
        suppressed_variables,
        dry_run: options.dry_run,
    })
}

/// Write the distill plan to disk.
pub fn execute_distill(plan: &DistillPlan) -> Result<()> {
    if plan.dry_run {
        return Ok(());
    }

    let template_dir = plan.output_dir.join("template");
    std::fs::create_dir_all(&template_dir).map_err(|e| DicecutError::Io {
        context: format!("creating output directory {}", template_dir.display()),
        source: e,
    })?;

    // Write diecut.toml
    let config_path = plan.output_dir.join("diecut.toml");
    std::fs::write(&config_path, &plan.config_toml).map_err(|e| DicecutError::Io {
        context: format!("writing config to {}", config_path.display()),
        source: e,
    })?;

    // Write each distilled file
    for distilled in &plan.files {
        let dest = template_dir.join(&distilled.template_path);

        // Create parent directories
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DicecutError::Io {
                context: format!("creating directory {}", parent.display()),
                source: e,
            })?;
        }

        match &distilled.content {
            DistilledContent::Text { content, .. } => {
                std::fs::write(&dest, content).map_err(|e| DicecutError::Io {
                    context: format!("writing file {}", dest.display()),
                    source: e,
                })?;
            }
            DistilledContent::Binary(bytes) => {
                std::fs::write(&dest, bytes).map_err(|e| DicecutError::Io {
                    context: format!("writing binary file {}", dest.display()),
                    source: e,
                })?;
            }
            DistilledContent::Static(content) => {
                std::fs::write(&dest, content).map_err(|e| DicecutError::Io {
                    context: format!("writing static file {}", dest.display()),
                    source: e,
                })?;
            }
        }
    }

    Ok(())
}

// ── Phase implementations ─────────────────────────────────────────────────────

fn validate_inputs(options: &DistillOptions) -> Result<()> {
    // At least 2 projects required
    if options.projects.len() < 2 {
        return Err(DicecutError::DistillMinProjects);
    }

    // All project paths must be directories
    for project in &options.projects {
        if !project.is_dir() {
            return Err(DicecutError::TemplateDirectoryMissing {
                path: project.clone(),
            });
        }
    }

    // Validate variable names and values
    let name_re = Regex::new(r"^[a-z][a-z0-9_]*$").unwrap();
    for (name, value) in &options.variables {
        if !name_re.is_match(name) {
            return Err(DicecutError::DistillInvalidVarName(name.clone()));
        }
        if value.is_empty() {
            return Err(DicecutError::DistillEmptyValue(name.clone()));
        }
        if value.contains('/') {
            return Err(DicecutError::DistillSlashInValue(value.clone()));
        }
    }

    // Output dir must not exist unless --force
    if options.output_dir.exists() && !options.force {
        return Err(DicecutError::DistillOutputExists(
            options.output_dir.display().to_string(),
        ));
    }

    Ok(())
}

/// Expand variables into DistillVariable entries and ReplacementRule entries.
///
/// Returns (distill_vars, sorted_rules).
fn expand_variables(
    variables: &[(String, String)],
) -> (Vec<DistillVariable>, Vec<ReplacementRule>) {
    let mut distill_vars = Vec::new();
    let mut rules: Vec<ReplacementRule> = Vec::new();

    for (name, value) in variables {
        let variants = generate_variants(name, value);

        for variant in &variants {
            rules.push(ReplacementRule {
                literal: variant.literal.clone(),
                replacement: variant.tera_expr.clone(),
                variable: name.clone(),
                variant: variant.name.to_string(),
            });
        }

        distill_vars.push(DistillVariable {
            name: name.clone(),
            value_in_p0: value.clone(),
            variants,
        });
    }

    build_replacement_rules(&mut rules);
    (distill_vars, rules)
}

/// Process all aligned files: apply replacements to project[0]'s content,
/// determine template paths, and classify as Text/Binary/Static.
fn process_files(
    aligned: &[intersect::AlignedFile],
    rules: &[ReplacementRule],
) -> Vec<DistilledFile> {
    let mut result = Vec::new();

    for file in aligned {
        if file.any_binary {
            // Binary: copy raw bytes from project[0]
            let bytes = file
                .raw_bytes
                .first()
                .and_then(|b| b.clone())
                .unwrap_or_default();
            let template_path = apply_path_replacements(&file.relative_path, rules);
            result.push(DistilledFile {
                template_path,
                content: DistilledContent::Binary(bytes),
            });
        } else {
            // Text: apply replacements to project[0]'s content
            let p0_content = file
                .contents
                .first()
                .and_then(|c| c.as_deref())
                .unwrap_or("");

            let (rendered, count) = apply_replacements(p0_content, rules);

            // Warn about existing template expressions
            if count > 0 {
                let brace_count = rendered.matches("{{").count();
                if brace_count > count {
                    eprintln!("  warning: {} contains existing '{{{{}}}}' syntax that may need {{% raw %}} blocks",
                        file.relative_path.display());
                }
            }

            let template_path = if count > 0 {
                // File has replacements: add .die suffix
                let mut path = apply_path_replacements(&file.relative_path, rules);
                let new_name = format!(
                    "{}.die",
                    path.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                );
                path.set_file_name(new_name);
                path
            } else {
                apply_path_replacements(&file.relative_path, rules)
            };

            let content = if count > 0 {
                DistilledContent::Text {
                    content: rendered,
                    replacement_count: count,
                }
            } else {
                DistilledContent::Static(rendered)
            };

            result.push(DistilledFile {
                template_path,
                content,
            });
        }
    }

    result
}

/// Generate diecut.toml content for the active variables.
fn generate_config(active_variables: &[DistillVariable], output_dir: &std::path::Path) -> String {
    let template_name = output_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "my-template".to_string());

    let mut prompted = Vec::new();
    let mut computed = Vec::new();

    for var in active_variables {
        let canonical_sep = detect_separator(&var.value_in_p0);

        for variant in &var.variants {
            if variant.name == "verbatim" || is_canonical_variant(variant.name, canonical_sep) {
                // Canonical variant → prompted variable (no default value, user must supply it)
                if !prompted
                    .iter()
                    .any(|p: &PromptedVariable| p.name == var.name)
                {
                    prompted.push(PromptedVariable {
                        name: var.name.clone(),
                        default_value: String::new(),
                        prompt: format!(
                            "{} (e.g. {})",
                            var.name.replace('_', " "),
                            var.value_in_p0
                        ),
                    });
                }
            } else {
                // Non-canonical variant → computed variable
                let computed_name = format!("{}_{}", var.name, variant.name);
                let expr = computed_expression(&var.name, variant.name, canonical_sep);
                computed.push(ComputedVariable {
                    name: computed_name,
                    expression: expr,
                });
            }
        }
    }

    generate_config_toml(&ConfigGenOptions {
        template_name,
        prompted_variables: prompted,
        computed_variables: computed,
        exclude_patterns: Vec::new(),
        copy_without_render: Vec::new(),
        conditional_entries: Vec::new(),
    })
}
