use std::path::Path;

use crate::adapter::resolve_template;
use crate::check::{check_template, CheckResult};
use crate::error::Result;

pub struct ReadyResult {
    pub check: CheckResult,
    /// E.g. missing version, description, README.
    pub distribution_warnings: Vec<String>,
}

impl ReadyResult {
    pub fn is_ready(&self) -> bool {
        self.check.errors.is_empty() && self.distribution_warnings.is_empty()
    }
}

/// Runs `check` validations plus distribution checks (version, description, README).
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
