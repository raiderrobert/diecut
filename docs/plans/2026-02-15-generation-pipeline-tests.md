# Generation Pipeline Tests Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add comprehensive tests for lib.rs and render/file.rs to increase coverage from 28.24%/53.19% to ~75% each.

**Architecture:** Test the main API functions (plan_generation, execute_generation, generate) and file rendering logic using TDD. Use tempfile for isolation, rstest for parameterized tests, and follow existing test patterns.

**Tech Stack:** Rust, rstest, tempfile, cargo test, cargo llvm-cov

**Beads Issue:** diecut-636

---

## Task 1: Test plan_generation() - Valid Local Template

**Files:**
- Modify: `src/lib.rs` (add #[cfg(test)] module at end)
- Test: `src/lib.rs::tests::test_plan_generation_local_template`

**Step 1: Write the failing test**

Add at end of `src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile;
    use std::fs;

    fn create_minimal_template(dir: &std::path::Path) {
        let config = r#"
[template]
name = "test-template"
version = "1.0.0"
templates_suffix = ".tera"

[[variables]]
name = "project_name"
type = "text"
default = "my-project"
"#;
        fs::write(dir.join("diecut.toml"), config).unwrap();
        fs::create_dir_all(dir.join("template")).unwrap();
        fs::write(dir.join("template/README.md.tera"), "# {{ project_name }}").unwrap();
    }

    #[test]
    fn test_plan_generation_local_template() {
        let template_dir = tempfile::tempdir().unwrap();
        create_minimal_template(template_dir.path());

        let output_dir = tempfile::tempdir().unwrap();

        let options = GenerateOptions {
            template: template_dir.path().display().to_string(),
            output: Some(output_dir.path().display().to_string()),
            data: vec![("project_name".to_string(), "test-proj".to_string())],
            defaults: false,
            overwrite: false,
            no_hooks: true,
        };

        let plan = plan_generation(options).unwrap();

        assert_eq!(plan.config.template.name, "test-template");
        assert!(plan.render_plan.files.len() > 0);
        assert_eq!(plan.variables.get("project_name").unwrap(), "test-proj");
    }
}
```

**Step 2: Run test to verify it passes**

Run: `cargo test test_plan_generation_local_template -q`
Expected: PASS (lib.rs already has implementation)

**Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "test: add test_plan_generation_local_template"
```

---

## Task 2: Test plan_generation() - Missing Template Directory

**Files:**
- Modify: `src/lib.rs::tests`

**Step 1: Write the test**

Add to `src/lib.rs::tests` module:

```rust
#[test]
fn test_plan_generation_template_missing() {
    let options = GenerateOptions {
        template: "/nonexistent/path/to/template".to_string(),
        output: None,
        data: vec![],
        defaults: true,
        overwrite: false,
        no_hooks: true,
    };

    let result = plan_generation(options);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, DicecutError::TemplateDirectoryMissing { .. }));
}
```

**Step 2: Run test**

Run: `cargo test test_plan_generation_template_missing -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "test: add test_plan_generation_template_missing"
```

---

## Task 3: Test plan_generation() - Output Directory Exists (No Overwrite)

**Files:**
- Modify: `src/lib.rs::tests`

**Step 1: Write the test**

Add to `src/lib.rs::tests`:

```rust
#[test]
fn test_plan_generation_output_exists_no_overwrite() {
    let template_dir = tempfile::tempdir().unwrap();
    create_minimal_template(template_dir.path());

    let output_dir = tempfile::tempdir().unwrap();
    fs::write(output_dir.path().join("existing.txt"), "exists").unwrap();

    let options = GenerateOptions {
        template: template_dir.path().display().to_string(),
        output: Some(output_dir.path().display().to_string()),
        data: vec![],
        defaults: true,
        overwrite: false,
        no_hooks: true,
    };

    let result = plan_generation(options);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, DicecutError::OutputExists { .. }));
}
```

**Step 2: Run test**

Run: `cargo test test_plan_generation_output_exists_no_overwrite -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "test: add test_plan_generation_output_exists_no_overwrite"
```

---

## Task 4: Test plan_generation() - Output Directory Exists (With Overwrite)

**Files:**
- Modify: `src/lib.rs::tests`

**Step 1: Write the test**

Add to `src/lib.rs::tests`:

```rust
#[test]
fn test_plan_generation_output_exists_with_overwrite() {
    let template_dir = tempfile::tempdir().unwrap();
    create_minimal_template(template_dir.path());

    let output_dir = tempfile::tempdir().unwrap();
    fs::write(output_dir.path().join("existing.txt"), "exists").unwrap();

    let options = GenerateOptions {
        template: template_dir.path().display().to_string(),
        output: Some(output_dir.path().display().to_string()),
        data: vec![],
        defaults: true,
        overwrite: true,
        no_hooks: true,
    };

    let plan = plan_generation(options);

    assert!(plan.is_ok());
}
```

**Step 2: Run test**

Run: `cargo test test_plan_generation_output_exists_with_overwrite -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "test: add test_plan_generation_output_exists_with_overwrite"
```

---

## Task 5: Test execute_generation() - Creates Output Directory

**Files:**
- Modify: `src/lib.rs::tests`

**Step 1: Write the test**

Add to `src/lib.rs::tests`:

```rust
#[test]
fn test_execute_generation_creates_output_dir() {
    let template_dir = tempfile::tempdir().unwrap();
    create_minimal_template(template_dir.path());

    let output_parent = tempfile::tempdir().unwrap();
    let output_path = output_parent.path().join("new_project");

    let options = GenerateOptions {
        template: template_dir.path().display().to_string(),
        output: Some(output_path.display().to_string()),
        data: vec![("project_name".to_string(), "test".to_string())],
        defaults: false,
        overwrite: false,
        no_hooks: true,
    };

    let plan = plan_generation(options).unwrap();

    assert!(!output_path.exists(), "Output dir should not exist before execution");

    let result = execute_generation(plan);

    assert!(result.is_ok());
    assert!(output_path.exists(), "Output dir should exist after execution");
}
```

**Step 2: Run test**

Run: `cargo test test_execute_generation_creates_output_dir -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "test: add test_execute_generation_creates_output_dir"
```

---

## Task 6: Test execute_generation() - Writes Answers File

**Files:**
- Modify: `src/lib.rs::tests`

**Step 1: Write the test**

Add to `src/lib.rs::tests`:

```rust
#[test]
fn test_execute_generation_writes_answers() {
    let template_dir = tempfile::tempdir().unwrap();
    create_minimal_template(template_dir.path());

    let output_dir = tempfile::tempdir().unwrap();

    let options = GenerateOptions {
        template: template_dir.path().display().to_string(),
        output: Some(output_dir.path().display().to_string()),
        data: vec![("project_name".to_string(), "test-project".to_string())],
        defaults: false,
        overwrite: true,
        no_hooks: true,
    };

    let plan = plan_generation(options).unwrap();
    let result = execute_generation(plan).unwrap();

    let answers_file = output_dir.path().join(".diecut_answers.toml");
    assert!(answers_file.exists(), "Answers file should exist");

    let contents = fs::read_to_string(&answers_file).unwrap();
    assert!(contents.contains("project_name"));
    assert!(contents.contains("test-project"));
}
```

**Step 2: Run test**

Run: `cargo test test_execute_generation_writes_answers -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "test: add test_execute_generation_writes_answers"
```

---

## Task 7: Test execute_generation() - Respects no_hooks Flag

**Files:**
- Modify: `src/lib.rs::tests`

**Step 1: Write the test**

Add to `src/lib.rs::tests`:

```rust
#[test]
fn test_execute_generation_respects_no_hooks() {
    let template_dir = tempfile::tempdir().unwrap();

    let config = r#"
[template]
name = "test-with-hooks"
version = "1.0.0"
templates_suffix = ".tera"

[hooks]
post_create = "touch hook_ran.txt"

[[variables]]
name = "name"
type = "text"
default = "test"
"#;
    fs::write(template_dir.path().join("diecut.toml"), config).unwrap();
    fs::create_dir_all(template_dir.path().join("template")).unwrap();
    fs::write(template_dir.path().join("template/README.md"), "test").unwrap();

    let output_dir = tempfile::tempdir().unwrap();

    let options = GenerateOptions {
        template: template_dir.path().display().to_string(),
        output: Some(output_dir.path().display().to_string()),
        data: vec![],
        defaults: true,
        overwrite: true,
        no_hooks: true,
    };

    let plan = plan_generation(options).unwrap();
    execute_generation(plan).unwrap();

    let hook_file = output_dir.path().join("hook_ran.txt");
    assert!(!hook_file.exists(), "Hook should not run when no_hooks=true");
}
```

**Step 2: Run test**

Run: `cargo test test_execute_generation_respects_no_hooks -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "test: add test_execute_generation_respects_no_hooks"
```

---

## Task 8: Test generate() - End-to-End Success

**Files:**
- Modify: `src/lib.rs::tests`

**Step 1: Write the test**

Add to `src/lib.rs::tests`:

```rust
#[test]
fn test_generate_end_to_end() {
    let template_dir = tempfile::tempdir().unwrap();
    create_minimal_template(template_dir.path());

    let output_dir = tempfile::tempdir().unwrap();

    let options = GenerateOptions {
        template: template_dir.path().display().to_string(),
        output: Some(output_dir.path().display().to_string()),
        data: vec![("project_name".to_string(), "my-proj".to_string())],
        defaults: false,
        overwrite: true,
        no_hooks: true,
    };

    let result = generate(options).unwrap();

    assert!(result.files_created.len() > 0);
    assert!(output_dir.path().join(".diecut_answers.toml").exists());

    // Verify rendered file exists
    let readme = output_dir.path().join("README.md");
    assert!(readme.exists());
    let content = fs::read_to_string(readme).unwrap();
    assert!(content.contains("my-proj"));
}
```

**Step 2: Run test**

Run: `cargo test test_generate_end_to_end -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "test: add test_generate_end_to_end"
```

---

## Task 9: Check Coverage for lib.rs

**Step 1: Run coverage report**

Run: `cargo llvm-cov --html`

**Step 2: Check lib.rs coverage**

Open: `target/llvm-cov/html/index.html`
Look for: `lib.rs` coverage should be ~75%+

**Step 3: Identify remaining gaps**

If coverage < 75%, check the HTML report to see which lines are uncovered. Add targeted tests for those specific code paths.

**Step 4: Commit coverage milestone**

```bash
git commit --allow-empty -m "chore: lib.rs coverage now at ~75%"
```

---

## Task 10: Test render/file.rs - File Type Detection

**Files:**
- Modify: `src/render/file.rs` (add #[cfg(test)] module at end if not exists)

**Step 1: Write the test**

Add at end of `src/render/file.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile;
    use std::fs;

    #[test]
    fn test_is_likely_text_with_text_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "Hello, world!").unwrap();

        assert!(is_likely_text(&file).unwrap());
    }

    #[test]
    fn test_is_likely_text_with_binary_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.bin");
        fs::write(&file, &[0xFF, 0xFE, 0x00, 0x01]).unwrap();

        assert!(!is_likely_text(&file).unwrap());
    }
}
```

**Step 2: Run tests**

Run: `cargo test -p diecut --lib render::file::tests -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/render/file.rs
git commit -m "test: add file type detection tests"
```

---

## Task 11: Test render/file.rs - Error Cases

**Files:**
- Modify: `src/render/file.rs::tests`

**Step 1: Write the test**

Add to `src/render/file.rs::tests`:

```rust
#[test]
fn test_is_likely_text_nonexistent_file() {
    let result = is_likely_text(&std::path::PathBuf::from("/nonexistent/file.txt"));
    assert!(result.is_err());
}
```

**Step 2: Run test**

Run: `cargo test test_is_likely_text_nonexistent_file -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/render/file.rs
git commit -m "test: add is_likely_text error case"
```

---

## Task 12: Final Coverage Check

**Step 1: Run full test suite**

Run: `cargo test -q`
Expected: All tests pass

**Step 2: Run coverage report**

Run: `cargo llvm-cov --summary-only`

**Step 3: Verify targets met**

Check:
- lib.rs: ~75%+ coverage
- render/file.rs: ~75%+ coverage
- Overall impact: +8-10 percentage points

**Step 4: Update beads issue**

Run: `bd update diecut-636 --status=completed`

**Step 5: Final commit**

```bash
git commit --allow-empty -m "test: generation pipeline tests complete - coverage increased"
```

---

## Validation Checklist

Before merging:
- [ ] All tests pass (`cargo test`)
- [ ] No new clippy warnings (`cargo clippy`)
- [ ] Code formatted (`cargo fmt --check`)
- [ ] Coverage increased by 8-10 percentage points
- [ ] lib.rs coverage ~75%+
- [ ] render/file.rs coverage ~75%+

## Next Steps

After this worktree is complete:
1. Create PR from `test/generation-pipeline` branch
2. Include coverage report in PR description
3. Move to next worktree (user-input-tests or output-tests)
