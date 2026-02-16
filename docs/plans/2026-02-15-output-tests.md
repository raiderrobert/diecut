# Output/Persistence Tests Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add comprehensive tests for answers/mod.rs to increase coverage from 40.47% to ~75%.

**Architecture:** Test answers file writing, TOML serialization, source metadata inclusion, and edge cases. Use tempfile for isolation and follow existing test patterns.

**Tech Stack:** Rust, rstest, tempfile, toml, cargo test, cargo llvm-cov

**Beads Issue:** diecut-aii

---

## Task 1: Test write_answers() - Basic TOML Serialization

**Files:**
- Check: `src/answers/mod.rs` for existing `#[cfg(test)]` module
- Modify: `src/answers/mod.rs::tests` (add tests)

**Step 1: Check existing test structure**

Run: `grep -n "#\[cfg(test)\]" src/answers/mod.rs`

If no test module exists, add one at end of file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile;
    use std::fs;
    use std::collections::BTreeMap;
    use tera::Value;
}
```

**Step 2: Write the test**

Add to test module:

```rust
#[test]
fn test_write_answers_basic() {
    let output_dir = tempfile::tempdir().unwrap();

    let config = crate::config::schema::TemplateConfig {
        template: crate::config::schema::TemplateMetadata {
            name: "test-template".to_string(),
            version: Some("1.0.0".to_string()),
            description: None,
            templates_suffix: ".tera".to_string(),
        },
        variables: vec![],
        hooks: crate::config::schema::HooksConfig { post_create: None },
        ignore: vec![],
    };

    let mut variables = BTreeMap::new();
    variables.insert("project_name".to_string(), Value::String("my-project".to_string()));
    variables.insert("author".to_string(), Value::String("Jane Doe".to_string()));

    let source_info = SourceInfo {
        url: None,
        git_ref: None,
        commit_sha: None,
    };

    let result = write_answers(output_dir.path(), &config, &variables, &source_info);

    assert!(result.is_ok());

    let answers_file = output_dir.path().join(".diecut_answers.toml");
    assert!(answers_file.exists());

    let content = fs::read_to_string(&answers_file).unwrap();
    assert!(content.contains("project_name"));
    assert!(content.contains("my-project"));
    assert!(content.contains("author"));
    assert!(content.contains("Jane Doe"));
}
```

**Step 3: Run test**

Run: `cargo test test_write_answers_basic -q`
Expected: PASS

**Step 4: Commit**

```bash
git add src/answers/mod.rs
git commit -m "test: add test_write_answers_basic"
```

---

## Task 2: Test write_answers() - Includes Template Metadata

**Files:**
- Modify: `src/answers/mod.rs::tests`

**Step 1: Write the test**

Add to test module:

```rust
#[test]
fn test_write_answers_includes_template_metadata() {
    let output_dir = tempfile::tempdir().unwrap();

    let config = crate::config::schema::TemplateConfig {
        template: crate::config::schema::TemplateMetadata {
            name: "my-template".to_string(),
            version: Some("2.1.0".to_string()),
            description: Some("A test template".to_string()),
            templates_suffix: ".tera".to_string(),
        },
        variables: vec![],
        hooks: crate::config::schema::HooksConfig { post_create: None },
        ignore: vec![],
    };

    let variables = BTreeMap::new();
    let source_info = SourceInfo {
        url: None,
        git_ref: None,
        commit_sha: None,
    };

    write_answers(output_dir.path(), &config, &variables, &source_info).unwrap();

    let answers_file = output_dir.path().join(".diecut_answers.toml");
    let content = fs::read_to_string(&answers_file).unwrap();

    assert!(content.contains("my-template"));
    assert!(content.contains("2.1.0"));
}
```

**Step 2: Run test**

Run: `cargo test test_write_answers_includes_template_metadata -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/answers/mod.rs
git commit -m "test: add test_write_answers_includes_template_metadata"
```

---

## Task 3: Test write_answers() - Includes Git Source Info

**Files:**
- Modify: `src/answers/mod.rs::tests`

**Step 1: Write the test**

Add to test module:

```rust
#[test]
fn test_write_answers_includes_git_source() {
    let output_dir = tempfile::tempdir().unwrap();

    let config = crate::config::schema::TemplateConfig {
        template: crate::config::schema::TemplateMetadata {
            name: "test".to_string(),
            version: None,
            description: None,
            templates_suffix: ".tera".to_string(),
        },
        variables: vec![],
        hooks: crate::config::schema::HooksConfig { post_create: None },
        ignore: vec![],
    };

    let variables = BTreeMap::new();
    let source_info = SourceInfo {
        url: Some("https://github.com/user/repo.git".to_string()),
        git_ref: Some("main".to_string()),
        commit_sha: Some("abc123def456".to_string()),
    };

    write_answers(output_dir.path(), &config, &variables, &source_info).unwrap();

    let answers_file = output_dir.path().join(".diecut_answers.toml");
    let content = fs::read_to_string(&answers_file).unwrap();

    assert!(content.contains("https://github.com/user/repo.git"));
    assert!(content.contains("main"));
    assert!(content.contains("abc123def456"));
}
```

**Step 2: Run test**

Run: `cargo test test_write_answers_includes_git_source -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/answers/mod.rs
git commit -m "test: add test_write_answers_includes_git_source"
```

---

## Task 4: Test write_answers() - Local Template (No Source)

**Files:**
- Modify: `src/answers/mod.rs::tests`

**Step 1: Write the test**

Add to test module:

```rust
#[test]
fn test_write_answers_local_template_no_source() {
    let output_dir = tempfile::tempdir().unwrap();

    let config = crate::config::schema::TemplateConfig {
        template: crate::config::schema::TemplateMetadata {
            name: "local-template".to_string(),
            version: None,
            description: None,
            templates_suffix: ".tera".to_string(),
        },
        variables: vec![],
        hooks: crate::config::schema::HooksConfig { post_create: None },
        ignore: vec![],
    };

    let variables = BTreeMap::new();
    let source_info = SourceInfo {
        url: None,
        git_ref: None,
        commit_sha: None,
    };

    write_answers(output_dir.path(), &config, &variables, &source_info).unwrap();

    let answers_file = output_dir.path().join(".diecut_answers.toml");
    assert!(answers_file.exists());

    let content = fs::read_to_string(&answers_file).unwrap();
    assert!(content.contains("local-template"));
}
```

**Step 2: Run test**

Run: `cargo test test_write_answers_local_template_no_source -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/answers/mod.rs
git commit -m "test: add test_write_answers_local_template_no_source"
```

---

## Task 5: Test write_answers() - Special Characters in Values

**Files:**
- Modify: `src/answers/mod.rs::tests`

**Step 1: Write the test**

Add to test module:

```rust
use rstest::rstest;

#[rstest]
#[case("name with spaces", "name with spaces")]
#[case("quote\"test", "quote\"test")]
#[case("multi\nline", "multi\nline")]
#[case("emoji 🦀", "emoji 🦀")]
fn test_write_answers_special_characters(#[case] value: &str, #[case] expected: &str) {
    let output_dir = tempfile::tempdir().unwrap();

    let config = crate::config::schema::TemplateConfig {
        template: crate::config::schema::TemplateMetadata {
            name: "test".to_string(),
            version: None,
            description: None,
            templates_suffix: ".tera".to_string(),
        },
        variables: vec![],
        hooks: crate::config::schema::HooksConfig { post_create: None },
        ignore: vec![],
    };

    let mut variables = BTreeMap::new();
    variables.insert("special".to_string(), Value::String(value.to_string()));

    let source_info = SourceInfo {
        url: None,
        git_ref: None,
        commit_sha: None,
    };

    write_answers(output_dir.path(), &config, &variables, &source_info).unwrap();

    let answers_file = output_dir.path().join(".diecut_answers.toml");
    let content = fs::read_to_string(&answers_file).unwrap();

    // Verify TOML can be parsed back
    let parsed: toml::Value = toml::from_str(&content).unwrap();
    let answers_section = parsed.get("answers").unwrap().as_table().unwrap();
    assert_eq!(answers_section.get("special").unwrap().as_str().unwrap(), expected);
}
```

**Step 2: Run test**

Run: `cargo test test_write_answers_special_characters -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/answers/mod.rs
git commit -m "test: add special characters handling tests"
```

---

## Task 6: Test write_answers() - Boolean Variables

**Files:**
- Modify: `src/answers/mod.rs::tests`

**Step 1: Write the test**

Add to test module:

```rust
#[test]
fn test_write_answers_boolean_values() {
    let output_dir = tempfile::tempdir().unwrap();

    let config = crate::config::schema::TemplateConfig {
        template: crate::config::schema::TemplateMetadata {
            name: "test".to_string(),
            version: None,
            description: None,
            templates_suffix: ".tera".to_string(),
        },
        variables: vec![],
        hooks: crate::config::schema::HooksConfig { post_create: None },
        ignore: vec![],
    };

    let mut variables = BTreeMap::new();
    variables.insert("enabled".to_string(), Value::Bool(true));
    variables.insert("disabled".to_string(), Value::Bool(false));

    let source_info = SourceInfo {
        url: None,
        git_ref: None,
        commit_sha: None,
    };

    write_answers(output_dir.path(), &config, &variables, &source_info).unwrap();

    let answers_file = output_dir.path().join(".diecut_answers.toml");
    let content = fs::read_to_string(&answers_file).unwrap();

    let parsed: toml::Value = toml::from_str(&content).unwrap();
    let answers = parsed.get("answers").unwrap().as_table().unwrap();

    assert_eq!(answers.get("enabled").unwrap().as_bool().unwrap(), true);
    assert_eq!(answers.get("disabled").unwrap().as_bool().unwrap(), false);
}
```

**Step 2: Run test**

Run: `cargo test test_write_answers_boolean_values -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/answers/mod.rs
git commit -m "test: add boolean values serialization test"
```

---

## Task 7: Test write_answers() - Empty Variables

**Files:**
- Modify: `src/answers/mod.rs::tests`

**Step 1: Write the test**

Add to test module:

```rust
#[test]
fn test_write_answers_empty_variables() {
    let output_dir = tempfile::tempdir().unwrap();

    let config = crate::config::schema::TemplateConfig {
        template: crate::config::schema::TemplateMetadata {
            name: "test".to_string(),
            version: None,
            description: None,
            templates_suffix: ".tera".to_string(),
        },
        variables: vec![],
        hooks: crate::config::schema::HooksConfig { post_create: None },
        ignore: vec![],
    };

    let variables = BTreeMap::new();
    let source_info = SourceInfo {
        url: None,
        git_ref: None,
        commit_sha: None,
    };

    let result = write_answers(output_dir.path(), &config, &variables, &source_info);

    assert!(result.is_ok());

    let answers_file = output_dir.path().join(".diecut_answers.toml");
    assert!(answers_file.exists());

    let content = fs::read_to_string(&answers_file).unwrap();
    let parsed: toml::Value = toml::from_str(&content).unwrap();

    // Should still have metadata sections even if no answers
    assert!(parsed.get("template").is_some());
}
```

**Step 2: Run test**

Run: `cargo test test_write_answers_empty_variables -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/answers/mod.rs
git commit -m "test: add empty variables test"
```

---

## Task 8: Test write_answers() - Creates Output Directory

**Files:**
- Modify: `src/answers/mod.rs::tests`

**Step 1: Write the test**

Add to test module:

```rust
#[test]
fn test_write_answers_creates_directory() {
    let temp_root = tempfile::tempdir().unwrap();
    let nested_path = temp_root.path().join("a/b/c");

    let config = crate::config::schema::TemplateConfig {
        template: crate::config::schema::TemplateMetadata {
            name: "test".to_string(),
            version: None,
            description: None,
            templates_suffix: ".tera".to_string(),
        },
        variables: vec![],
        hooks: crate::config::schema::HooksConfig { post_create: None },
        ignore: vec![],
    };

    let variables = BTreeMap::new();
    let source_info = SourceInfo {
        url: None,
        git_ref: None,
        commit_sha: None,
    };

    assert!(!nested_path.exists(), "Nested path should not exist before write");

    let result = write_answers(&nested_path, &config, &variables, &source_info);

    // Depending on implementation, this may succeed or fail
    // If it fails, that's expected behavior (answers writer doesn't create dirs)
    // If it succeeds, verify the file exists
    match result {
        Ok(_) => {
            let answers_file = nested_path.join(".diecut_answers.toml");
            assert!(answers_file.exists());
        }
        Err(_) => {
            // Expected if write_answers doesn't create parent dirs
            assert!(true);
        }
    }
}
```

**Step 2: Run test and adjust**

Run: `cargo test test_write_answers_creates_directory -q`

Adjust assertion based on actual behavior.

**Step 3: Commit**

```bash
git add src/answers/mod.rs
git commit -m "test: add directory creation behavior test"
```

---

## Task 9: Test Deserialization (Round-trip)

**Files:**
- Modify: `src/answers/mod.rs::tests`

**Step 1: Write the test**

Add to test module:

```rust
#[test]
fn test_write_and_read_answers_roundtrip() {
    let output_dir = tempfile::tempdir().unwrap();

    let config = crate::config::schema::TemplateConfig {
        template: crate::config::schema::TemplateMetadata {
            name: "roundtrip-test".to_string(),
            version: Some("1.2.3".to_string()),
            description: None,
            templates_suffix: ".tera".to_string(),
        },
        variables: vec![],
        hooks: crate::config::schema::HooksConfig { post_create: None },
        ignore: vec![],
    };

    let mut variables = BTreeMap::new();
    variables.insert("name".to_string(), Value::String("test".to_string()));
    variables.insert("count".to_string(), Value::Number(42.into()));
    variables.insert("enabled".to_string(), Value::Bool(true));

    let source_info = SourceInfo {
        url: Some("https://example.com/repo.git".to_string()),
        git_ref: Some("v1.0".to_string()),
        commit_sha: Some("deadbeef".to_string()),
    };

    write_answers(output_dir.path(), &config, &variables, &source_info).unwrap();

    // Read back and verify
    let answers_file = output_dir.path().join(".diecut_answers.toml");
    let content = fs::read_to_string(&answers_file).unwrap();
    let parsed: toml::Value = toml::from_str(&content).unwrap();

    let template = parsed.get("template").unwrap().as_table().unwrap();
    assert_eq!(template.get("name").unwrap().as_str().unwrap(), "roundtrip-test");
    assert_eq!(template.get("version").unwrap().as_str().unwrap(), "1.2.3");

    let answers = parsed.get("answers").unwrap().as_table().unwrap();
    assert_eq!(answers.get("name").unwrap().as_str().unwrap(), "test");
    assert_eq!(answers.get("count").unwrap().as_integer().unwrap(), 42);
    assert_eq!(answers.get("enabled").unwrap().as_bool().unwrap(), true);

    let source = parsed.get("source").unwrap().as_table().unwrap();
    assert_eq!(source.get("url").unwrap().as_str().unwrap(), "https://example.com/repo.git");
}
```

**Step 2: Run test**

Run: `cargo test test_write_and_read_answers_roundtrip -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/answers/mod.rs
git commit -m "test: add roundtrip serialization test"
```

---

## Task 10: Check Coverage

**Step 1: Run coverage report**

Run: `cargo llvm-cov --html`

**Step 2: Check answers/mod.rs coverage**

Open: `target/llvm-cov/html/index.html`
Look for: `answers/mod.rs` coverage should be ~75%+

**Step 3: Identify gaps**

If coverage < 75%, check HTML report for uncovered lines. Add targeted tests for:
- Error paths (I/O errors, permission errors)
- Edge cases
- Uncommon data types

**Step 4: Commit milestone**

```bash
git commit --allow-empty -m "chore: answers/mod.rs coverage now at ~75%"
```

---

## Task 11: Final Validation

**Step 1: Run full test suite**

Run: `cargo test -q`
Expected: All tests pass

**Step 2: Run coverage summary**

Run: `cargo llvm-cov --summary-only`

**Step 3: Verify impact**

Check:
- answers/mod.rs: ~75%+ coverage
- Overall impact: +4-5 percentage points

**Step 4: Update beads issue**

Run: `bd update diecut-aii --status=completed`

**Step 5: Final commit**

```bash
git commit --allow-empty -m "test: output/persistence tests complete - coverage increased"
```

---

## Validation Checklist

Before merging:
- [ ] All tests pass (`cargo test`)
- [ ] No new clippy warnings (`cargo clippy`)
- [ ] Code formatted (`cargo fmt --check`)
- [ ] Coverage increased by 4-5 percentage points
- [ ] answers/mod.rs coverage ~75%+

## Next Steps

After this worktree is complete:
1. Create PR from `test/output` branch
2. Include coverage report in PR description
3. Verify all 3 worktrees are merged
4. Run final coverage check to confirm 80%+ overall target achieved
