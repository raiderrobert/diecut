# Design: Replace gix with System git for Template Cloning

**Issue:** diecut-e2t — Use GitHub CLI auth for private template cloning
**Date:** 2026-02-13
**Status:** Approved design, pending implementation

## Problem

When cloning private GitHub templates (`diecut new gh:org/private-repo`), users are prompted for credentials even when they have `gh` CLI, SSH keys, or macOS Keychain configured. This happens because diecut uses `gix` (Gitoxide) for clone operations, which has incomplete credential helper support compared to system `git`.

## Research Findings

No production Rust project uses gix for network operations with private repos:

- **Cargo**: Uses git2 (libgit2) by default, gix behind unstable flag only, plus `CARGO_NET_GIT_FETCH_WITH_CLI=true` escape hatch to system git
- **Jujutsu (jj)**: Deprecated libgit2, chose system `git` CLI for all network ops over both gix and libgit2
- **Claude Code, Homebrew, cookiecutter, npm**: All shell out to system `git` for clone operations

The pattern is universal: delegate to system `git` and inherit the user's credential stack.

## Design

### Replace gix with `std::process::Command` calling system `git`

The clone command:

```
git clone --depth 1 [--branch <ref>] <url> <tmp_dir>
```

Commit SHA extracted separately:

```
git -C <tmp_dir> rev-parse HEAD
```

### What changes

| File | Change |
|---|---|
| `crates/diecut-core/src/template/clone.rs` | Replace `gix` calls with `std::process::Command` |
| `Cargo.toml` (workspace root) | Remove `gix` dependency |
| `crates/diecut-core/Cargo.toml` | Remove `gix = { workspace = true }` |
| `crates/diecut-core/src/error.rs` | Add `GitNotFound` variant, improve `GitClone` diagnostics |

### What stays the same

- `CloneResult` struct (`TempDir` + `Option<String>` commit SHA)
- `clone_template(url, git_ref)` function signature
- All callers (`cache.rs`, `lib.rs`) — no API changes
- `file://` URL rejection, `http://` insecure warning
- Existing test coverage (adapted to new implementation)

### Auth benefits

System `git` inherits the user's full credential stack automatically:

- macOS Keychain (`credential.helper = osxkeychain`)
- `gh auth login` / `gh auth setup-git` credential helper
- SSH agent keys
- `~/.netrc` credentials
- Git Credential Manager
- Any custom credential helper in `~/.gitconfig`

No auth-specific code needed in diecut.

### Error handling

On clone failure, parse git stderr for common patterns:

| stderr pattern | Diagnostic |
|---|---|
| `Authentication failed` / `could not read Username` | "For private repos, configure git credentials: run `gh auth login` or set up SSH keys" |
| `Repository not found` | "Check the URL. If private, ensure git credentials are configured" |
| `Host key verification failed` | "Add the host to known_hosts: `ssh-keyscan github.com >> ~/.ssh/known_hosts`" |
| `Could not resolve host` / `Connection refused` | "Check your network connection and the repository URL" |
| Other | Show raw git stderr |

### New requirement: git binary

Add a check when `clone_template` is first called. If `git` is not found in `PATH`:

```
error: git is not installed
  help: Install git from https://git-scm.com
```

Error variant: `DicecutError::GitNotFound`

### Dependency reduction

Removing `gix` eliminates ~65 transitive crates, reducing compile time and binary size. The only new "dependency" is the system `git` binary, which is already a de facto requirement for any tool that clones git repos.

## Implementation Plan

1. Add `GitNotFound` error variant to `error.rs`
2. Rewrite `clone_template()` in `clone.rs` to use `std::process::Command`
3. Improve `GitClone` error diagnostics with stderr parsing
4. Remove `gix` from both `Cargo.toml` files
5. Update existing tests, add tests for error message parsing
6. Run `cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Alternatives Considered

### A) SSH URL rewriting in source.rs (original design)

Detect SSH availability and rewrite `gh:user/repo` to `git@github.com:user/repo.git`. Falls back to `gh auth token` for HTTPS, then plain HTTPS.

**Rejected:** Treats symptoms, not root cause. The real issue is gix's credential helper support, not URL format.

### B) Keep gix, fix credential configuration

Ensure gix's credential features are properly configured and test with common helpers.

**Rejected:** gix credential support has had bugs (#1284), lacks production validation, and no major project trusts it for private repo auth. Even Cargo maintains a system git fallback.

### C) Hybrid gix + system git fallback

Try gix first, fall back to system git on auth failure.

**Rejected:** Unnecessary complexity. If system git is the reliable path, just use it directly.
