pub mod diff;
pub mod merge;

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::adapter::resolve_template;
use crate::answers::{load_answers, toml_value_to_tera, write_answers_with_source};
use crate::error::{DicecutError, Result};
use crate::render::{build_context_with_namespace, walk_and_render};
use crate::template::{get_or_clone, resolve_source, TemplateSource};

use merge::{three_way_merge, FileMergeResult, MergeAction};

/// Options for the `update_project` operation.
pub struct UpdateOptions {
    /// Override the template source (if None, read from answers file).
    pub template_source: Option<String>,
    /// Git ref to update to (branch, tag, or commit).
    pub git_ref: Option<String>,
}

/// Report of what happened during an update.
pub struct UpdateReport {
    pub files_updated: Vec<PathBuf>,
    pub files_added: Vec<PathBuf>,
    pub files_removed: Vec<PathBuf>,
    pub conflicts: Vec<PathBuf>,
    pub files_kept: Vec<PathBuf>,
}

impl UpdateReport {
    fn from_results(results: &[FileMergeResult]) -> Self {
        let mut report = UpdateReport {
            files_updated: Vec::new(),
            files_added: Vec::new(),
            files_removed: Vec::new(),
            conflicts: Vec::new(),
            files_kept: Vec::new(),
        };

        for result in results {
            match &result.action {
                MergeAction::UpdateFromTemplate => {
                    report.files_updated.push(result.rel_path.clone());
                }
                MergeAction::AddFromTemplate => {
                    report.files_added.push(result.rel_path.clone());
                }
                MergeAction::MarkForRemoval => {
                    report.files_removed.push(result.rel_path.clone());
                }
                MergeAction::Conflict => {
                    report.conflicts.push(result.rel_path.clone());
                }
                MergeAction::KeepUser => {
                    report.files_kept.push(result.rel_path.clone());
                }
                MergeAction::Unchanged => {}
            }
        }

        report
    }

    pub fn has_changes(&self) -> bool {
        !self.files_updated.is_empty()
            || !self.files_added.is_empty()
            || !self.files_removed.is_empty()
            || !self.conflicts.is_empty()
    }
}

impl fmt::Display for UpdateReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} updated, {} added, {} marked for removal, {} conflicts",
            self.files_updated.len(),
            self.files_added.len(),
            self.files_removed.len(),
            self.conflicts.len(),
        )
    }
}

/// Update a project by re-applying its template with the latest version.
///
/// This performs a three-way merge:
/// 1. Load old answers from the project's `.diecut-answers.toml`
/// 2. Clone/fetch the latest template
/// 3. Generate "old snapshot" (template at old ref + old answers)
/// 4. Generate "new snapshot" (template at new ref + old answers)
/// 5. Three-way merge: old snapshot vs new snapshot vs user's project
pub fn update_project(project_path: &Path, options: UpdateOptions) -> Result<UpdateReport> {
    // 1. Load saved answers
    let saved = load_answers(project_path)?;

    // Determine template source
    let template_arg = options
        .template_source
        .as_deref()
        .unwrap_or(&saved.template_source);

    if template_arg.is_empty() {
        return Err(DicecutError::NoAnswerFile {
            path: project_path.to_path_buf(),
        });
    }

    let new_ref = options.git_ref.as_deref();

    // 2. Resolve template source for the new version
    let new_source = resolve_source(template_arg)?;
    let new_template_dir = match &new_source {
        TemplateSource::Local(path) => path.clone(),
        TemplateSource::Git { url, git_ref: _ } => get_or_clone(url, new_ref)?.0,
    };

    // 3. Generate old snapshot (using old ref if available)
    let old_snapshot = tempfile::tempdir().map_err(|e| DicecutError::Io {
        context: "creating temp dir for old snapshot".into(),
        source: e,
    })?;

    // For old snapshot, we need to get the template at the old ref
    let old_template_dir = match &new_source {
        TemplateSource::Local(path) => {
            // For local templates, old and new are the same (no versioning)
            path.clone()
        }
        TemplateSource::Git { url, .. } => get_or_clone(url, saved.template_ref.as_deref())?.0,
    };

    // Resolve and render old snapshot
    let old_resolved = resolve_template(&old_template_dir)?;
    let old_variables = build_variables_from_answers(&saved.answers, &old_resolved);
    let old_context = build_context_with_namespace(&old_variables, &old_resolved.context_namespace);
    walk_and_render(
        &old_resolved,
        old_snapshot.path(),
        &old_variables,
        &old_context,
    )?;

    // 4. Generate new snapshot
    let new_snapshot = tempfile::tempdir().map_err(|e| DicecutError::Io {
        context: "creating temp dir for new snapshot".into(),
        source: e,
    })?;

    let new_resolved = resolve_template(&new_template_dir)?;
    let new_variables = build_variables_from_answers(&saved.answers, &new_resolved);
    let new_context = build_context_with_namespace(&new_variables, &new_resolved.context_namespace);
    walk_and_render(
        &new_resolved,
        new_snapshot.path(),
        &new_variables,
        &new_context,
    )?;

    // 5. Three-way merge
    let merge_results = three_way_merge(project_path, old_snapshot.path(), new_snapshot.path())?;

    let report = UpdateReport::from_results(&merge_results);

    // 6. Apply the merge
    merge::apply_merge(
        project_path,
        new_snapshot.path(),
        old_snapshot.path(),
        &merge_results,
    )?;

    // 7. Update the answers file with new ref
    let effective_ref = new_ref.or(saved.template_ref.as_deref());
    write_answers_with_source(
        project_path,
        &new_resolved.config,
        &new_variables,
        Some(template_arg),
        effective_ref,
        None,
    )?;

    Ok(report)
}

/// Build a BTreeMap of tera variables from saved TOML answer values,
/// using defaults from the config for any missing variables.
fn build_variables_from_answers(
    answers: &std::collections::HashMap<String, toml::Value>,
    resolved: &crate::adapter::ResolvedTemplate,
) -> BTreeMap<String, tera::Value> {
    let mut variables = BTreeMap::new();

    // First, apply defaults from the template config
    for (name, var_config) in &resolved.config.variables {
        if let Some(default) = &var_config.default {
            variables.insert(name.clone(), toml_value_to_tera(default));
        }
    }

    // Then override with saved answers
    for (name, value) in answers {
        variables.insert(name.clone(), toml_value_to_tera(value));
    }

    // Evaluate computed variables (we can't use collect_variables since it prompts interactively)
    let computed_vars: Vec<_> = resolved
        .config
        .variables
        .iter()
        .filter(|(_, v)| v.computed.is_some())
        .map(|(name, var)| (name.clone(), var.computed.clone().unwrap()))
        .collect();

    for _ in 0..computed_vars.len() + 1 {
        let mut progress = false;
        for (name, expr) in &computed_vars {
            if variables.contains_key(name) {
                continue;
            }
            let mut tera = tera::Tera::default();
            if tera.add_raw_template("__computed__", expr).is_err() {
                continue;
            }
            let mut context = tera::Context::new();
            for (k, v) in &variables {
                context.insert(k, v);
            }
            if let Ok(result) = tera.render("__computed__", &context) {
                variables.insert(name.clone(), tera::Value::String(result));
                progress = true;
            }
        }
        if !progress {
            break;
        }
    }

    variables
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::answers::write_answers_with_source;
    use crate::config::schema::{
        AnswersConfig, FilesConfig, HooksConfig, TemplateConfig, TemplateMetadata,
    };

    #[test]
    fn test_answer_file_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();

        let config = TemplateConfig {
            template: TemplateMetadata {
                name: "roundtrip-test".to_string(),
                version: Some("2.0.0".to_string()),
                description: None,
                min_diecut_version: None,
                templates_suffix: ".tera".to_string(),
            },
            variables: Default::default(),
            files: FilesConfig::default(),
            hooks: HooksConfig::default(),
            answers: AnswersConfig::default(),
        };

        let mut vars = BTreeMap::new();
        vars.insert(
            "project_name".to_string(),
            tera::Value::String("myproject".to_string()),
        );
        vars.insert("use_ci".to_string(), tera::Value::Bool(true));
        vars.insert(
            "port".to_string(),
            tera::Value::Number(serde_json::Number::from(8080)),
        );

        write_answers_with_source(
            dir,
            &config,
            &vars,
            Some("gh:user/template"),
            Some("v2.0"),
            None,
        )
        .unwrap();

        let loaded = load_answers(dir).unwrap();
        assert_eq!(loaded.template_source, "gh:user/template");
        assert_eq!(loaded.template_ref.as_deref(), Some("v2.0"));
        assert!(!loaded.diecut_version.is_empty());
        assert_eq!(
            loaded.answers.get("project_name"),
            Some(&toml::Value::String("myproject".to_string()))
        );
        assert_eq!(
            loaded.answers.get("use_ci"),
            Some(&toml::Value::Boolean(true))
        );
        assert_eq!(
            loaded.answers.get("port"),
            Some(&toml::Value::Integer(8080))
        );
    }

    #[test]
    fn test_load_answers_missing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let result = load_answers(tmp.path());
        assert!(result.is_err());
        match result.unwrap_err() {
            DicecutError::NoAnswerFile { .. } => {}
            other => panic!("expected NoAnswerFile, got: {other:?}"),
        }
    }

    #[test]
    fn test_three_way_merge_user_unchanged_template_changed() {
        let old_snap = tempfile::tempdir().unwrap();
        let new_snap = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();

        // Old snapshot and project have same content
        std::fs::write(old_snap.path().join("file.txt"), "old content").unwrap();
        std::fs::write(project.path().join("file.txt"), "old content").unwrap();
        // New snapshot has updated content
        std::fs::write(new_snap.path().join("file.txt"), "new content").unwrap();

        let results = three_way_merge(project.path(), old_snap.path(), new_snap.path()).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action, MergeAction::UpdateFromTemplate);
    }

    #[test]
    fn test_three_way_merge_user_changed_template_unchanged() {
        let old_snap = tempfile::tempdir().unwrap();
        let new_snap = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();

        std::fs::write(old_snap.path().join("file.txt"), "original").unwrap();
        std::fs::write(new_snap.path().join("file.txt"), "original").unwrap();
        std::fs::write(project.path().join("file.txt"), "user modified").unwrap();

        let results = three_way_merge(project.path(), old_snap.path(), new_snap.path()).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action, MergeAction::KeepUser);
    }

    #[test]
    fn test_three_way_merge_both_changed_conflict() {
        let old_snap = tempfile::tempdir().unwrap();
        let new_snap = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();

        std::fs::write(old_snap.path().join("file.txt"), "original").unwrap();
        std::fs::write(new_snap.path().join("file.txt"), "template changed").unwrap();
        std::fs::write(project.path().join("file.txt"), "user changed").unwrap();

        let results = three_way_merge(project.path(), old_snap.path(), new_snap.path()).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action, MergeAction::Conflict);
    }

    #[test]
    fn test_three_way_merge_new_file_in_template() {
        let old_snap = tempfile::tempdir().unwrap();
        let new_snap = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();

        // Only exists in new snapshot
        std::fs::write(new_snap.path().join("new-file.txt"), "brand new").unwrap();
        // Project needs at least one file to exist as a dir
        std::fs::write(project.path().join("existing.txt"), "exists").unwrap();
        std::fs::write(old_snap.path().join("existing.txt"), "exists").unwrap();
        std::fs::write(new_snap.path().join("existing.txt"), "exists").unwrap();

        let results = three_way_merge(project.path(), old_snap.path(), new_snap.path()).unwrap();

        let new_file_result = results
            .iter()
            .find(|r| r.rel_path.as_os_str() == "new-file.txt")
            .expect("should have result for new-file.txt");
        assert_eq!(new_file_result.action, MergeAction::AddFromTemplate);
    }

    #[test]
    fn test_three_way_merge_file_removed_from_template() {
        let old_snap = tempfile::tempdir().unwrap();
        let new_snap = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();

        // File in old and project, but not in new
        std::fs::write(old_snap.path().join("removed.txt"), "old content").unwrap();
        std::fs::write(project.path().join("removed.txt"), "old content").unwrap();
        // new_snap doesn't have it

        // Need at least one file in new_snap
        std::fs::write(new_snap.path().join("other.txt"), "other").unwrap();
        std::fs::write(old_snap.path().join("other.txt"), "other").unwrap();
        std::fs::write(project.path().join("other.txt"), "other").unwrap();

        let results = three_way_merge(project.path(), old_snap.path(), new_snap.path()).unwrap();

        let removed_result = results
            .iter()
            .find(|r| r.rel_path.as_os_str() == "removed.txt")
            .expect("should have result for removed.txt");
        assert_eq!(removed_result.action, MergeAction::MarkForRemoval);
    }

    #[test]
    fn test_update_report_display() {
        let results = vec![
            FileMergeResult {
                rel_path: PathBuf::from("a.txt"),
                action: MergeAction::UpdateFromTemplate,
            },
            FileMergeResult {
                rel_path: PathBuf::from("b.txt"),
                action: MergeAction::AddFromTemplate,
            },
            FileMergeResult {
                rel_path: PathBuf::from("c.txt"),
                action: MergeAction::Conflict,
            },
        ];

        let report = UpdateReport::from_results(&results);
        assert_eq!(report.files_updated.len(), 1);
        assert_eq!(report.files_added.len(), 1);
        assert_eq!(report.conflicts.len(), 1);
        assert!(report.has_changes());

        let display = format!("{report}");
        assert!(display.contains("1 updated"));
        assert!(display.contains("1 added"));
        assert!(display.contains("1 conflicts"));
    }

    #[test]
    fn test_apply_merge_updates_files() {
        let old_snap = tempfile::tempdir().unwrap();
        let new_snap = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();

        // Setup: user hasn't changed file, template updated it
        std::fs::write(old_snap.path().join("file.txt"), "old content").unwrap();
        std::fs::write(new_snap.path().join("file.txt"), "new content").unwrap();
        std::fs::write(project.path().join("file.txt"), "old content").unwrap();

        let results = vec![FileMergeResult {
            rel_path: PathBuf::from("file.txt"),
            action: MergeAction::UpdateFromTemplate,
        }];

        merge::apply_merge(project.path(), new_snap.path(), old_snap.path(), &results).unwrap();

        let content = std::fs::read_to_string(project.path().join("file.txt")).unwrap();
        assert_eq!(content, "new content");
    }

    #[test]
    fn test_apply_merge_adds_new_files() {
        let old_snap = tempfile::tempdir().unwrap();
        let new_snap = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();

        std::fs::write(new_snap.path().join("new-file.txt"), "brand new").unwrap();

        let results = vec![FileMergeResult {
            rel_path: PathBuf::from("new-file.txt"),
            action: MergeAction::AddFromTemplate,
        }];

        merge::apply_merge(project.path(), new_snap.path(), old_snap.path(), &results).unwrap();

        let content = std::fs::read_to_string(project.path().join("new-file.txt")).unwrap();
        assert_eq!(content, "brand new");
    }

    #[test]
    fn test_apply_merge_conflict_writes_rej_file() {
        let old_snap = tempfile::tempdir().unwrap();
        let new_snap = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();

        std::fs::write(old_snap.path().join("file.txt"), "original").unwrap();
        std::fs::write(new_snap.path().join("file.txt"), "template changed").unwrap();
        std::fs::write(project.path().join("file.txt"), "user changed").unwrap();

        let results = vec![FileMergeResult {
            rel_path: PathBuf::from("file.txt"),
            action: MergeAction::Conflict,
        }];

        merge::apply_merge(project.path(), new_snap.path(), old_snap.path(), &results).unwrap();

        // User's file should be untouched
        let content = std::fs::read_to_string(project.path().join("file.txt")).unwrap();
        assert_eq!(content, "user changed");

        // .rej file should exist
        assert!(project.path().join("file.txt.rej").exists());
        let rej = std::fs::read_to_string(project.path().join("file.txt.rej")).unwrap();
        assert!(rej.contains("Conflict"));
    }
}
