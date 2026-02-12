use std::path::Path;

use crate::adapter::resolve_template;
use crate::check::{check_template, CheckResult};
use crate::error::Result;

/// Result of checking distribution readiness.
pub struct ReadyResult {
    /// The underlying template validation result.
    pub check: CheckResult,
    /// Distribution-specific warnings (missing version, description, etc.).
    pub distribution_warnings: Vec<String>,
}

impl ReadyResult {
    /// True if the template is ready for distribution (no errors, no distribution warnings).
    pub fn is_ready(&self) -> bool {
        self.check.errors.is_empty() && self.distribution_warnings.is_empty()
    }
}

/// Validate a template for distribution readiness.
///
/// Runs all standard `check` validations plus additional checks:
/// - Template version is specified
/// - Template description is specified
/// - Template directory contains a README
pub fn check_ready(template_dir: &Path) -> Result<ReadyResult> {
    let check = check_template(template_dir)?;
    let mut dist_warnings = Vec::new();

    // Re-resolve to access config fields not exposed in CheckResult
    let resolved = resolve_template(template_dir)?;
    let config = &resolved.config;

    if config.template.version.is_none() {
        dist_warnings.push("No 'version' specified in [template] section".to_string());
    }

    if config.template.description.is_none() {
        dist_warnings.push("No 'description' specified in [template] section".to_string());
    }

    // Check for a README file in the template root
    let has_readme = ["README.md", "README.txt", "README"]
        .iter()
        .any(|f| template_dir.join(f).exists());
    if !has_readme {
        dist_warnings.push("No README file found in template root".to_string());
    }

    Ok(ReadyResult {
        check,
        distribution_warnings: dist_warnings,
    })
}
