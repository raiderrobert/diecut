use std::path::{Path, PathBuf};

use crate::error::{DicecutError, Result};
use crate::update::diff;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeAction {
    /// File unchanged by user, take the new template version.
    UpdateFromTemplate,
    /// New file added in the template.
    AddFromTemplate,
    /// File removed in the updated template (user hasn't changed it).
    MarkForRemoval,
    /// User changed the file, template did not — keep user's version.
    KeepUser,
    /// Both user and template changed — conflict.
    Conflict,
    /// File unchanged by either side.
    Unchanged,
}

#[derive(Debug)]
pub struct FileMergeResult {
    pub rel_path: PathBuf,
    pub action: MergeAction,
}

pub fn three_way_merge(
    project_dir: &Path,
    old_snapshot_dir: &Path,
    new_snapshot_dir: &Path,
) -> Result<Vec<FileMergeResult>> {
    let project_files = diff::collect_files(project_dir)?;
    let old_files = diff::collect_files(old_snapshot_dir)?;
    let new_files = diff::collect_files(new_snapshot_dir)?;

    let mut all_paths: Vec<PathBuf> = project_files
        .union(&old_files)
        .cloned()
        .collect::<std::collections::HashSet<_>>()
        .union(&new_files)
        .cloned()
        .collect();
    all_paths.sort();

    let mut results = Vec::new();

    for rel_path in all_paths {
        let in_project = project_files.contains(&rel_path);
        let in_old = old_files.contains(&rel_path);
        let in_new = new_files.contains(&rel_path);

        let action = match (in_old, in_new, in_project) {
            (true, true, true) => {
                let old_path = old_snapshot_dir.join(&rel_path);
                let new_path = new_snapshot_dir.join(&rel_path);
                let proj_path = project_dir.join(&rel_path);

                let user_changed = !diff::files_equal(&proj_path, &old_path)?;
                let template_changed = !diff::files_equal(&old_path, &new_path)?;

                match (user_changed, template_changed) {
                    (false, false) => MergeAction::Unchanged,
                    (false, true) => MergeAction::UpdateFromTemplate,
                    (true, false) => MergeAction::KeepUser,
                    (true, true) => {
                        // Both changed — check if they converged to the same content
                        if diff::files_equal(&proj_path, &new_path)? {
                            MergeAction::Unchanged
                        } else {
                            MergeAction::Conflict
                        }
                    }
                }
            }

            (false, true, false) => MergeAction::AddFromTemplate,

            (true, false, true) => {
                let old_path = old_snapshot_dir.join(&rel_path);
                let proj_path = project_dir.join(&rel_path);
                let user_changed = !diff::files_equal(&proj_path, &old_path)?;
                if user_changed {
                    MergeAction::Conflict
                } else {
                    MergeAction::MarkForRemoval
                }
            }

            (false, false, true) => MergeAction::KeepUser,

            (false, true, true) => {
                let new_path = new_snapshot_dir.join(&rel_path);
                let proj_path = project_dir.join(&rel_path);
                if diff::files_equal(&proj_path, &new_path)? {
                    MergeAction::Unchanged
                } else {
                    MergeAction::Conflict
                }
            }

            (true, false, false) => MergeAction::Unchanged,

            (true, true, false) => {
                let old_path = old_snapshot_dir.join(&rel_path);
                let new_path = new_snapshot_dir.join(&rel_path);
                if diff::files_equal(&old_path, &new_path)? {
                    MergeAction::KeepUser
                } else {
                    MergeAction::Conflict
                }
            }

            (false, false, false) => MergeAction::Unchanged,
        };

        if action != MergeAction::Unchanged {
            results.push(FileMergeResult { rel_path, action });
        }
    }

    Ok(results)
}

pub fn apply_merge(
    project_dir: &Path,
    new_snapshot_dir: &Path,
    old_snapshot_dir: &Path,
    results: &[FileMergeResult],
) -> Result<()> {
    for result in results {
        let proj_path = project_dir.join(&result.rel_path);

        match &result.action {
            MergeAction::UpdateFromTemplate | MergeAction::AddFromTemplate => {
                let new_path = new_snapshot_dir.join(&result.rel_path);
                if let Some(parent) = proj_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| DicecutError::Io {
                        context: format!("creating directory {}", parent.display()),
                        source: e,
                    })?;
                }
                std::fs::copy(&new_path, &proj_path).map_err(|e| DicecutError::Io {
                    context: format!("copying {} to {}", new_path.display(), proj_path.display()),
                    source: e,
                })?;
            }

            MergeAction::MarkForRemoval => {
                // Don't auto-delete; write a .rej file noting the removal
                let rej_path = proj_path.with_extension(
                    proj_path
                        .extension()
                        .map(|e| format!("{}.removing", e.to_string_lossy()))
                        .unwrap_or_else(|| "removing".to_string()),
                );
                let msg = "This file was removed in the updated template.\n\
                     Review and delete it manually if no longer needed.\n";
                std::fs::write(&rej_path, msg).map_err(|e| DicecutError::Io {
                    context: format!("writing removal marker {}", rej_path.display()),
                    source: e,
                })?;
            }

            MergeAction::Conflict => {
                // Write a .rej file with diff3-style output showing all three versions
                let new_path = new_snapshot_dir.join(&result.rel_path);

                let user_content = std::fs::read_to_string(&proj_path).unwrap_or_default();
                let new_content = std::fs::read_to_string(&new_path).unwrap_or_default();

                let old_path = old_snapshot_dir.join(&result.rel_path);
                let rej_content = if old_path.exists() {
                    let old_content = std::fs::read_to_string(&old_path).unwrap_or_default();
                    format!(
                        "# Conflict in {path}\n\
                         # Three-way diff: base (old template) vs yours vs new template\n\n\
                         ## Base version (old template):\n{base}\n\n\
                         ## Your version:\n{user}\n\n\
                         ## New template version:\n{new}\n\n\
                         ## Diff: base -> new template:\n{diff_old_new}\n\n\
                         ## Diff: your version -> new template:\n{diff_user_new}\n",
                        path = result.rel_path.display(),
                        base = old_content,
                        user = user_content,
                        new = new_content,
                        diff_old_new =
                            diff::unified_diff(&old_content, &new_content, &result.rel_path),
                        diff_user_new =
                            diff::unified_diff(&user_content, &new_content, &result.rel_path),
                    )
                } else {
                    format!(
                        "# Conflict in {}\n\
                         # Both you and the template created/modified this file differently.\n\n\
                         ## Your version:\n{}\n\n\
                         ## New template version:\n{}\n\n\
                         ## Diff (yours -> template):\n{}\n",
                        result.rel_path.display(),
                        user_content,
                        new_content,
                        diff::unified_diff(&user_content, &new_content, &result.rel_path),
                    )
                };

                let rej_path = proj_path.with_extension(
                    proj_path
                        .extension()
                        .map(|e| format!("{}.rej", e.to_string_lossy()))
                        .unwrap_or_else(|| "rej".to_string()),
                );
                std::fs::write(&rej_path, rej_content).map_err(|e| DicecutError::Io {
                    context: format!("writing conflict file {}", rej_path.display()),
                    source: e,
                })?;
            }

            MergeAction::KeepUser | MergeAction::Unchanged => {
                // Nothing to do
            }
        }
    }

    Ok(())
}
