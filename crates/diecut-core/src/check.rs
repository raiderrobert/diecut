use std::path::Path;

use crate::adapter::{resolve_template, TemplateFormat};
use crate::error::Result;

/// Result of validating a template.
pub struct CheckResult {
    pub format: TemplateFormat,
    pub template_name: String,
    pub variable_count: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// Validate a template directory.
pub fn check_template(template_dir: &Path) -> Result<CheckResult> {
    let resolved = resolve_template(template_dir)?;
    let config = &resolved.config;

    let mut warnings = resolved.warnings.clone();
    let mut errors = Vec::new();

    // Validate variables
    if let Err(e) = config.validate() {
        errors.push(format!("Config validation: {e}"));
    }

    // Check template directory exists
    if !resolved.content_dir.exists() {
        errors.push(format!(
            "Template content directory not found: {}",
            resolved.content_dir.display()
        ));
    }

    // Check hooks reference valid files
    for hook in &config.hooks.pre_generate {
        let hook_path = template_dir.join(hook);
        if !hook_path.exists() {
            errors.push(format!("Pre-generate hook not found: {hook}"));
        }
    }
    for hook in &config.hooks.post_generate {
        let hook_path = template_dir.join(hook);
        if !hook_path.exists() {
            errors.push(format!("Post-generate hook not found: {hook}"));
        }
    }

    // Validate Tera syntax in template files
    let suffix = &config.template.templates_suffix;
    if resolved.content_dir.exists() {
        validate_tera_files(&resolved.content_dir, suffix, &mut warnings, &mut errors);
    }

    // Check conditional expressions are parseable
    for cond in &config.files.conditional {
        if let Err(e) = validate_tera_expression(&cond.when) {
            errors.push(format!(
                "Invalid conditional expression for pattern '{}': {e}",
                cond.pattern
            ));
        }
    }

    // Check 'when' expressions on variables
    for (name, var) in &config.variables {
        if let Some(when) = &var.when {
            if let Err(e) = validate_tera_expression(when) {
                errors.push(format!(
                    "Invalid 'when' expression for variable '{name}': {e}"
                ));
            }
        }
        if let Some(computed) = &var.computed {
            if let Err(e) = validate_tera_template(computed) {
                errors.push(format!(
                    "Invalid 'computed' expression for variable '{name}': {e}"
                ));
            }
        }
    }

    Ok(CheckResult {
        format: resolved.format,
        template_name: config.template.name.clone(),
        variable_count: config.variables.len(),
        warnings,
        errors,
    })
}

fn validate_tera_files(
    dir: &Path,
    suffix: &str,
    warnings: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    let walker = walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok());
    for entry in walker {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let path_str = path.to_string_lossy();

        // Only check files with the template suffix
        if !suffix.is_empty() && !path_str.ends_with(suffix) {
            continue;
        }

        // Skip binary files
        if crate::render::file::is_binary_file(path) {
            continue;
        }

        match std::fs::read_to_string(path) {
            Ok(content) => {
                let mut tera = tera::Tera::default();
                let template_name = path.strip_prefix(dir).unwrap_or(path).to_string_lossy();
                if let Err(e) = tera.add_raw_template(&template_name, &content) {
                    let rel = path.strip_prefix(dir).unwrap_or(path).display();
                    errors.push(format!("Tera syntax error in {rel}: {e}"));
                }
            }
            Err(e) => {
                let rel = path.strip_prefix(dir).unwrap_or(path).display();
                warnings.push(format!("Could not read {rel}: {e}"));
            }
        }
    }
}

fn validate_tera_expression(expr: &str) -> std::result::Result<(), String> {
    let template = format!("{{% if {expr} %}}ok{{% endif %}}");
    let mut tera = tera::Tera::default();
    tera.add_raw_template("__check__", &template)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn validate_tera_template(expr: &str) -> std::result::Result<(), String> {
    let mut tera = tera::Tera::default();
    tera.add_raw_template("__check__", expr)
        .map_err(|e| e.to_string())?;
    Ok(())
}
