# User Input Tests Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add comprehensive tests for prompt/engine.rs to increase coverage from 26.61% to ~65%.

**Architecture:** Test variable collection, data overrides, defaults mode, and validation logic. Focus on behavior, not I/O mocking. Use existing test patterns with rstest for parameterized cases.

**Tech Stack:** Rust, rstest, tempfile, cargo test, cargo llvm-cov

**Beads Issue:** diecut-3kz

---

## Task 1: Test collect_variables() - Text Variable with Default

**Files:**
- Check: `src/prompt/engine.rs` (likely has existing tests, find `#[cfg(test)]` module)
- Modify: `src/prompt/engine.rs::tests` (add tests)

**Step 1: Check existing test structure**

Run: `grep -n "#\[cfg(test)\]" src/prompt/engine.rs`

If no test module exists, add one at end:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::*;
    use std::collections::BTreeMap;
}
```

**Step 2: Write the test**

Add to test module:

```rust
#[test]
fn test_collect_variables_text_with_default() {
    let config = TemplateConfig {
        template: TemplateMetadata {
            name: "test".to_string(),
            version: None,
            description: None,
            templates_suffix: ".tera".to_string(),
        },
        variables: vec![
            Variable {
                name: "project_name".to_string(),
                var_type: VariableType::Text,
                description: None,
                default: Some(tera::Value::String("my-project".to_string())),
                choices: None,
                computed: None,
            },
        ],
        hooks: HooksConfig { post_create: None },
        ignore: vec![],
    };

    let options = PromptOptions {
        data_overrides: BTreeMap::new(),
        use_defaults: true,
    };

    let result = collect_variables(&config, &options).unwrap();

    assert_eq!(result.get("project_name").unwrap(), "my-project");
}
```

**Step 3: Run test**

Run: `cargo test test_collect_variables_text_with_default -q`
Expected: PASS

**Step 4: Commit**

```bash
git add src/prompt/engine.rs
git commit -m "test: add test_collect_variables_text_with_default"
```

---

## Task 2: Test collect_variables() - Data Override

**Files:**
- Modify: `src/prompt/engine.rs::tests`

**Step 1: Write the test**

Add to test module:

```rust
#[test]
fn test_collect_variables_data_override() {
    let config = TemplateConfig {
        template: TemplateMetadata {
            name: "test".to_string(),
            version: None,
            description: None,
            templates_suffix: ".tera".to_string(),
        },
        variables: vec![
            Variable {
                name: "project_name".to_string(),
                var_type: VariableType::Text,
                description: None,
                default: Some(tera::Value::String("default-name".to_string())),
                choices: None,
                computed: None,
            },
        ],
        hooks: HooksConfig { post_create: None },
        ignore: vec![],
    };

    let mut overrides = BTreeMap::new();
    overrides.insert("project_name".to_string(), "overridden-name".to_string());

    let options = PromptOptions {
        data_overrides: overrides,
        use_defaults: false,
    };

    let result = collect_variables(&config, &options).unwrap();

    assert_eq!(result.get("project_name").unwrap(), "overridden-name");
}
```

**Step 2: Run test**

Run: `cargo test test_collect_variables_data_override -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/prompt/engine.rs
git commit -m "test: add test_collect_variables_data_override"
```

---

## Task 3: Test collect_variables() - Select Variable

**Files:**
- Modify: `src/prompt/engine.rs::tests`

**Step 1: Write the test**

Add to test module:

```rust
#[test]
fn test_collect_variables_select_with_default() {
    let config = TemplateConfig {
        template: TemplateMetadata {
            name: "test".to_string(),
            version: None,
            description: None,
            templates_suffix: ".tera".to_string(),
        },
        variables: vec![
            Variable {
                name: "license".to_string(),
                var_type: VariableType::Select,
                description: None,
                default: Some(tera::Value::String("MIT".to_string())),
                choices: Some(vec!["MIT".to_string(), "Apache-2.0".to_string(), "GPL-3.0".to_string()]),
                computed: None,
            },
        ],
        hooks: HooksConfig { post_create: None },
        ignore: vec![],
    };

    let options = PromptOptions {
        data_overrides: BTreeMap::new(),
        use_defaults: true,
    };

    let result = collect_variables(&config, &options).unwrap();

    assert_eq!(result.get("license").unwrap(), "MIT");
}
```

**Step 2: Run test**

Run: `cargo test test_collect_variables_select_with_default -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/prompt/engine.rs
git commit -m "test: add test_collect_variables_select_with_default"
```

---

## Task 4: Test collect_variables() - Boolean Variable

**Files:**
- Modify: `src/prompt/engine.rs::tests`

**Step 1: Write the test**

Add to test module:

```rust
#[test]
fn test_collect_variables_boolean() {
    let config = TemplateConfig {
        template: TemplateMetadata {
            name: "test".to_string(),
            version: None,
            description: None,
            templates_suffix: ".tera".to_string(),
        },
        variables: vec![
            Variable {
                name: "use_docker".to_string(),
                var_type: VariableType::Boolean,
                description: None,
                default: Some(tera::Value::Bool(true)),
                choices: None,
                computed: None,
            },
        ],
        hooks: HooksConfig { post_create: None },
        ignore: vec![],
    };

    let options = PromptOptions {
        data_overrides: BTreeMap::new(),
        use_defaults: true,
    };

    let result = collect_variables(&config, &options).unwrap();

    assert_eq!(result.get("use_docker").unwrap(), &tera::Value::Bool(true));
}
```

**Step 2: Run test**

Run: `cargo test test_collect_variables_boolean -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/prompt/engine.rs
git commit -m "test: add test_collect_variables_boolean"
```

---

## Task 5: Test collect_variables() - Computed Variable (Slug)

**Files:**
- Modify: `src/prompt/engine.rs::tests`

**Step 1: Write the test**

Add to test module:

```rust
#[test]
fn test_collect_variables_computed_slug() {
    let config = TemplateConfig {
        template: TemplateMetadata {
            name: "test".to_string(),
            version: None,
            description: None,
            templates_suffix: ".tera".to_string(),
        },
        variables: vec![
            Variable {
                name: "project_name".to_string(),
                var_type: VariableType::Text,
                description: None,
                default: Some(tera::Value::String("My Cool Project".to_string())),
                choices: None,
                computed: None,
            },
            Variable {
                name: "project_slug".to_string(),
                var_type: VariableType::Text,
                description: None,
                default: None,
                choices: None,
                computed: Some("{{ project_name | slugify }}".to_string()),
            },
        ],
        hooks: HooksConfig { post_create: None },
        ignore: vec![],
    };

    let options = PromptOptions {
        data_overrides: BTreeMap::new(),
        use_defaults: true,
    };

    let result = collect_variables(&config, &options).unwrap();

    assert_eq!(result.get("project_slug").unwrap(), "my-cool-project");
}
```

**Step 2: Run test**

Run: `cargo test test_collect_variables_computed_slug -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/prompt/engine.rs
git commit -m "test: add test_collect_variables_computed_slug"
```

---

## Task 6: Test collect_variables() - Invalid Select Choice

**Files:**
- Modify: `src/prompt/engine.rs::tests`

**Step 1: Write the test**

Add to test module:

```rust
#[test]
fn test_collect_variables_invalid_select_choice() {
    let config = TemplateConfig {
        template: TemplateMetadata {
            name: "test".to_string(),
            version: None,
            description: None,
            templates_suffix: ".tera".to_string(),
        },
        variables: vec![
            Variable {
                name: "license".to_string(),
                var_type: VariableType::Select,
                description: None,
                default: None,
                choices: Some(vec!["MIT".to_string(), "Apache-2.0".to_string()]),
                computed: None,
            },
        ],
        hooks: HooksConfig { post_create: None },
        ignore: vec![],
    };

    let mut overrides = BTreeMap::new();
    overrides.insert("license".to_string(), "InvalidLicense".to_string());

    let options = PromptOptions {
        data_overrides: overrides,
        use_defaults: false,
    };

    let result = collect_variables(&config, &options);

    assert!(result.is_err(), "Should error on invalid select choice");
}
```

**Step 2: Run test**

Run: `cargo test test_collect_variables_invalid_select_choice -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/prompt/engine.rs
git commit -m "test: add test_collect_variables_invalid_select_choice"
```

---

## Task 7: Test collect_variables() - Multiple Variables

**Files:**
- Modify: `src/prompt/engine.rs::tests`

**Step 1: Write the test**

Add to test module:

```rust
#[test]
fn test_collect_variables_multiple() {
    let config = TemplateConfig {
        template: TemplateMetadata {
            name: "test".to_string(),
            version: None,
            description: None,
            templates_suffix: ".tera".to_string(),
        },
        variables: vec![
            Variable {
                name: "name".to_string(),
                var_type: VariableType::Text,
                description: None,
                default: Some(tera::Value::String("test".to_string())),
                choices: None,
                computed: None,
            },
            Variable {
                name: "license".to_string(),
                var_type: VariableType::Select,
                description: None,
                default: Some(tera::Value::String("MIT".to_string())),
                choices: Some(vec!["MIT".to_string(), "Apache-2.0".to_string()]),
                computed: None,
            },
            Variable {
                name: "use_ci".to_string(),
                var_type: VariableType::Boolean,
                description: None,
                default: Some(tera::Value::Bool(false)),
                choices: None,
                computed: None,
            },
        ],
        hooks: HooksConfig { post_create: None },
        ignore: vec![],
    };

    let options = PromptOptions {
        data_overrides: BTreeMap::new(),
        use_defaults: true,
    };

    let result = collect_variables(&config, &options).unwrap();

    assert_eq!(result.len(), 3);
    assert_eq!(result.get("name").unwrap(), "test");
    assert_eq!(result.get("license").unwrap(), "MIT");
    assert_eq!(result.get("use_ci").unwrap(), &tera::Value::Bool(false));
}
```

**Step 2: Run test**

Run: `cargo test test_collect_variables_multiple -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/prompt/engine.rs
git commit -m "test: add test_collect_variables_multiple"
```

---

## Task 8: Test Data Override Type Coercion

**Files:**
- Modify: `src/prompt/engine.rs::tests`

**Step 1: Write test for boolean coercion**

Add to test module:

```rust
use rstest::rstest;

#[rstest]
#[case("true", true)]
#[case("false", false)]
#[case("1", true)]
#[case("0", false)]
fn test_boolean_override_coercion(#[case] input: &str, #[case] expected: bool) {
    let config = TemplateConfig {
        template: TemplateMetadata {
            name: "test".to_string(),
            version: None,
            description: None,
            templates_suffix: ".tera".to_string(),
        },
        variables: vec![
            Variable {
                name: "enabled".to_string(),
                var_type: VariableType::Boolean,
                description: None,
                default: Some(tera::Value::Bool(false)),
                choices: None,
                computed: None,
            },
        ],
        hooks: HooksConfig { post_create: None },
        ignore: vec![],
    };

    let mut overrides = BTreeMap::new();
    overrides.insert("enabled".to_string(), input.to_string());

    let options = PromptOptions {
        data_overrides: overrides,
        use_defaults: false,
    };

    let result = collect_variables(&config, &options).unwrap();

    assert_eq!(result.get("enabled").unwrap(), &tera::Value::Bool(expected));
}
```

**Step 2: Run test**

Run: `cargo test test_boolean_override_coercion -q`
Expected: PASS

**Step 3: Commit**

```bash
git add src/prompt/engine.rs
git commit -m "test: add boolean override coercion tests"
```

---

## Task 9: Test Missing Default Handling

**Files:**
- Modify: `src/prompt/engine.rs::tests`

**Step 1: Write the test**

Add to test module:

```rust
#[test]
fn test_collect_variables_missing_default_with_defaults_mode() {
    let config = TemplateConfig {
        template: TemplateMetadata {
            name: "test".to_string(),
            version: None,
            description: None,
            templates_suffix: ".tera".to_string(),
        },
        variables: vec![
            Variable {
                name: "required_field".to_string(),
                var_type: VariableType::Text,
                description: None,
                default: None,
                choices: None,
                computed: None,
            },
        ],
        hooks: HooksConfig { post_create: None },
        ignore: vec![],
    };

    let options = PromptOptions {
        data_overrides: BTreeMap::new(),
        use_defaults: true,
    };

    let result = collect_variables(&config, &options);

    // Behavior depends on implementation - either error or empty string
    // Adjust based on actual behavior
    assert!(result.is_err() || result.unwrap().get("required_field").is_some());
}
```

**Step 2: Run test and adjust**

Run: `cargo test test_collect_variables_missing_default_with_defaults_mode -q`

If test fails, check the actual behavior and adjust the assertion.

**Step 3: Commit**

```bash
git add src/prompt/engine.rs
git commit -m "test: add missing default handling test"
```

---

## Task 10: Check Coverage

**Step 1: Run coverage report**

Run: `cargo llvm-cov --html`

**Step 2: Check prompt/engine.rs coverage**

Open: `target/llvm-cov/html/index.html`
Look for: `prompt/engine.rs` coverage should be ~65%+

**Step 3: Identify gaps**

If coverage < 65%, check HTML report for uncovered lines. Common gaps:
- Error paths
- Edge cases in validation
- Uncommon variable types

Add targeted tests for uncovered code paths.

**Step 4: Commit milestone**

```bash
git commit --allow-empty -m "chore: prompt/engine.rs coverage now at ~65%"
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
- prompt/engine.rs: ~65%+ coverage
- Overall impact: +4-6 percentage points

**Step 4: Update beads issue**

Run: `bd update diecut-3kz --status=completed`

**Step 5: Final commit**

```bash
git commit --allow-empty -m "test: user input tests complete - coverage increased"
```

---

## Validation Checklist

Before merging:
- [ ] All tests pass (`cargo test`)
- [ ] No new clippy warnings (`cargo clippy`)
- [ ] Code formatted (`cargo fmt --check`)
- [ ] Coverage increased by 4-6 percentage points
- [ ] prompt/engine.rs coverage ~65%+

## Next Steps

After this worktree is complete:
1. Create PR from `test/user-input` branch
2. Include coverage report in PR description
3. Move to next worktree (output-tests) or merge completed worktrees
