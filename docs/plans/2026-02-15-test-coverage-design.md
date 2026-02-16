# Test Coverage Improvement Design

**Date:** 2026-02-15
**Status:** Approved
**Target:** 61.49% → 80%+ line coverage

## Goal

Improve test coverage of diecut's core features to enable safe refactoring. Focus on generation pipeline, user input, and output persistence.

## Architecture & Scope

### Work Distribution

Split work across 3 parallel worktrees, each focused on a distinct functional area:

#### Worktree 1: `generation-pipeline-tests`
- **Modules:** `lib.rs`, `render/file.rs`
- **Focus:** Main API functions and file rendering logic
- **Current coverage:** lib.rs 28.24%, render/file.rs 53.19%
- **Target coverage:** ~75% for both
- **Impact:** +8-10 percentage points

#### Worktree 2: `user-input-tests`
- **Modules:** `prompt/engine.rs`
- **Focus:** Variable collection, user prompts, default handling
- **Current coverage:** 26.61%
- **Target coverage:** ~65%
- **Impact:** +4-6 percentage points

#### Worktree 3: `output-tests`
- **Modules:** `answers/mod.rs`
- **Focus:** Answers file serialization, metadata writing
- **Current coverage:** 40.47%
- **Target coverage:** ~75%
- **Impact:** +4-5 percentage points

### Dependencies

- All worktrees work independently (no shared state)
- Each branches from current `main`
- Merge order doesn't matter (no conflicts expected)

## Test Components

### Worktree 1: Generation Pipeline Tests

**lib.rs:**
1. `plan_generation()` - Happy path and error cases
   - Valid template sources (local, git)
   - Template directory missing
   - Invalid output paths
   - Variable collection with overrides/defaults
   - Hook warnings for remote templates

2. `execute_generation()` - File writing and hooks
   - Creates output directory
   - Executes render plan correctly
   - Writes answers file
   - Runs post_create hooks (or skips with --no-hooks)
   - Error handling during file writes

3. `generate()` - Full pipeline integration
   - Complete generation flow
   - Verify file counts match expectations

**render/file.rs:**
- File type detection and handling
- Template rendering edge cases
- Binary file copying
- Error propagation

### Worktree 2: User Input Tests

**prompt/engine.rs:**
1. Variable collection with different config types
   - Text variables with defaults
   - Select variables (choice validation)
   - Boolean variables
   - Computed variables (slug generation, etc.)

2. Data overrides via CLI (`-d key=value`)
   - Override precedence
   - Type coercion
   - Invalid overrides

3. Default mode (`--defaults`)
   - Uses all defaults
   - Skips prompts
   - Handles missing defaults gracefully

4. Validation
   - Required fields
   - Choice validation
   - Type validation

### Worktree 3: Output Tests

**answers/mod.rs:**
1. Answers file writing
   - Correct TOML serialization
   - Includes all variables
   - Includes source metadata (URL, commit SHA, git ref)
   - File permissions

2. Edge cases
   - Empty variables
   - Special characters in values
   - Missing source info (local templates)
   - Output directory creation

3. Deserialization
   - Can read back written answers
   - Version compatibility

## Testing Strategy & Patterns

### Test Patterns

**Use existing conventions:**
- `rstest` with `#[case]` for parameterized tests
- `tempfile::tempdir()` for temporary directories
- Standard `#[test]` attribute for simple tests
- Descriptive names: `test_<module>_<behavior>`

**Test organization:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use tempfile;

    // Helper functions at top
    fn setup_fixture() -> ... { ... }

    // Simple tests
    #[test]
    fn test_basic_behavior() { ... }

    // Parameterized tests
    #[rstest]
    #[case(input1, expected1)]
    #[case(input2, expected2)]
    fn test_cases(#[case] input: T, #[case] expected: T) { ... }
}
```

### Focus Areas

- **Happy path:** Core functionality works as expected
- **Error cases:** Proper error propagation and messages
- **Edge cases:** Empty inputs, special characters, boundary conditions
- **Integration:** Components work together correctly

### Quality Gates

**Before marking worktree complete:**
1. Run `cargo test` - all tests pass
2. Run `cargo llvm-cov` - verify coverage increased
3. Run `cargo clippy` - no new warnings
4. Run `cargo fmt --check` - formatting consistent

**Coverage expectations per worktree:**
- Worktree 1: +8-10 percentage points
- Worktree 2: +4-6 percentage points
- Worktree 3: +4-5 percentage points
- **Combined:** 61.49% → 80%+ overall

### Merge Strategy

**Order:** Any order (no dependencies)

**Per worktree:**
1. Create branch from main: `test/generation-pipeline`, `test/user-input`, `test/output`
2. Run tests locally, verify coverage
3. Create PR with coverage report in description
4. Merge when approved

**Final validation:**
After all 3 merged, run `cargo llvm-cov` to confirm 80%+ achieved.

### Anti-patterns to Avoid

- Don't mock what you can test directly (use real tempfiles)
- Don't test implementation details (test behavior)
- Don't skip error cases (they catch refactoring bugs)
- Don't create brittle tests (use fixtures, not hardcoded paths)

## Success Criteria

- [ ] Overall coverage reaches 80%+
- [ ] All new tests pass
- [ ] No new clippy warnings
- [ ] Core features (generation, input, output) have solid test foundation
- [ ] Refactoring can proceed with confidence
