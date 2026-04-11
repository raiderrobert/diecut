# Design: SSH-default shortcodes and `--protocol` override

**Issue:** raiderrobert/diecut#131
**Status:** Approved
**Date:** 2026-04-11

## Problem

`gh:user/repo` today expands via a runtime shell-out to `gh config get git_protocol -h github.com` (`src/template/source.rs:25-40`). This produces inconsistent behavior:

- The same shortcode produces different URLs on different machines depending on whether `gh` is installed and how it's configured.
- `gl:` and `cb:` never got this detection — they're hardcoded to HTTPS.
- The detection silently depends on an external CLI being present.

The original design intent was SSH-first cloning — how git is normally used for authenticated work. In practice, today's default is HTTPS unless `gh` is installed *and* configured for SSH. The behavior is the opposite of intended, and it's a bug.

## Goals

- All three built-in shortcodes (`gh:`, `gl:`, `cb:`) default to SSH URLs.
- A single, uniform code path — no GitHub special case.
- No dependency on the `gh` CLI.
- An escape hatch for users stuck on HTTPS (corporate firewalls blocking port 22, GitHub Enterprise with SAML-only HTTPS auth) — via CLI flag and environment variable, no config file yet.
- The `DIECUT_GIT_PROTOCOL` env var is the "set once and forget" story; the `--protocol` flag is the per-invocation override.

## Non-goals

- A user config file at `~/.config/diecut/config.toml`. Deferred to #137. When it lands, `git_protocol` in the config file will slot in as a fourth precedence tier (flag > env > config > default).
- Changing how custom `[abbreviations]` work. They're URL templates and already let authors pick their own scheme. Note: this dead code remains untouched.
- Auto-retry with HTTPS if an SSH clone fails. Too magical; git's own error messages are clear.

## Architecture

The fix lives entirely in the shortcode expansion pipeline. No changes to `TemplateSource`, `clone.rs`, or the top-level CLI resolution order. Touch points:

- `src/template/source.rs` — delete `detect_github_protocol()`, rewrite the `ABBREVIATIONS` table, thread a `GitProtocol` parameter through expansion.
- `src/cli.rs` — add `--protocol <ssh|https>` to the `New` subcommand.
- `src/lib.rs` / command handler — resolve CLI flag + env var into a single `GitProtocol` value, pass it down.
- `src/error.rs` — new `InvalidProtocol` error variant for a bad env var value.
- `tests/integration.rs` — new tests for flag, env var, and precedence.
- `README.md`, `docs/src/content/docs/...`, `CHANGELOG.md` — updated.

No new files, no new modules.

## New types

```rust
// in src/template/source.rs (or a new src/template/protocol.rs if we prefer isolation)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum GitProtocol {
    #[default]
    Ssh,
    Https,
}

impl std::str::FromStr for GitProtocol {
    type Err = DicecutError;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "ssh"   => Ok(GitProtocol::Ssh),
            "https" => Ok(GitProtocol::Https),
            other   => Err(DicecutError::InvalidProtocol {
                value: other.to_string(),
                source: "DIECUT_GIT_PROTOCOL",
            }),
        }
    }
}
```

clap's `ValueEnum` derive handles `--protocol ssh|https` directly. The manual `FromStr` is used by the env var parser so both paths share the same validation and error type.

### Resolution helper

```rust
fn resolve_git_protocol(cli_flag: Option<GitProtocol>) -> Result<GitProtocol> {
    if let Some(p) = cli_flag {
        return Ok(p);
    }
    if let Ok(env_value) = std::env::var("DIECUT_GIT_PROTOCOL") {
        return env_value.parse();  // returns InvalidProtocol on bad value
    }
    Ok(GitProtocol::default())
}
```

Called once at CLI entry. Precedence: flag > env > default.

## Shortcode expansion rewrite

Replace the current `ABBREVIATIONS: &[(&str, &str, &str)]` table (which embeds `https://` and `.git` fragments) with a host-keyed table and a protocol-aware URL builder:

```rust
struct VendorShortcode {
    prefix: &'static str,
    host: &'static str,  // "github.com", "gitlab.com", "codeberg.org"
}

const SHORTCODES: &[VendorShortcode] = &[
    VendorShortcode { prefix: "gh:", host: "github.com"   },
    VendorShortcode { prefix: "gl:", host: "gitlab.com"   },
    VendorShortcode { prefix: "cb:", host: "codeberg.org" },
];

fn build_url(host: &str, repo: &str, protocol: GitProtocol) -> String {
    match protocol {
        GitProtocol::Ssh   => format!("git@{host}:{repo}.git"),
        GitProtocol::Https => format!("https://{host}/{repo}.git"),
    }
}
```

`expand_abbreviation` becomes a single loop over `SHORTCODES` — no more GitHub special case. The following symbols are deleted entirely:

- `detect_github_protocol()`
- `build_github_url()`
- The old `ABBREVIATIONS` 3-tuple const

`split_repo_subpath` stays unchanged — subpath parsing is orthogonal to protocol choice.

## `resolve_source` cleanup

Current public API has three variants that form an unwieldy ladder:

```rust
pub fn resolve_source(template_arg: &str) -> Result<TemplateSource>
pub fn resolve_source_with_ref(template_arg: &str, git_ref: Option<&str>) -> Result<TemplateSource>
pub fn resolve_source_full(
    template_arg: &str,
    git_ref: Option<&str>,
    user_abbreviations: Option<&HashMap<String, String>>,
) -> Result<TemplateSource>
```

`resolve_source` is the only variant called outside tests (`src/lib.rs:47`). The other two are re-exported publicly but unused.

Collapse all three into a single function taking an options struct:

```rust
#[derive(Debug, Default)]
pub struct ResolveOptions<'a> {
    pub git_ref: Option<&'a str>,
    pub protocol: GitProtocol,
    pub user_abbreviations: Option<&'a HashMap<String, String>>,
}

pub fn resolve_source(arg: &str, opts: ResolveOptions<'_>) -> Result<TemplateSource>
```

The existing `resolve_source(&template)` call in `src/lib.rs:47` becomes `resolve_source(&template, ResolveOptions { protocol, ..Default::default() })`. The `with_ref` and `full` names are removed from the public re-exports in `src/template/mod.rs:7`.

This is minor API surgery but worth doing now — the project is pre-1.0, the ladder is the only internal consumer, and future tickets (#132 will add `--ref`/`--subpath` for URL parsing, #134 will read user config) will add more parameters. The struct extends cleanly.

## CLI wiring

Add to `src/cli.rs` in the `New` subcommand:

```rust
/// Protocol to use when expanding shortcodes (ssh or https).
/// Defaults to ssh. Override with DIECUT_GIT_PROTOCOL env var.
#[arg(long, value_enum)]
protocol: Option<GitProtocol>,
```

`Option` distinguishes "not provided" (fall through to env var → default) from "explicitly set" (overrides env var).

In the command handler (wherever `New` is dispatched), at the top:

```rust
let protocol = resolve_git_protocol(args.protocol)?;
let source = resolve_source(&args.template, ResolveOptions {
    protocol,
    ..Default::default()
})?;
```

No changes to the `List` subcommand — it doesn't expand shortcodes.

## Error handling

New error variant in `src/error.rs`:

```rust
#[error("Invalid git protocol value '{value}' in {source}")]
#[diagnostic(help("Expected 'ssh' or 'https'"))]
InvalidProtocol {
    value: String,
    source: &'static str,  // "DIECUT_GIT_PROTOCOL" or "--protocol"
}
```

For the CLI flag, clap's `ValueEnum` derive produces its own error before we ever hit this variant — `InvalidProtocol` is only raised from env var parsing. The flag path never reaches our `FromStr` impl because clap handles it.

The existing `DicecutError::InvalidAbbreviation` variant and its help text (`src/error.rs:81-83`) are correct and stay unchanged — shortcodes still expand, just with a different protocol default.

## Dry-run URL observability (new, required for tests)

Integration testing protocol choice needs the resolved URL to be observable without actually cloning. Today, `diecut new --dry-run` does not print the resolved git URL (confirmed by user).

Add to the dry-run output: print the resolved `TemplateSource` to stdout before any clone would happen. Concrete format:

```
Would clone from: git@github.com:user/repo.git
  ref: main
  subpath: templates/py
```

`ref` and `subpath` lines only appear when their values are `Some`. `Local` sources print `Would use local path: /absolute/path`. Tests match on the URL line only, not the optional indented lines, to keep assertions stable.

This is a small, useful addition for humans ("what is this tool about to do?") and it's the testability hook integration tests need.

## Testing strategy

### Unit tests (`src/template/source.rs`)

Replace / augment the existing abbreviation tests:

- `build_url` for each (vendor, protocol) pair — six cases, rstest-parameterized:
  - `gh:user/repo` × SSH → `git@github.com:user/repo.git`
  - `gh:user/repo` × HTTPS → `https://github.com/user/repo.git`
  - `gl:org/project` × SSH → `git@gitlab.com:org/project.git`
  - `gl:org/project` × HTTPS → `https://gitlab.com/org/project.git`
  - `cb:user/repo` × SSH → `git@codeberg.org:user/repo.git`
  - `cb:user/repo` × HTTPS → `https://codeberg.org/user/repo.git`
- Subpath preservation: `gh:user/repo/templates/py` with each protocol produces correct URL and `subpath = Some("templates/py")`.
- Default value test: `GitProtocol::default() == GitProtocol::Ssh`.

Tests to delete or rewrite:

- `build_github_url_ssh` / `build_github_url_https` (`source.rs:220-229`) — replaced by the parameterized `build_url` test.
- `expand_github_abbreviation` (`source.rs:232-241`) — currently accepts either SSH or HTTPS; rewrite to strict SSH expectation.
- `resolve_abbreviation_to_git_source` (`source.rs:274-293`) — rewrite to strict SSH expectation.
- `resolve_with_ref_sets_git_ref`, `resolve_with_ref_none_leaves_ref_none` (`source.rs:331+`) — rewrite.
- All other tests that assert `url == "...ssh..." || url == "...https..."` — tighten to strict SSH.

### Unit tests for `resolve_git_protocol`

Env-var tests must not race — `std::env::set_var` is process-global. Add `serial_test = "3"` as a dev-dependency and mark env-var tests with `#[serial_test::serial]`. (Neither `serial_test` nor `temp_env` is currently in `Cargo.toml` — confirmed.) Tests:

- No flag, no env → SSH
- Flag=Some(Https), no env → Https
- No flag, env=`https` → Https
- No flag, env=`ssh` → Ssh
- Flag=Some(Https), env=`ssh` → Https (flag wins)
- Flag=Some(Ssh), env=`https` → Ssh (flag wins)
- No flag, env=`http` → `InvalidProtocol` error
- No flag, env=`` (empty) → `InvalidProtocol` error

### Integration tests (`tests/integration.rs`)

Require dry-run URL observability (above) to work.

- `diecut new gh:some/repo --dry-run` → stdout contains `git@github.com:some/repo.git`
- `diecut new gh:some/repo --protocol https --dry-run` → stdout contains `https://github.com/some/repo.git`
- `DIECUT_GIT_PROTOCOL=https diecut new gh:some/repo --dry-run` → stdout contains `https://github.com/some/repo.git`
- `DIECUT_GIT_PROTOCOL=https diecut new gh:some/repo --protocol ssh --dry-run` → stdout contains `git@github.com:some/repo.git` (flag overrides env)
- `DIECUT_GIT_PROTOCOL=foo diecut new gh:some/repo --dry-run` → exits nonzero with "Invalid git protocol" error

### Tests to delete

- `test_resolve_source_rejects_empty_abbreviation_remainder` (`tests/integration.rs:267-271`) stays — empty remainders are still errors, unrelated to this change.
- Any integration test that pokes `resolve_source_full` directly and depends on the old three-function ladder must be updated to use the new `ResolveOptions` struct.

## Docs and changelog

- `README.md`: shortcode section — note that `gh:`/`gl:`/`cb:` default to SSH, link to `--protocol` and `DIECUT_GIT_PROTOCOL`.
- `docs/src/content/docs/using-templates/index.mdx`: same.
- `docs/src/content/docs/reference/commands.md`: document the new flag on `diecut new`.
- `CHANGELOG.md`: **Changed (breaking):** Built-in shortcodes now default to SSH. Users on HTTPS pass `--protocol https` or set `DIECUT_GIT_PROTOCOL=https`. The previous `gh config get` detection is removed.

## Implementation order

1. Introduce `GitProtocol` enum + `resolve_git_protocol` helper + `InvalidProtocol` error variant
2. Rewrite shortcode expansion to take `GitProtocol`, delete `detect_github_protocol` and friends
3. Collapse `resolve_source*` ladder into `resolve_source(arg, ResolveOptions)`
4. Wire `--protocol` flag into `src/cli.rs` and the `New` command handler
5. Add dry-run URL output
6. Delete/rewrite unit tests to match
7. Add new unit tests for `resolve_git_protocol`
8. Add integration tests for flag, env var, precedence
9. Update docs (README, docs site, CHANGELOG)
10. `cargo fmt --check && cargo clippy -- -D warnings && cargo test` clean

## Acceptance criteria

(mirroring #131)

- [ ] `detect_github_protocol()` removed
- [ ] Built-in shortcodes default to SSH
- [ ] `--protocol` CLI flag on `diecut new`
- [ ] `DIECUT_GIT_PROTOCOL` env var honored
- [ ] Precedence order tested (flag > env > default)
- [ ] Dry-run prints the resolved clone URL
- [ ] Tests cover: SSH default, HTTPS via flag, HTTPS via env var, flag overriding env var, invalid env var value
- [ ] Docs updated (README, docs site)
- [ ] CHANGELOG notes the behavior change
