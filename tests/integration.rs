use std::collections::BTreeMap;
use std::path::PathBuf;

use diecut::adapter;
use diecut::config::load_config;
use diecut::prompt::PromptOptions;
use diecut::render::{build_context, execute_plan, plan_render, walk_and_render};
use diecut::template::source::{resolve_source, resolve_source_full};

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
