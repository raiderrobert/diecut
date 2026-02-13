use std::path::{Path, PathBuf};

use fs4::fs_std::FileExt;
use sha2::{Digest, Sha256};

use serde::{Deserialize, Serialize};

use crate::error::{DicecutError, Result};
use crate::template::clone::clone_template;

/// Metadata stored alongside a cached template.
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheMetadata {
    pub url: String,
    /// Branch, tag, or commit if specified.
    pub git_ref: Option<String>,
    /// Unix timestamp in seconds.
    pub cached_at: String,
    #[serde(default)]
    pub commit_sha: Option<String>,
}

/// A cached template entry returned by `list_cached()`.
#[derive(Debug)]
pub struct CachedTemplate {
    /// The cache key (directory name).
    pub key: String,
    pub path: PathBuf,
    pub metadata: CacheMetadata,
}

const CACHE_METADATA_FILE: &str = ".diecut-cache.toml";

/// Get the cache directory for templates.
///
/// Checks `DIECUT_CACHE_DIR` env var first. Falls back to
/// `~/.cache/diecut/templates/` on Linux/macOS (XDG-compliant).
/// Returns an error if neither source provides a cache directory.
pub fn get_cache_dir() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("DIECUT_CACHE_DIR") {
        return Ok(PathBuf::from(dir));
    }
    dirs::cache_dir()
        .map(|d| d.join("diecut").join("templates"))
        .ok_or_else(|| DicecutError::Io {
            context: "unable to determine cache directory: set DIECUT_CACHE_DIR or ensure a home directory exists".into(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "no cache directory available"),
        })
}

/// Normalize a URL by stripping trailing `.git` and `/` for consistent comparison.
fn normalize_url(url: &str) -> &str {
    url.trim_end_matches('/').trim_end_matches(".git")
}

/// Generate a deterministic cache key from a URL and optional ref.
///
/// Normalizes the URL by stripping trailing `.git` and trailing `/` before
/// hashing, so `https://github.com/user/repo` and
/// `https://github.com/user/repo.git` produce the same key.
pub(crate) fn cache_key(url: &str, git_ref: Option<&str>) -> String {
    let normalized = normalize_url(url);

    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    if let Some(r) = git_ref {
        hasher.update(b"\0");
        hasher.update(r.as_bytes());
    }
    let digest = hasher.finalize();
    let hash: String = digest.iter().take(8).map(|b| format!("{b:02x}")).collect();

    // Sanitize components to prevent path traversal
    let sanitize = |s: &str| -> String { s.replace(['/', '\\'], "_").replace("..", "_") };

    // Build a human-readable prefix from the URL
    let prefix = sanitize(normalized.rsplit('/').next().unwrap_or("template"));

    match git_ref {
        Some(r) => format!("{prefix}-{}-{hash}", sanitize(r)),
        None => format!("{prefix}-{hash}"),
    }
}

/// Check cache first, clone if missing, return path to the template
/// and the resolved commit SHA (if available).
///
/// Uses OS-level advisory file locks (via `fs4`) to prevent concurrent
/// processes from cloning the same template simultaneously. The lock is
/// automatically released when the process exits, even on crashes.
pub fn get_or_clone(url: &str, git_ref: Option<&str>) -> Result<(PathBuf, Option<String>)> {
    let cache_dir = get_cache_dir()?;
    let key = cache_key(url, git_ref);
    let cached_path = cache_dir.join(&key);

    // Fast path: cache hit without locking
    if cached_path.exists() && cached_path.join(CACHE_METADATA_FILE).exists() {
        let metadata = read_cache_metadata(&cached_path)?;
        return Ok((cached_path, metadata.commit_sha));
    }

    std::fs::create_dir_all(&cache_dir).map_err(|e| DicecutError::Io {
        context: format!("creating cache directory {}", cache_dir.display()),
        source: e,
    })?;

    // Acquire an exclusive advisory lock for this cache key.
    // Blocks until the lock is available; automatically released on drop/exit.
    let lock_path = cache_dir.join(format!("{key}.lock"));
    let lock_file = std::fs::File::create(&lock_path).map_err(|e| DicecutError::Io {
        context: format!("creating lock file {}", lock_path.display()),
        source: e,
    })?;
    lock_file.lock_exclusive().map_err(|e| DicecutError::Io {
        context: format!("acquiring cache lock for {key}"),
        source: e,
    })?;

    // Re-check cache after acquiring lock — another process may have populated it.
    if cached_path.exists() && cached_path.join(CACHE_METADATA_FILE).exists() {
        let metadata = read_cache_metadata(&cached_path)?;
        return Ok((cached_path, metadata.commit_sha));
    }

    let clone_result = clone_template(url, git_ref)?;

    let metadata = CacheMetadata {
        url: url.to_string(),
        git_ref: git_ref.map(String::from),
        cached_at: unix_timestamp_secs(),
        commit_sha: clone_result.commit_sha.clone(),
    };
    let metadata_toml =
        toml::to_string_pretty(&metadata).map_err(|e| DicecutError::CacheMetadata {
            context: format!("serializing cache metadata: {e}"),
        })?;
    std::fs::write(
        clone_result.dir.path().join(CACHE_METADATA_FILE),
        metadata_toml,
    )
    .map_err(|e| DicecutError::Io {
        context: "writing cache metadata".into(),
        source: e,
    })?;

    if cached_path.exists() {
        std::fs::remove_dir_all(&cached_path).map_err(|e| DicecutError::Io {
            context: format!("removing stale cache entry {}", cached_path.display()),
            source: e,
        })?;
    }

    // Only persist (leak) the tempdir after successful placement —
    // on error, drop cleans it up.
    let commit_sha = clone_result.commit_sha.clone();
    std::fs::rename(clone_result.dir.path(), &cached_path).or_else(|rename_err| {
        // rename can fail across filesystems; fall back to copy + delete
        copy_dir_all(clone_result.dir.path(), &cached_path).map_err(|e| DicecutError::Io {
            context: format!("copying cloned template to cache (rename failed: {rename_err}): {e}"),
            source: std::io::Error::other(e.to_string()),
        })?;
        Ok(())
    })?;

    // Source may already be gone after a successful rename.
    let _ = clone_result.dir.keep();

    Ok((cached_path, commit_sha))
}

fn read_cache_metadata(cached_path: &Path) -> Result<CacheMetadata> {
    let metadata_path = cached_path.join(CACHE_METADATA_FILE);
    let metadata_str = std::fs::read_to_string(&metadata_path).map_err(|e| DicecutError::Io {
        context: format!("reading cache metadata {}", metadata_path.display()),
        source: e,
    })?;
    toml::from_str(&metadata_str).map_err(|e| DicecutError::CacheMetadata {
        context: format!("parsing cache metadata: {e}"),
    })
}

/// List all cached templates.
pub fn list_cached() -> Result<Vec<CachedTemplate>> {
    let cache_dir = get_cache_dir()?;
    if !cache_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    let read_dir = std::fs::read_dir(&cache_dir).map_err(|e| DicecutError::Io {
        context: format!("reading cache directory {}", cache_dir.display()),
        source: e,
    })?;

    for entry in read_dir {
        let entry = entry.map_err(|e| DicecutError::Io {
            context: "reading cache directory entry".into(),
            source: e,
        })?;

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let metadata_path = path.join(CACHE_METADATA_FILE);
        if !metadata_path.exists() {
            continue;
        }

        let metadata_str =
            std::fs::read_to_string(&metadata_path).map_err(|e| DicecutError::Io {
                context: format!("reading cache metadata {}", metadata_path.display()),
                source: e,
            })?;

        let metadata: CacheMetadata =
            toml::from_str(&metadata_str).map_err(|e| DicecutError::CacheMetadata {
                context: format!("parsing cache metadata: {e}"),
            })?;

        let key = entry.file_name().to_string_lossy().into_owned();

        entries.push(CachedTemplate {
            key,
            path,
            metadata,
        });
    }

    Ok(entries)
}

/// Clear cached templates.
///
/// If `url` is provided, only the cache entry matching that URL is removed.
/// If `url` is None, the entire cache directory is cleared.
pub fn clear_cache(url: Option<&str>) -> Result<()> {
    let cache_dir = get_cache_dir()?;

    if let Some(url) = url {
        if !cache_dir.exists() {
            return Ok(());
        }
        let normalized_input = normalize_url(url);
        let entries = list_cached()?;
        for entry in entries {
            if normalize_url(&entry.metadata.url) == normalized_input {
                std::fs::remove_dir_all(&entry.path).map_err(|e| DicecutError::Io {
                    context: format!("removing cached template {}", entry.path.display()),
                    source: e,
                })?;
            }
        }
    } else if cache_dir.exists() {
        std::fs::remove_dir_all(&cache_dir).map_err(|e| DicecutError::Io {
            context: format!("removing cache directory {}", cache_dir.display()),
            source: e,
        })?;
    }

    Ok(())
}

/// Recursively copy a directory, skipping symlinks.
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).map_err(|e| DicecutError::Io {
        context: format!("creating directory {}", dst.display()),
        source: e,
    })?;

    for entry in std::fs::read_dir(src).map_err(|e| DicecutError::Io {
        context: format!("reading directory {}", src.display()),
        source: e,
    })? {
        let entry = entry.map_err(|e| DicecutError::Io {
            context: "reading directory entry".into(),
            source: e,
        })?;

        let file_type = entry.file_type().map_err(|e| DicecutError::Io {
            context: "reading file type of directory entry".into(),
            source: e,
        })?;

        // Skip symlinks to prevent symlink-following attacks
        if file_type.is_symlink() {
            continue;
        }

        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| DicecutError::Io {
                context: format!("copying {} to {}", src_path.display(), dst_path.display()),
                source: e,
            })?;
        }
    }

    Ok(())
}

fn unix_timestamp_secs() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_secs().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Tests that set DIECUT_CACHE_DIR must hold this lock to avoid racing
    /// each other (env vars are process-global).
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn cache_key_deterministic() {
        let key1 = cache_key("https://github.com/user/repo.git", None);
        let key2 = cache_key("https://github.com/user/repo.git", None);
        assert_eq!(key1, key2);
    }

    #[test]
    fn cache_key_normalizes_trailing_git() {
        let key1 = cache_key("https://github.com/user/repo.git", None);
        let key2 = cache_key("https://github.com/user/repo", None);
        assert_eq!(key1, key2);
    }

    #[test]
    fn cache_key_normalizes_trailing_slash() {
        let key1 = cache_key("https://github.com/user/repo/", None);
        let key2 = cache_key("https://github.com/user/repo", None);
        assert_eq!(key1, key2);
    }

    #[test]
    fn cache_key_differs_by_ref() {
        let key1 = cache_key("https://github.com/user/repo", Some("main"));
        let key2 = cache_key("https://github.com/user/repo", Some("develop"));
        assert_ne!(key1, key2);
    }

    #[test]
    fn cache_key_none_ref_differs_from_some() {
        let key1 = cache_key("https://github.com/user/repo", None);
        let key2 = cache_key("https://github.com/user/repo", Some("main"));
        assert_ne!(key1, key2);
    }

    #[test]
    fn cache_key_includes_repo_name() {
        let key = cache_key("https://github.com/user/my-template.git", None);
        assert!(key.starts_with("my-template-"));
    }

    #[test]
    fn cache_key_includes_ref_in_name() {
        let key = cache_key("https://github.com/user/repo", Some("v2.0"));
        assert!(key.contains("v2.0"));
    }

    #[test]
    fn get_cache_dir_returns_xdg_path() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::remove_var("DIECUT_CACHE_DIR");
        let dir = get_cache_dir().unwrap();
        assert!(dir.ends_with("diecut/templates"));
    }

    #[test]
    fn get_cache_dir_respects_env_var() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::set_var("DIECUT_CACHE_DIR", "/tmp/test-diecut-cache");
        let dir = get_cache_dir().unwrap();
        std::env::remove_var("DIECUT_CACHE_DIR");
        assert_eq!(dir, PathBuf::from("/tmp/test-diecut-cache"));
    }

    #[test]
    fn normalize_url_strips_trailing_git_and_slash() {
        assert_eq!(
            normalize_url("https://github.com/user/repo.git"),
            "https://github.com/user/repo"
        );
        assert_eq!(
            normalize_url("https://github.com/user/repo/"),
            "https://github.com/user/repo"
        );
        assert_eq!(
            normalize_url("https://github.com/user/repo"),
            "https://github.com/user/repo"
        );
    }

    #[test]
    fn list_cached_empty_when_no_cache() {
        let _lock = ENV_LOCK.lock().unwrap();
        // Point at a non-existent dir so the result is deterministic
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("DIECUT_CACHE_DIR", tmp.path().join("empty"));
        let entries = list_cached();
        assert!(entries.is_ok());
        assert!(entries.unwrap().is_empty());
    }

    /// Helper: acquire the env lock, set DIECUT_CACHE_DIR to a temp directory,
    /// and return both guards. The lock is held until both are dropped.
    fn setup_cache_env() -> (std::sync::MutexGuard<'static, ()>, tempfile::TempDir) {
        let lock = ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        std::env::set_var("DIECUT_CACHE_DIR", tmp.path());
        (lock, tmp)
    }

    /// Helper: create a fake cached template entry with metadata.
    fn create_fake_cache_entry(
        cache_dir: &Path,
        key: &str,
        url: &str,
        git_ref: Option<&str>,
    ) -> PathBuf {
        create_fake_cache_entry_with_sha(cache_dir, key, url, git_ref, None)
    }

    fn create_fake_cache_entry_with_sha(
        cache_dir: &Path,
        key: &str,
        url: &str,
        git_ref: Option<&str>,
        commit_sha: Option<&str>,
    ) -> PathBuf {
        let entry_dir = cache_dir.join(key);
        std::fs::create_dir_all(&entry_dir).unwrap();
        let metadata = CacheMetadata {
            url: url.to_string(),
            git_ref: git_ref.map(String::from),
            cached_at: "1700000000".to_string(),
            commit_sha: commit_sha.map(String::from),
        };
        let toml_str = toml::to_string_pretty(&metadata).unwrap();
        std::fs::write(entry_dir.join(CACHE_METADATA_FILE), toml_str).unwrap();
        entry_dir
    }

    // ── copy_dir_all tests ──────────────────────────────────────────

    #[test]
    fn copy_dir_all_copies_flat_directory() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        let dst_target = dst.path().join("output");

        std::fs::write(src.path().join("a.txt"), "alpha").unwrap();
        std::fs::write(src.path().join("b.txt"), "bravo").unwrap();

        copy_dir_all(src.path(), &dst_target).unwrap();

        assert_eq!(
            std::fs::read_to_string(dst_target.join("a.txt")).unwrap(),
            "alpha"
        );
        assert_eq!(
            std::fs::read_to_string(dst_target.join("b.txt")).unwrap(),
            "bravo"
        );
    }

    #[test]
    fn copy_dir_all_copies_nested_directories() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        let dst_target = dst.path().join("output");

        std::fs::create_dir_all(src.path().join("sub/deep")).unwrap();
        std::fs::write(src.path().join("root.txt"), "root").unwrap();
        std::fs::write(src.path().join("sub/mid.txt"), "mid").unwrap();
        std::fs::write(src.path().join("sub/deep/leaf.txt"), "leaf").unwrap();

        copy_dir_all(src.path(), &dst_target).unwrap();

        assert_eq!(
            std::fs::read_to_string(dst_target.join("root.txt")).unwrap(),
            "root"
        );
        assert_eq!(
            std::fs::read_to_string(dst_target.join("sub/mid.txt")).unwrap(),
            "mid"
        );
        assert_eq!(
            std::fs::read_to_string(dst_target.join("sub/deep/leaf.txt")).unwrap(),
            "leaf"
        );
    }

    #[test]
    fn copy_dir_all_skips_symlinks() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        let dst_target = dst.path().join("output");

        std::fs::write(src.path().join("real.txt"), "real").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(src.path().join("real.txt"), src.path().join("link.txt"))
            .unwrap();

        copy_dir_all(src.path(), &dst_target).unwrap();

        assert!(dst_target.join("real.txt").exists());
        #[cfg(unix)]
        assert!(!dst_target.join("link.txt").exists());
    }

    #[test]
    fn copy_dir_all_creates_destination_if_missing() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        let dst_target = dst.path().join("a/b/c");

        std::fs::write(src.path().join("file.txt"), "content").unwrap();

        copy_dir_all(src.path(), &dst_target).unwrap();

        assert_eq!(
            std::fs::read_to_string(dst_target.join("file.txt")).unwrap(),
            "content"
        );
    }

    #[test]
    fn copy_dir_all_empty_directory() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        let dst_target = dst.path().join("output");

        copy_dir_all(src.path(), &dst_target).unwrap();

        assert!(dst_target.exists());
        assert!(dst_target.is_dir());
        let count = std::fs::read_dir(&dst_target).unwrap().count();
        assert_eq!(count, 0);
    }

    // ── clear_cache tests ───────────────────────────────────────────

    #[test]
    fn clear_cache_all_removes_entire_directory() {
        let (_lock, tmp) = setup_cache_env();
        let cache_dir = tmp.path().to_path_buf();

        create_fake_cache_entry(&cache_dir, "repo-abc123", "https://github.com/u/repo", None);
        create_fake_cache_entry(
            &cache_dir,
            "other-def456",
            "https://github.com/u/other",
            None,
        );

        assert!(cache_dir.join("repo-abc123").exists());
        assert!(cache_dir.join("other-def456").exists());

        clear_cache(None).unwrap();

        assert!(!cache_dir.exists());
    }

    #[test]
    fn clear_cache_by_url_removes_only_matching_entries() {
        let (_lock, tmp) = setup_cache_env();
        let cache_dir = tmp.path().to_path_buf();

        create_fake_cache_entry(&cache_dir, "repo-abc123", "https://github.com/u/repo", None);
        create_fake_cache_entry(
            &cache_dir,
            "repo-main-def456",
            "https://github.com/u/repo",
            Some("main"),
        );
        create_fake_cache_entry(
            &cache_dir,
            "other-ghi789",
            "https://github.com/u/other",
            None,
        );

        clear_cache(Some("https://github.com/u/repo")).unwrap();

        // Both entries for /u/repo should be gone
        assert!(!cache_dir.join("repo-abc123").exists());
        assert!(!cache_dir.join("repo-main-def456").exists());
        // The unrelated entry should remain
        assert!(cache_dir.join("other-ghi789").exists());
    }

    #[test]
    fn clear_cache_by_url_normalizes_trailing_git() {
        let (_lock, tmp) = setup_cache_env();
        let cache_dir = tmp.path().to_path_buf();

        // Cached with bare URL
        create_fake_cache_entry(&cache_dir, "repo-abc123", "https://github.com/u/repo", None);

        // Clear using .git suffix — should still match
        clear_cache(Some("https://github.com/u/repo.git")).unwrap();

        assert!(!cache_dir.join("repo-abc123").exists());
    }

    #[test]
    fn clear_cache_noop_when_cache_dir_missing() {
        let _lock = ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let nonexistent = tmp.path().join("does-not-exist");
        std::env::set_var("DIECUT_CACHE_DIR", &nonexistent);

        // Should succeed without error even though the dir doesn't exist
        assert!(clear_cache(None).is_ok());
        assert!(clear_cache(Some("https://example.com/repo")).is_ok());
    }

    // ── list_cached tests ───────────────────────────────────────────

    #[test]
    fn list_cached_returns_entries_with_metadata() {
        let (_lock, tmp) = setup_cache_env();
        let cache_dir = tmp.path().to_path_buf();

        create_fake_cache_entry(&cache_dir, "repo-abc", "https://github.com/u/repo", None);
        create_fake_cache_entry(
            &cache_dir,
            "repo-main-def",
            "https://github.com/u/repo",
            Some("main"),
        );

        let entries = list_cached().unwrap();
        assert_eq!(entries.len(), 2);

        let keys: Vec<&str> = entries.iter().map(|e| e.key.as_str()).collect();
        assert!(keys.contains(&"repo-abc"));
        assert!(keys.contains(&"repo-main-def"));

        for entry in &entries {
            assert_eq!(entry.metadata.url, "https://github.com/u/repo");
            assert_eq!(entry.metadata.cached_at, "1700000000");
        }

        // Check that the entry with ref has it recorded
        let with_ref = entries.iter().find(|e| e.key == "repo-main-def").unwrap();
        assert_eq!(with_ref.metadata.git_ref.as_deref(), Some("main"));

        let without_ref = entries.iter().find(|e| e.key == "repo-abc").unwrap();
        assert!(without_ref.metadata.git_ref.is_none());
    }

    #[test]
    fn list_cached_skips_dirs_without_metadata() {
        let (_lock, tmp) = setup_cache_env();
        let cache_dir = tmp.path().to_path_buf();

        create_fake_cache_entry(&cache_dir, "valid-entry", "https://github.com/u/repo", None);

        // Directory without metadata file
        std::fs::create_dir_all(cache_dir.join("orphan-dir")).unwrap();
        std::fs::write(cache_dir.join("orphan-dir/some-file.txt"), "data").unwrap();

        let entries = list_cached().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "valid-entry");
    }

    #[test]
    fn list_cached_skips_plain_files() {
        let (_lock, tmp) = setup_cache_env();
        let cache_dir = tmp.path().to_path_buf();

        create_fake_cache_entry(&cache_dir, "valid-entry", "https://github.com/u/repo", None);

        // Plain file in the cache dir (not a directory)
        std::fs::write(cache_dir.join("stray-file.txt"), "oops").unwrap();

        let entries = list_cached().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "valid-entry");
    }

    #[test]
    fn list_cached_empty_when_dir_does_not_exist() {
        let _lock = ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let nonexistent = tmp.path().join("no-such-dir");
        std::env::set_var("DIECUT_CACHE_DIR", &nonexistent);

        let entries = list_cached().unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn list_cached_path_points_to_entry_dir() {
        let (_lock, tmp) = setup_cache_env();
        let cache_dir = tmp.path().to_path_buf();

        let expected = create_fake_cache_entry(
            &cache_dir,
            "myrepo-abc",
            "https://github.com/u/myrepo",
            None,
        );

        let entries = list_cached().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, expected);
    }

    // ── get_or_clone cache-hit path ─────────────────────────────────

    #[test]
    fn get_or_clone_returns_cached_path_on_hit() {
        let (_lock, tmp) = setup_cache_env();
        let cache_dir = tmp.path().to_path_buf();

        let url = "https://github.com/u/repo";
        let key = cache_key(url, None);

        // Pre-populate cache with a fake entry including metadata
        create_fake_cache_entry(&cache_dir, &key, url, None);
        std::fs::write(cache_dir.join(&key).join("diecut.toml"), "[template]").unwrap();

        let (path, sha) = get_or_clone(url, None).unwrap();

        assert_eq!(path, cache_dir.join(&key));
        assert!(sha.is_none()); // No SHA in legacy entries
        assert_eq!(
            std::fs::read_to_string(path.join("diecut.toml")).unwrap(),
            "[template]"
        );
    }

    #[test]
    fn get_or_clone_returns_commit_sha_from_cache() {
        let (_lock, tmp) = setup_cache_env();
        let cache_dir = tmp.path().to_path_buf();

        let url = "https://github.com/u/repo";
        let key = cache_key(url, None);
        let expected_sha = "abc123def456";

        create_fake_cache_entry_with_sha(&cache_dir, &key, url, None, Some(expected_sha));

        let (path, sha) = get_or_clone(url, None).unwrap();
        assert_eq!(path, cache_dir.join(&key));
        assert_eq!(sha.as_deref(), Some(expected_sha));
    }

    #[test]
    fn get_or_clone_does_not_use_stale_entry_without_metadata() {
        let (_lock, tmp) = setup_cache_env();
        let cache_dir = tmp.path().to_path_buf();

        let url = "https://github.com/u/repo";
        let key = cache_key(url, None);

        // Create directory but WITHOUT metadata file
        std::fs::create_dir_all(cache_dir.join(&key)).unwrap();
        std::fs::write(cache_dir.join(&key).join("some-file.txt"), "old").unwrap();

        // Should NOT return the stale entry — will try to clone and fail
        let result = get_or_clone(url, None);
        assert!(result.is_err());
    }

    #[test]
    fn get_or_clone_cache_hit_with_ref() {
        let (_lock, tmp) = setup_cache_env();
        let cache_dir = tmp.path().to_path_buf();

        let url = "https://github.com/u/repo";
        let git_ref = Some("v1.0");
        let key = cache_key(url, git_ref);

        create_fake_cache_entry(&cache_dir, &key, url, git_ref);

        let (path, _sha) = get_or_clone(url, git_ref).unwrap();
        assert_eq!(path, cache_dir.join(&key));
    }

    #[test]
    fn cache_metadata_deserializes_without_commit_sha() {
        // Old cache entries won't have commit_sha — verify backwards compat
        let toml_str = r#"
url = "https://github.com/u/repo"
cached_at = "1700000000"
"#;
        let metadata: CacheMetadata = toml::from_str(toml_str).unwrap();
        assert_eq!(metadata.url, "https://github.com/u/repo");
        assert!(metadata.commit_sha.is_none());
    }
}
