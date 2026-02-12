mod rhai_runtime;

use std::collections::BTreeMap;
use std::path::Path;

use tera::Value;

use crate::config::schema::HooksConfig;
use crate::error::{DicecutError, Result};

pub use rhai_runtime::create_engine;

/// Run pre-generation hooks.
pub fn run_pre_generate(
    hooks: &HooksConfig,
    template_dir: &Path,
    variables: &BTreeMap<String, Value>,
) -> Result<()> {
    for hook_path in &hooks.pre_generate {
        run_hook(template_dir, hook_path, variables)?;
    }
    Ok(())
}

/// Run post-generation hooks.
pub fn run_post_generate(
    hooks: &HooksConfig,
    template_dir: &Path,
    output_dir: &Path,
    variables: &BTreeMap<String, Value>,
) -> Result<()> {
    for hook_path in &hooks.post_generate {
        run_hook_with_output(template_dir, hook_path, output_dir, variables)?;
    }
    Ok(())
}

fn run_hook(
    template_dir: &Path,
    hook_path: &str,
    variables: &BTreeMap<String, Value>,
) -> Result<()> {
    let full_path = template_dir.join(hook_path);
    let script = std::fs::read_to_string(&full_path).map_err(|e| DicecutError::Io {
        context: format!("reading hook {}", full_path.display()),
        source: e,
    })?;

    let engine = rhai_runtime::create_engine();
    let mut scope = rhai_runtime::build_scope(variables, None);

    engine
        .run_with_scope(&mut scope, &script)
        .map_err(|e| DicecutError::HookError {
            hook: hook_path.to_string(),
            message: e.to_string(),
        })?;

    Ok(())
}

fn run_hook_with_output(
    template_dir: &Path,
    hook_path: &str,
    output_dir: &Path,
    variables: &BTreeMap<String, Value>,
) -> Result<()> {
    let full_path = template_dir.join(hook_path);
    let script = std::fs::read_to_string(&full_path).map_err(|e| DicecutError::Io {
        context: format!("reading hook {}", full_path.display()),
        source: e,
    })?;

    let engine = rhai_runtime::create_engine();
    let mut scope = rhai_runtime::build_scope(variables, Some(output_dir));

    engine
        .run_with_scope(&mut scope, &script)
        .map_err(|e| DicecutError::HookError {
            hook: hook_path.to_string(),
            message: e.to_string(),
        })?;

    Ok(())
}
