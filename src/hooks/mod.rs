use std::path::Path;

use crate::config::schema::HooksConfig;
use crate::error::{DicecutError, Result};

pub fn run_post_create(hooks: &HooksConfig, output_dir: &Path) -> Result<()> {
    if let Some(cmd) = &hooks.post_create {
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(output_dir)
            .status()
            .map_err(|e| DicecutError::HookError {
                hook: "post_create".to_string(),
                message: format!("failed to execute: {e}"),
            })?;

        if !status.success() {
            return Err(DicecutError::HookError {
                hook: "post_create".to_string(),
                message: format!("exited with status {status}"),
            });
        }
    }
    Ok(())
}
