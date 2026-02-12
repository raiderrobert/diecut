use std::collections::BTreeMap;
use std::path::PathBuf;

use diecut_core::adapter::migrate::{execute_migration, plan_migration, FileOp};
use diecut_core::adapter::{self, TemplateFormat};
use diecut_core::config::load_config;
use diecut_core::prompt::PromptOptions;
use diecut_core::render::{build_context, build_context_with_namespace, walk_and_render};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
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
    let config: diecut_core::config::schema::TemplateConfig = toml::from_str(toml_str).unwrap();
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
    let variables = diecut_core::prompt::collect_variables(&config, &options).unwrap();

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
    let variables = diecut_core::prompt::collect_variables(&config, &options).unwrap();

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

    diecut_core::answers::write_answers(output_dir.path(), &resolved.config, &variables).unwrap();

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
    let variables = diecut_core::prompt::collect_variables(&resolved.config, &options).unwrap();

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
    let variables = diecut_core::prompt::collect_variables(&resolved.config, &options).unwrap();

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
        diecut_core::config::variable::VariableType::Select
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
