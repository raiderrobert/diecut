use std::path::{Path, PathBuf};

use crate::error::{DicecutError, Result};
use crate::update::diff;

/// The outcome of merging a single file.
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

/// Result of a three-way merge for a single file.
#[derive(Debug)]
pub struct FileMergeResult {
    pub rel_path: PathBuf,
    pub action: MergeAction,
}

/// Perform a three-way merge comparison for all files across old snapshot,
/// new snapshot, and the user's project.
///
/// - `project_dir`: the user's project (current state)
/// - `old_snapshot_dir`: template rendered with old answers at old ref
/// - `new_snapshot_dir`: template rendered with old answers at new ref
pub fn three_way_merge(
    project_dir: &Path,
    old_snapshot_dir: &Path,
    new_snapshot_dir: &Path,
) -> Result<Vec<FileMergeResult>> {
    let project_files = diff::collect_files(project_dir)?;
    let old_files = diff::collect_files(old_snapshot_dir)?;
    let new_files = diff::collect_files(new_snapshot_dir)?;

    // Union of all file paths
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
            // File exists in all three — compare contents
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

            // File only in new snapshot — added by template update
            (false, true, false) => MergeAction::AddFromTemplate,

            // File in old and project but not new — template removed it
            (true, false, true) => {
                let old_path = old_snapshot_dir.join(&rel_path);
                let proj_path = project_dir.join(&rel_path);
                let user_changed = !diff::files_equal(&proj_path, &old_path)?;
                if user_changed {
                    // User modified a file the template wants to remove — conflict
                    MergeAction::Conflict
                } else {
                    MergeAction::MarkForRemoval
                }
            }

            // File only in project — user added it, keep it
            (false, false, true) => MergeAction::KeepUser,

            // File in new and project but not old — both added independently
            (false, true, true) => {
                let new_path = new_snapshot_dir.join(&rel_path);
                let proj_path = project_dir.join(&rel_path);
                if diff::files_equal(&proj_path, &new_path)? {
                    MergeAction::Unchanged
                } else {
                    MergeAction::Conflict
                }
            }

            // File only in old snapshot — was in template, removed by both user and template
            (true, false, false) => MergeAction::Unchanged,

            // File in old and new but not project — user deleted it
            (true, true, false) => {
                let old_path = old_snapshot_dir.join(&rel_path);
                let new_path = new_snapshot_dir.join(&rel_path);
                if diff::files_equal(&old_path, &new_path)? {
                    // Template didn't change it, user deleted — respect user's deletion
                    MergeAction::KeepUser
                } else {
                    // Template changed it, user deleted — conflict
                    MergeAction::Conflict
                }
            }

            // Shouldn't happen (would need the file in none of the three sets)
            (false, false, false) => MergeAction::Unchanged,
        };

        // Skip unchanged files from the report
        if action != MergeAction::Unchanged {
            results.push(FileMergeResult { rel_path, action });
        }
    }

    Ok(results)
}

/// Apply the merge results to the user's project directory.
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
                // Write a .rej file with the diff
                let new_path = new_snapshot_dir.join(&result.rel_path);

                let user_content = std::fs::read_to_string(&proj_path).unwrap_or_default();
                let new_content = std::fs::read_to_string(&new_path).unwrap_or_default();

                // If the file existed in old snapshot, also include context
                let old_path = old_snapshot_dir.join(&result.rel_path);
                let rej_content = if old_path.exists() {
                    let old_content = std::fs::read_to_string(&old_path).unwrap_or_default();
                    format!(
                        "# Conflict in {}\n\
                         # Your file differs from both the old and new template versions.\n\n\
                         ## Template changes (old -> new):\n{}\n\n\
                         ## Your version vs new template:\n{}\n",
                        result.rel_path.display(),
                        diff::unified_diff(&old_content, &new_content, &result.rel_path),
                        diff::unified_diff(&user_content, &new_content, &result.rel_path),
                    )
                } else {
                    format!(
                        "# Conflict in {}\n\
                         # Both you and the template created/modified this file differently.\n\n\
                         ## Diff (your version vs template):\n{}\n",
                        result.rel_path.display(),
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
