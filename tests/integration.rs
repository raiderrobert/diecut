use std::collections::BTreeMap;
use std::path::PathBuf;

use diecut::adapter::migrate::{execute_migration, plan_migration, FileOp};
use diecut::adapter::{self, TemplateFormat};
use diecut::config::load_config;
use diecut::prompt::PromptOptions;
use diecut::render::{
    build_context, build_context_with_namespace, execute_plan, plan_render, walk_and_render,
};
use diecut::template::source::{resolve_source, resolve_source_full};
use diecut::update::merge::{apply_merge, three_way_merge, MergeAction};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn default_variables() -> BTreeMap<String, tera::Value> {
    let mut vars = BTreeMap::new();
    vars.insert(
        "project_name".to_string(),
        tera::Value::String("test-project".to_string()),
    );
    vars.insert(
        "author".to_string(),
        tera::Value::String("Jane Doe".to_string()),
    );
    vars.insert("use_docker".to_string(), tera::Value::Bool(false));
    vars.insert(
        "license".to_string(),
        tera::Value::String("MIT".to_string()),
    );
    vars.insert(
        "project_slug".to_string(),
        tera::Value::String("test-project".to_string()),
    );
    vars
}

#[test]
fn test_load_config() {
    let config = load_config(&fixture_path("basic-template")).unwrap();
    assert_eq!(config.template.name, "basic-test");
    assert_eq!(config.template.version.as_deref(), Some("0.1.0"));
    assert_eq!(config.variables.len(), 5);
    assert_eq!(config.template.templates_suffix, ".tera");
}

#[test]
fn test_config_validation_select_without_choices() {
    let toml_str = r#"
[template]
name = "bad"

[variables.pick]
type = "select"
prompt = "Pick one"
"#;
    let config: diecut::config::schema::TemplateConfig = toml::from_str(toml_str).unwrap();
    let result = config.validate();
    assert!(result.is_err());
}

#[test]
fn test_generate_basic_template() {
    let template_dir = fixture_path("basic-template");
    let resolved = adapter::resolve_template(&template_dir).unwrap();
    let variables = default_variables();
    let context = build_context(&variables);

    let output_dir = tempfile::tempdir().unwrap();
    let result = walk_and_render(&resolved, output_dir.path(), &variables, &context).unwrap();

    // Check output directory has the project directory (rendered from {{project_name}})
    let project_dir = output_dir.path().join("test-project");
    assert!(project_dir.exists(), "project directory should exist");

    // Check .tera files were rendered and suffix stripped
    let readme = project_dir.join("README.md");
    assert!(readme.exists(), "README.md should exist (suffix stripped)");
    let readme_content = std::fs::read_to_string(&readme).unwrap();
    assert!(
        readme_content.contains("test-project"),
        "README should contain project name"
    );
    assert!(
        readme_content.contains("Jane Doe"),
        "README should contain author"
    );
    assert!(
        readme_content.contains("MIT"),
        "README should contain license"
    );

    // Check Cargo.toml was rendered
    let cargo_toml = project_dir.join("Cargo.toml");
    assert!(cargo_toml.exists(), "Cargo.toml should exist");
    let cargo_content = std::fs::read_to_string(&cargo_toml).unwrap();
    assert!(cargo_content.contains("test-project"));
    assert!(cargo_content.contains("Jane Doe"));

    // Check main.rs was rendered
    let main_rs = project_dir.join("src/main.rs");
    assert!(main_rs.exists(), "src/main.rs should exist");
    let main_content = std::fs::read_to_string(&main_rs).unwrap();
    assert!(main_content.contains("test-project"));

    // Check .gitignore was copied verbatim (no .tera suffix)
    let gitignore = project_dir.join(".gitignore");
    assert!(gitignore.exists(), ".gitignore should be copied verbatim");
    let gitignore_content = std::fs::read_to_string(&gitignore).unwrap();
    assert!(gitignore_content.contains("/target"));

    // Check binary file was copied verbatim
    let logo = project_dir.join("assets/logo.png");
    assert!(logo.exists(), "logo.png should be copied");
    let logo_bytes = std::fs::read(&logo).unwrap();
    assert_eq!(&logo_bytes[..4], b"\x89PNG", "PNG header should be intact");

    // Check file counts
    assert!(
        !result.files_created.is_empty(),
        "should have rendered files"
    );
    assert!(!result.files_copied.is_empty(), "should have copied files");
}

#[test]
fn test_conditional_file_excluded() {
    let template_dir = fixture_path("basic-template");
    let resolved = adapter::resolve_template(&template_dir).unwrap();
    let mut variables = default_variables();
    variables.insert("use_docker".to_string(), tera::Value::Bool(false));

    let context = build_context(&variables);
    let output_dir = tempfile::tempdir().unwrap();
    walk_and_render(&resolved, output_dir.path(), &variables, &context).unwrap();

    let dockerfile = output_dir.path().join("test-project/Dockerfile");
    assert!(
        !dockerfile.exists(),
        "Dockerfile should not exist when use_docker=false"
    );
}

#[test]
fn test_conditional_file_included() {
    let template_dir = fixture_path("basic-template");
    let resolved = adapter::resolve_template(&template_dir).unwrap();
    let mut variables = default_variables();
    variables.insert("use_docker".to_string(), tera::Value::Bool(true));

    let context = build_context(&variables);
    let output_dir = tempfile::tempdir().unwrap();
    walk_and_render(&resolved, output_dir.path(), &variables, &context).unwrap();

    let dockerfile = output_dir.path().join("test-project/Dockerfile");
    assert!(
        dockerfile.exists(),
        "Dockerfile should exist when use_docker=true"
    );
}

#[test]
fn test_computed_variable() {
    let config = load_config(&fixture_path("basic-template")).unwrap();
    let computed_var = config.variables.get("project_slug").unwrap();
    assert!(computed_var.computed.is_some());
    assert_eq!(
        computed_var.computed.as_deref(),
        Some("{{ project_name | slugify }}")
    );
}

#[test]
fn test_prompt_options_with_data_overrides() {
    let options = PromptOptions {
        data_overrides: [
            ("project_name".to_string(), "override-name".to_string()),
            ("author".to_string(), "Override Author".to_string()),
            ("use_docker".to_string(), "false".to_string()),
            ("license".to_string(), "MIT".to_string()),
        ]
        .into_iter()
        .collect(),
        use_defaults: false,
    };

    let config = load_config(&fixture_path("basic-template")).unwrap();
    let variables = diecut::prompt::collect_variables(&config, &options).unwrap();

    assert_eq!(
        variables.get("project_name"),
        Some(&tera::Value::String("override-name".to_string()))
    );
    assert_eq!(
        variables.get("author"),
        Some(&tera::Value::String("Override Author".to_string()))
    );
    assert!(variables.contains_key("project_slug"));
}

#[test]
fn test_prompt_options_with_defaults() {
    let options = PromptOptions {
        data_overrides: std::collections::HashMap::new(),
        use_defaults: true,
    };

    let config = load_config(&fixture_path("basic-template")).unwrap();
    let variables = diecut::prompt::collect_variables(&config, &options).unwrap();

    assert_eq!(
        variables.get("project_name"),
        Some(&tera::Value::String("my-project".to_string()))
    );
    assert_eq!(
        variables.get("author"),
        Some(&tera::Value::String("Test Author".to_string()))
    );
    assert_eq!(variables.get("use_docker"), Some(&tera::Value::Bool(false)));
    assert_eq!(
        variables.get("license"),
        Some(&tera::Value::String("MIT".to_string()))
    );
}

#[test]
fn test_answers_file_written() {
    let template_dir = fixture_path("basic-template");
    let resolved = adapter::resolve_template(&template_dir).unwrap();
    let variables = default_variables();
    let context = build_context(&variables);

    let output_dir = tempfile::tempdir().unwrap();
    walk_and_render(&resolved, output_dir.path(), &variables, &context).unwrap();

    let source_info = diecut::answers::SourceInfo {
        url: None,
        git_ref: None,
        commit_sha: None,
    };
    diecut::answers::write_answers(
        output_dir.path(),
        &resolved.config,
        &variables,
        &source_info,
    )
    .unwrap();

    let answers_path = output_dir.path().join(".diecut-answers.toml");
    assert!(answers_path.exists(), "answers file should be written");
    let content = std::fs::read_to_string(&answers_path).unwrap();
    assert!(
        content.contains("basic-test"),
        "should contain template name"
    );
    assert!(
        content.contains("test-project"),
        "should contain project_name value"
    );
}

#[test]
fn test_template_directory_missing() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("diecut.toml"),
        r#"
[template]
name = "missing-template-dir"
"#,
    )
    .unwrap();

    let resolved = adapter::resolve_template(temp.path()).unwrap();
    let variables = BTreeMap::new();
    let context = build_context(&variables);

    let output_dir = tempfile::tempdir().unwrap();
    let result = walk_and_render(&resolved, output_dir.path(), &variables, &context);
    assert!(result.is_err(), "should fail when template/ dir is missing");
}

// --- Format detection tests ---

#[test]
fn test_detect_native_format() {
    let template_dir = fixture_path("basic-template");
    let resolved = adapter::resolve_template(&template_dir).unwrap();
    assert_eq!(resolved.format, TemplateFormat::Native);
    assert!(!resolved.render_all);
    assert!(resolved.context_namespace.is_none());
}

#[test]
fn test_detect_cookiecutter_format() {
    let template_dir = fixture_path("cookiecutter-basic");
    let resolved = adapter::resolve_template(&template_dir).unwrap();
    assert_eq!(resolved.format, TemplateFormat::Cookiecutter);
    assert!(resolved.render_all);
    assert_eq!(resolved.context_namespace.as_deref(), Some("cookiecutter"));
}

// --- Cookiecutter adapter tests ---

#[test]
fn test_cookiecutter_generate_with_defaults() {
    let template_dir = fixture_path("cookiecutter-basic");
    let resolved = adapter::resolve_template(&template_dir).unwrap();

    // Use defaults
    let options = PromptOptions {
        data_overrides: std::collections::HashMap::new(),
        use_defaults: true,
    };
    let variables = diecut::prompt::collect_variables(&resolved.config, &options).unwrap();

    // Build context with cookiecutter namespace
    let context = build_context_with_namespace(&variables, &resolved.context_namespace);

    let output_dir = tempfile::tempdir().unwrap();
    let result = walk_and_render(&resolved, output_dir.path(), &variables, &context).unwrap();

    // The project_slug is computed from project_name "My Project" → "my-project"
    // (via {{ project_name | lower | replace(' ', '-') }})
    let slug = variables.get("project_slug").unwrap();
    let slug_str = slug.as_str().unwrap();

    // Check the rendered directory exists
    let project_dir = output_dir.path().join(slug_str);
    assert!(
        project_dir.exists(),
        "project directory '{}' should exist",
        slug_str
    );

    // Check README was rendered with cookiecutter.* resolved
    let readme = project_dir.join("README.md");
    assert!(readme.exists(), "README.md should exist");
    let readme_content = std::fs::read_to_string(&readme).unwrap();
    assert!(
        readme_content.contains("My Project"),
        "README should contain project_name, got: {readme_content}"
    );
    assert!(
        readme_content.contains("Your Name"),
        "README should contain full_name, got: {readme_content}"
    );

    // Check setup.py was rendered
    let setup_py = project_dir.join("setup.py");
    assert!(setup_py.exists(), "setup.py should exist");
    let setup_content = std::fs::read_to_string(&setup_py).unwrap();
    assert!(
        setup_content.contains(slug_str),
        "setup.py should contain project_slug"
    );
    assert!(
        setup_content.contains("Your Name"),
        "setup.py should contain full_name"
    );

    // Check nested directory was rendered
    let nested_dir = project_dir.join(slug_str);
    assert!(nested_dir.exists(), "nested project directory should exist");

    // Check __init__.py exists (empty file, copied)
    let init_py = nested_dir.join("__init__.py");
    assert!(init_py.exists(), "__init__.py should exist");

    assert!(
        !result.files_created.is_empty(),
        "should have rendered files"
    );
}

#[test]
fn test_cookiecutter_computed_variable() {
    let template_dir = fixture_path("cookiecutter-basic");
    let resolved = adapter::resolve_template(&template_dir).unwrap();

    let options = PromptOptions {
        data_overrides: std::collections::HashMap::new(),
        use_defaults: true,
    };
    let variables = diecut::prompt::collect_variables(&resolved.config, &options).unwrap();

    // project_slug should be computed from project_name "My Project"
    let slug = variables.get("project_slug").unwrap();
    assert_eq!(
        slug.as_str().unwrap(),
        "my-project",
        "project_slug should be computed as 'my-project'"
    );
}

#[test]
fn test_cookiecutter_choice_variable() {
    let template_dir = fixture_path("cookiecutter-basic");
    let resolved = adapter::resolve_template(&template_dir).unwrap();

    // license should be a select type with default "MIT" (first choice)
    let license_var = resolved.config.variables.get("license").unwrap();
    assert_eq!(
        license_var.var_type,
        diecut::config::variable::VariableType::Select
    );
    assert_eq!(
        license_var.choices.as_ref().unwrap(),
        &["MIT", "BSD-3", "Apache-2.0"]
    );
    assert_eq!(
        license_var.default,
        Some(toml::Value::String("MIT".to_string()))
    );
}

#[test]
fn test_cookiecutter_copy_without_render() {
    let template_dir = fixture_path("cookiecutter-basic");
    let resolved = adapter::resolve_template(&template_dir).unwrap();

    assert!(
        resolved
            .config
            .files
            .copy_without_render
            .contains(&"*.csv".to_string()),
        "copy_without_render should include *.csv"
    );
}

#[test]
fn test_cookiecutter_hooks_warning() {
    let template_dir = fixture_path("cookiecutter-with-hooks");
    let resolved = adapter::resolve_template(&template_dir).unwrap();

    assert!(
        resolved.warnings.iter().any(|w| w.contains("hooks")),
        "should warn about Python hooks, got: {:?}",
        resolved.warnings
    );
}

// --- Migration tests ---

#[test]
fn test_migration_plan_cookiecutter() {
    let template_dir = fixture_path("cookiecutter-basic");
    let plan = plan_migration(&template_dir).unwrap();

    assert_eq!(plan.source_format, TemplateFormat::Cookiecutter);
    assert!(!plan.operations.is_empty(), "plan should have operations");

    // Should have a Create operation for diecut.toml
    assert!(
        plan.operations.iter().any(|op| matches!(op, FileOp::Create { path, .. } if path.to_string_lossy().contains("diecut.toml"))),
        "plan should create diecut.toml"
    );

    // Should have a Delete operation for cookiecutter.json
    assert!(
        plan.operations.iter().any(|op| matches!(op, FileOp::Delete { path } if path.to_string_lossy().contains("cookiecutter.json"))),
        "plan should delete cookiecutter.json"
    );

    // Should have Move operations for template files
    let move_count = plan
        .operations
        .iter()
        .filter(|op| matches!(op, FileOp::Move { .. }))
        .count();
    assert!(move_count > 0, "plan should have file moves");

    // The generated diecut.toml should contain template metadata
    assert!(
        plan.diecut_toml_content.contains("[template]"),
        "diecut.toml should contain [template] section"
    );
    assert!(
        plan.diecut_toml_content.contains("[variables."),
        "diecut.toml should contain variable definitions"
    );
}

#[test]
fn test_migration_plan_native_fails() {
    let template_dir = fixture_path("basic-template");
    let result = plan_migration(&template_dir);
    assert!(result.is_err(), "migrating a native template should fail");
}

#[test]
fn test_migration_execute() {
    let template_dir = fixture_path("cookiecutter-basic");
    let plan = plan_migration(&template_dir).unwrap();

    let output_dir = tempfile::tempdir().unwrap();
    execute_migration(&plan, &template_dir, output_dir.path()).unwrap();

    // Check diecut.toml was created
    let diecut_toml = output_dir.path().join("diecut.toml");
    assert!(diecut_toml.exists(), "diecut.toml should be created");
    let content = std::fs::read_to_string(&diecut_toml).unwrap();
    assert!(content.contains("[template]"));
    assert!(content.contains("cookiecutter-basic")); // template name from directory

    // Check template/ directory was created with content
    let template_subdir = output_dir.path().join("template");
    assert!(template_subdir.exists(), "template/ directory should exist");

    // Check that {{project_slug}} directory exists (with cookiecutter. removed)
    let project_dir = template_subdir.join("{{project_slug}}");
    assert!(
        project_dir.exists(),
        "{{project_slug}} directory should exist under template/"
    );

    // Check that README.md was moved and has .tera suffix (since it has template syntax)
    let readme = project_dir.join("README.md.tera");
    assert!(readme.exists(), "README.md.tera should exist");
    let readme_content = std::fs::read_to_string(&readme).unwrap();
    // The cookiecutter.X references should have been rewritten to X
    assert!(
        !readme_content.contains("cookiecutter."),
        "cookiecutter. references should be removed from README, got: {readme_content}"
    );
    assert!(
        readme_content.contains("project_name"),
        "should still reference project_name variable"
    );

    // The migrated template should be usable with diecut
    let migrated_resolved = adapter::resolve_template(output_dir.path()).unwrap();
    assert_eq!(migrated_resolved.format, TemplateFormat::Native);
}

// --- Edge case: merge with binary files ---

#[test]
fn test_three_way_merge_binary_files_unchanged() {
    let old_snap = tempfile::tempdir().unwrap();
    let new_snap = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    // Binary content (has null bytes)
    let binary = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
    std::fs::write(old_snap.path().join("logo.png"), binary).unwrap();
    std::fs::write(new_snap.path().join("logo.png"), binary).unwrap();
    std::fs::write(project.path().join("logo.png"), binary).unwrap();

    let results = three_way_merge(project.path(), old_snap.path(), new_snap.path()).unwrap();
    // All identical → should produce no changes
    assert!(
        results.is_empty(),
        "identical binary files should produce no merge results"
    );
}

#[test]
fn test_three_way_merge_binary_file_updated_in_template() {
    let old_snap = tempfile::tempdir().unwrap();
    let new_snap = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    let old_binary = b"\x89PNG\r\n\x1a\n\x00OLD";
    let new_binary = b"\x89PNG\r\n\x1a\n\x00NEW";
    std::fs::write(old_snap.path().join("logo.png"), old_binary).unwrap();
    std::fs::write(new_snap.path().join("logo.png"), new_binary).unwrap();
    std::fs::write(project.path().join("logo.png"), old_binary).unwrap();

    let results = three_way_merge(project.path(), old_snap.path(), new_snap.path()).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].action, MergeAction::UpdateFromTemplate);
}

// --- Edge case: merge with empty files ---

#[test]
fn test_three_way_merge_empty_files() {
    let old_snap = tempfile::tempdir().unwrap();
    let new_snap = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    std::fs::write(old_snap.path().join("empty.txt"), "").unwrap();
    std::fs::write(new_snap.path().join("empty.txt"), "").unwrap();
    std::fs::write(project.path().join("empty.txt"), "").unwrap();

    let results = three_way_merge(project.path(), old_snap.path(), new_snap.path()).unwrap();
    assert!(
        results.is_empty(),
        "identical empty files should be unchanged"
    );
}

#[test]
fn test_three_way_merge_empty_to_content() {
    let old_snap = tempfile::tempdir().unwrap();
    let new_snap = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    std::fs::write(old_snap.path().join("file.txt"), "").unwrap();
    std::fs::write(new_snap.path().join("file.txt"), "new content").unwrap();
    std::fs::write(project.path().join("file.txt"), "").unwrap();

    let results = three_way_merge(project.path(), old_snap.path(), new_snap.path()).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].action, MergeAction::UpdateFromTemplate);
}

// --- Edge case: merge with nested directory changes ---

#[test]
fn test_three_way_merge_nested_new_file() {
    let old_snap = tempfile::tempdir().unwrap();
    let new_snap = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    // Common file in all three
    std::fs::write(old_snap.path().join("root.txt"), "stable").unwrap();
    std::fs::write(new_snap.path().join("root.txt"), "stable").unwrap();
    std::fs::write(project.path().join("root.txt"), "stable").unwrap();

    // New file in a nested directory only in new snapshot
    std::fs::create_dir_all(new_snap.path().join("sub/deep")).unwrap();
    std::fs::write(new_snap.path().join("sub/deep/new.txt"), "hello").unwrap();

    let results = three_way_merge(project.path(), old_snap.path(), new_snap.path()).unwrap();
    let new_file = results
        .iter()
        .find(|r| r.rel_path.ends_with("sub/deep/new.txt"));
    assert!(new_file.is_some(), "should detect new nested file");
    assert_eq!(new_file.unwrap().action, MergeAction::AddFromTemplate);
}

// --- Edge case: both sides converge to same content ---

#[test]
fn test_three_way_merge_convergent_changes() {
    let old_snap = tempfile::tempdir().unwrap();
    let new_snap = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    std::fs::write(old_snap.path().join("file.txt"), "original").unwrap();
    // Both user and template independently changed to the same content
    std::fs::write(new_snap.path().join("file.txt"), "converged").unwrap();
    std::fs::write(project.path().join("file.txt"), "converged").unwrap();

    let results = three_way_merge(project.path(), old_snap.path(), new_snap.path()).unwrap();
    // Should detect convergence → no conflict
    assert!(
        results.is_empty(),
        "convergent changes should produce no merge results"
    );
}

// --- Edge case: template source URL parsing ---

#[test]
fn test_resolve_source_rejects_empty_abbreviation_remainder() {
    assert!(resolve_source("gh:").is_err());
    assert!(resolve_source("gl:").is_err());
    assert!(resolve_source("bb:").is_err());
    assert!(resolve_source("sr:").is_err());
}

#[test]
fn test_resolve_source_user_abbreviation_empty_remainder() {
    let mut abbrevs = std::collections::HashMap::new();
    abbrevs.insert("co".to_string(), "https://git.co.com/{}.git".to_string());
    assert!(resolve_source_full("co:", None, Some(&abbrevs)).is_err());
}

// --- Edge case: unsupported template format ---

#[test]
fn test_resolve_template_unsupported_format() {
    let tmp = tempfile::tempdir().unwrap();
    // No diecut.toml or cookiecutter.json
    std::fs::write(tmp.path().join("random.txt"), "not a template").unwrap();

    let result = adapter::resolve_template(tmp.path());
    assert!(result.is_err(), "should fail for unsupported format");
}

// --- Edge case: render with special characters in variable values ---

#[test]
fn test_render_with_special_characters() {
    let template_dir = fixture_path("basic-template");
    let resolved = adapter::resolve_template(&template_dir).unwrap();

    let mut variables = default_variables();
    // Use a name with special characters that could trip up template engines
    variables.insert(
        "project_name".to_string(),
        tera::Value::String("my-project_v2.0".to_string()),
    );
    variables.insert(
        "project_slug".to_string(),
        tera::Value::String("my-project_v2.0".to_string()),
    );

    let context = build_context(&variables);
    let output_dir = tempfile::tempdir().unwrap();
    let result = walk_and_render(&resolved, output_dir.path(), &variables, &context);
    assert!(result.is_ok(), "should handle special characters in values");

    // Verify the rendered output actually contains the special character value
    let project_dir = output_dir.path().join("my-project_v2.0");
    assert!(
        project_dir.exists(),
        "project directory with special characters should exist"
    );

    let readme = project_dir.join("README.md");
    assert!(readme.exists(), "README.md should exist");
    let readme_content = std::fs::read_to_string(&readme).unwrap();
    assert!(
        readme_content.contains("my-project_v2.0"),
        "README should contain the special character project name, got: {readme_content}"
    );

    let cargo_toml = project_dir.join("Cargo.toml");
    assert!(cargo_toml.exists(), "Cargo.toml should exist");
    let cargo_content = std::fs::read_to_string(&cargo_toml).unwrap();
    assert!(
        cargo_content.contains("my-project_v2.0"),
        "Cargo.toml should contain the special character project name, got: {cargo_content}"
    );
}

// --- plan_generation dry-run tests ---

#[test]
fn test_plan_generation_dry_run_no_files_written() {
    let template_dir = fixture_path("basic-template");
    let tmp = tempfile::tempdir().unwrap();
    let output_path = tmp.path().join("dry-run-output");

    let options = diecut::GenerateOptions {
        template: template_dir.to_string_lossy().to_string(),
        output: Some(output_path.to_string_lossy().to_string()),
        data: vec![
            ("project_name".to_string(), "test-project".to_string()),
            ("author".to_string(), "Jane Doe".to_string()),
            ("use_docker".to_string(), "false".to_string()),
            ("license".to_string(), "MIT".to_string()),
        ],
        defaults: true,
        overwrite: false,
        no_hooks: true,
    };

    // plan_generation should succeed
    let plan = diecut::plan_generation(options).unwrap();

    // Plan should have files
    assert!(!plan.render_plan.files.is_empty(), "plan should have files");

    // But nothing should be written to disk
    assert!(
        !output_path.exists(),
        "output directory should NOT exist after plan_generation"
    );

    // Verify plan contents are sensible
    let has_rendered = plan.render_plan.files.iter().any(|f| !f.is_copy);
    let has_copied = plan.render_plan.files.iter().any(|f| f.is_copy);
    assert!(has_rendered, "plan should have rendered files");
    assert!(has_copied, "plan should have copied files");
}

// --- plan_render + execute_plan tests ---

#[test]
fn test_plan_render_matches_walk_and_render() {
    let template_dir = fixture_path("basic-template");
    let resolved = adapter::resolve_template(&template_dir).unwrap();
    let variables = default_variables();
    let context = build_context(&variables);

    // Generate via plan_render + execute_plan
    let plan = plan_render(&resolved, &variables, &context).unwrap();
    let plan_output = tempfile::tempdir().unwrap();
    let plan_result = execute_plan(&plan, plan_output.path()).unwrap();

    // Generate via walk_and_render
    let walk_output = tempfile::tempdir().unwrap();
    let walk_result = walk_and_render(&resolved, walk_output.path(), &variables, &context).unwrap();

    // Same file counts
    assert_eq!(
        plan_result.files_created.len(),
        walk_result.files_created.len(),
        "rendered file count should match"
    );
    assert_eq!(
        plan_result.files_copied.len(),
        walk_result.files_copied.len(),
        "copied file count should match"
    );

    // Same file paths (sorted for stable comparison)
    let mut plan_created: Vec<_> = plan_result.files_created.clone();
    plan_created.sort();
    let mut walk_created: Vec<_> = walk_result.files_created.clone();
    walk_created.sort();
    assert_eq!(
        plan_created, walk_created,
        "rendered file paths should match"
    );

    let mut plan_copied: Vec<_> = plan_result.files_copied.clone();
    plan_copied.sort();
    let mut walk_copied: Vec<_> = walk_result.files_copied.clone();
    walk_copied.sort();
    assert_eq!(plan_copied, walk_copied, "copied file paths should match");

    // Same file contents
    for rel_path in plan_result
        .files_created
        .iter()
        .chain(plan_result.files_copied.iter())
    {
        let plan_file = plan_output.path().join(rel_path);
        let walk_file = walk_output.path().join(rel_path);
        let plan_bytes = std::fs::read(&plan_file).unwrap();
        let walk_bytes = std::fs::read(&walk_file).unwrap();
        assert_eq!(
            plan_bytes,
            walk_bytes,
            "content mismatch for {}",
            rel_path.display()
        );
    }
}

#[test]
fn test_plan_render_file_counts() {
    let template_dir = fixture_path("basic-template");
    let resolved = adapter::resolve_template(&template_dir).unwrap();
    let variables = default_variables();
    let context = build_context(&variables);

    let plan = plan_render(&resolved, &variables, &context).unwrap();

    let rendered_count = plan.files.iter().filter(|f| !f.is_copy).count();
    let copied_count = plan.files.iter().filter(|f| f.is_copy).count();

    assert!(rendered_count > 0, "should have rendered files in plan");
    assert!(copied_count > 0, "should have copied files in plan");
    assert_eq!(
        rendered_count + copied_count,
        plan.files.len(),
        "rendered + copied should equal total files"
    );
}

#[test]
fn test_plan_render_conditional_exclude() {
    let template_dir = fixture_path("basic-template");
    let resolved = adapter::resolve_template(&template_dir).unwrap();
    let mut variables = default_variables();
    variables.insert("use_docker".to_string(), tera::Value::Bool(false));

    let context = build_context(&variables);
    let plan = plan_render(&resolved, &variables, &context).unwrap();

    let has_dockerfile = plan
        .files
        .iter()
        .any(|f| f.relative_path.to_string_lossy().contains("Dockerfile"));
    assert!(
        !has_dockerfile,
        "Dockerfile should not be in plan when use_docker=false"
    );
}

// --- Dry-run: verbose content available in plan ---

#[test]
fn test_plan_generation_verbose_has_content() {
    let template_dir = fixture_path("basic-template");

    let tmp = tempfile::tempdir().unwrap();
    let output_path = tmp.path().join("verbose-output");

    let options = diecut::GenerateOptions {
        template: template_dir.to_string_lossy().to_string(),
        output: Some(output_path.to_string_lossy().to_string()),
        data: vec![
            ("project_name".to_string(), "test-project".to_string()),
            ("author".to_string(), "Jane Doe".to_string()),
            ("use_docker".to_string(), "false".to_string()),
            ("license".to_string(), "MIT".to_string()),
        ],
        defaults: true,
        overwrite: false,
        no_hooks: true,
    };

    let plan = diecut::plan_generation(options).unwrap();

    // Rendered files should have non-empty content with template variables resolved
    for file in &plan.render_plan.files {
        assert!(
            !file.content.is_empty(),
            "file {} should have content",
            file.relative_path.display()
        );

        if !file.is_copy {
            // Rendered files should be valid UTF-8
            let text = String::from_utf8(file.content.clone());
            assert!(
                text.is_ok(),
                "rendered file {} should be valid UTF-8",
                file.relative_path.display()
            );
        }
    }

    // At least one rendered file should contain the resolved project name
    let has_project_name = plan
        .render_plan
        .files
        .iter()
        .any(|f| !f.is_copy && String::from_utf8_lossy(&f.content).contains("test-project"));
    assert!(
        has_project_name,
        "at least one rendered file should contain the resolved project name"
    );
}

// --- Dry-run: merge report without writing changes ---

#[test]
fn test_update_dry_run_no_changes_written() {
    let old_snap = tempfile::tempdir().unwrap();
    let new_snap = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    // Old snapshot and project have same content (user hasn't changed anything)
    std::fs::write(old_snap.path().join("file.txt"), "old content").unwrap();
    std::fs::write(project.path().join("file.txt"), "old content").unwrap();

    // New snapshot has updated content from template
    std::fs::write(new_snap.path().join("file.txt"), "new content").unwrap();

    // New file only in new snapshot (would be added)
    std::fs::write(new_snap.path().join("added.txt"), "brand new file").unwrap();

    // File in old snapshot + project but removed from new snapshot
    std::fs::write(old_snap.path().join("removed.txt"), "to be removed").unwrap();
    std::fs::write(project.path().join("removed.txt"), "to be removed").unwrap();

    // Conflict: both user and template changed
    std::fs::write(old_snap.path().join("conflict.txt"), "original").unwrap();
    std::fs::write(new_snap.path().join("conflict.txt"), "template changed").unwrap();
    std::fs::write(project.path().join("conflict.txt"), "user changed").unwrap();

    // Run three-way merge to get the report (this is what update_project does)
    let merge_results = three_way_merge(project.path(), old_snap.path(), new_snap.path()).unwrap();

    // Verify the merge detected changes
    let has_update = merge_results
        .iter()
        .any(|r| r.action == MergeAction::UpdateFromTemplate);
    let has_add = merge_results
        .iter()
        .any(|r| r.action == MergeAction::AddFromTemplate);
    let has_remove = merge_results
        .iter()
        .any(|r| r.action == MergeAction::MarkForRemoval);
    let has_conflict = merge_results
        .iter()
        .any(|r| r.action == MergeAction::Conflict);
    assert!(has_update, "should detect updated file");
    assert!(has_add, "should detect added file");
    assert!(has_remove, "should detect removed file");
    assert!(has_conflict, "should detect conflict");

    // Simulate dry_run: do NOT call apply_merge.
    // In update_project with dry_run=true, we return the report without
    // calling apply_merge() or write_answers_with_source().

    // Verify project files were NOT modified
    let file_content = std::fs::read_to_string(project.path().join("file.txt")).unwrap();
    assert_eq!(
        file_content, "old content",
        "file.txt should not have been updated"
    );

    // Verify no new files were added
    assert!(
        !project.path().join("added.txt").exists(),
        "added.txt should not exist in project (dry run)"
    );

    // Verify removed file still exists
    assert!(
        project.path().join("removed.txt").exists(),
        "removed.txt should still exist (dry run)"
    );

    // Verify no .rej files were created
    assert!(
        !project.path().join("conflict.txt.rej").exists(),
        "conflict.txt.rej should not exist (dry run)"
    );

    // Verify no .removing files were created
    assert!(
        !project.path().join("removed.txt.removing").exists(),
        "removed.txt.removing should not exist (dry run)"
    );

    // Verify conflict file was not changed
    let conflict_content = std::fs::read_to_string(project.path().join("conflict.txt")).unwrap();
    assert_eq!(
        conflict_content, "user changed",
        "conflict.txt should not have been modified"
    );

    // Now verify that if we DO call apply_merge, files ARE modified
    // (proving the dry_run skip is the only difference)
    apply_merge(
        project.path(),
        new_snap.path(),
        old_snap.path(),
        &merge_results,
    )
    .unwrap();

    let updated_content = std::fs::read_to_string(project.path().join("file.txt")).unwrap();
    assert_eq!(
        updated_content, "new content",
        "file.txt should be updated after apply_merge"
    );
    assert!(
        project.path().join("added.txt").exists(),
        "added.txt should exist after apply_merge"
    );
    assert!(
        project.path().join("conflict.txt.rej").exists(),
        "conflict.txt.rej should exist after apply_merge"
    );
}
