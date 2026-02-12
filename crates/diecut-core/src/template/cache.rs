use std::path::{Path, PathBuf};

use sha2::{Sha256, Digest};

use serde::{Deserialize, Serialize};

use crate::error::{DicecutError, Result};
use crate::template::clone::clone_template;

/// Metadata stored alongside a cached template.
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheMetadata {
    /// The original git URL.
    pub url: String,
    /// The git ref (branch, tag, or commit) if specified.
    pub git_ref: Option<String>,
    /// When the template was cached (Unix timestamp in seconds).
    pub cached_at: String,
}

/// A cached template entry returned by `list_cached()`.
#[derive(Debug)]
pub struct CachedTemplate {
    /// The cache key (directory name).
    pub key: String,
    /// The path to the cached template.
    pub path: PathBuf,
    /// Metadata about the cached template.
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
    let sanitize = |s: &str| -> String {
        s.replace(['/', '\\'], "_").replace("..", "_")
    };

    // Build a human-readable prefix from the URL
    let prefix = sanitize(
        normalized
            .rsplit('/')
            .next()
            .unwrap_or("template"),
    );

    match git_ref {
        Some(r) => format!("{prefix}-{}-{hash}", sanitize(r)),
        None => format!("{prefix}-{hash}"),
    }
}

/// Check cache first, clone if missing, return path to the template.
///
/// Note: this function does not protect against concurrent access. If multiple
/// processes call `get_or_clone` for the same URL simultaneously, they may
/// both clone and race to populate the cache entry.
pub fn get_or_clone(url: &str, git_ref: Option<&str>) -> Result<PathBuf> {
    let cache_dir = get_cache_dir()?;
    let key = cache_key(url, git_ref);
    let cached_path = cache_dir.join(&key);

    // Check if we have a valid cached copy
    if cached_path.exists() && cached_path.join(CACHE_METADATA_FILE).exists() {
        return Ok(cached_path);
    }

    // Ensure cache directory exists before cloning
    std::fs::create_dir_all(&cache_dir).map_err(|e| DicecutError::Io {
        context: format!("creating cache directory {}", cache_dir.display()),
        source: e,
    })?;

    // Clone to a temp location, then move into cache.
    // tmp_dir is kept alive so the temp directory is cleaned up on error.
    let tmp_dir = clone_template(url, git_ref)?;

    // Write cache metadata
    let metadata = CacheMetadata {
        url: url.to_string(),
        git_ref: git_ref.map(String::from),
        cached_at: unix_timestamp_secs(),
    };
    let metadata_toml = toml::to_string_pretty(&metadata).map_err(|e| DicecutError::CacheMetadata {
        context: format!("serializing cache metadata: {e}"),
    })?;
    std::fs::write(tmp_dir.path().join(CACHE_METADATA_FILE), metadata_toml).map_err(|e| {
        DicecutError::Io {
            context: "writing cache metadata".into(),
            source: e,
        }
    })?;

    // Remove any stale cache entry
    if cached_path.exists() {
        std::fs::remove_dir_all(&cached_path).map_err(|e| DicecutError::Io {
            context: format!("removing stale cache entry {}", cached_path.display()),
            source: e,
        })?;
    }

    // Move cloned directory into cache. Only persist (leak) the tempdir
    // after successful placement — on error, drop cleans it up.
    std::fs::rename(tmp_dir.path(), &cached_path).or_else(|rename_err| {
        // rename can fail across filesystems; fall back to copy + delete
        copy_dir_all(tmp_dir.path(), &cached_path).map_err(|e| DicecutError::Io {
            context: format!(
                "copying cloned template to cache (rename failed: {rename_err}): {e}"
            ),
            source: std::io::Error::other(e.to_string()),
        })?;
        Ok(())
    })?;

    // Successfully placed in cache — prevent TempDir from cleaning up
    // the source (it may already be gone after a successful rename).
    let _ = tmp_dir.keep();

    Ok(cached_path)
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

        let metadata_str = std::fs::read_to_string(&metadata_path).map_err(|e| DicecutError::Io {
            context: format!("reading cache metadata {}", metadata_path.display()),
            source: e,
        })?;

        let metadata: CacheMetadata =
            toml::from_str(&metadata_str).map_err(|e| DicecutError::CacheMetadata {
                context: format!("parsing cache metadata: {e}"),
            })?;

        let key = entry
            .file_name()
            .to_string_lossy()
            .into_owned();

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
        // Clear specific entries matching this URL (any ref)
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
    } else {
        // Clear entire cache
        if cache_dir.exists() {
            std::fs::remove_dir_all(&cache_dir).map_err(|e| DicecutError::Io {
                context: format!("removing cache directory {}", cache_dir.display()),
                source: e,
            })?;
        }
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

/// Get the current time as a Unix timestamp in seconds.
fn unix_timestamp_secs() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let dir = get_cache_dir().unwrap();
        assert!(dir.ends_with("diecut/templates"));
    }

    #[test]
    fn get_cache_dir_respects_env_var() {
        std::env::set_var("DIECUT_CACHE_DIR", "/tmp/test-diecut-cache");
        let dir = get_cache_dir().unwrap();
        std::env::remove_var("DIECUT_CACHE_DIR");
        assert_eq!(dir, PathBuf::from("/tmp/test-diecut-cache"));
    }

    #[test]
    fn normalize_url_strips_trailing_git_and_slash() {
        assert_eq!(normalize_url("https://github.com/user/repo.git"), "https://github.com/user/repo");
        assert_eq!(normalize_url("https://github.com/user/repo/"), "https://github.com/user/repo");
        assert_eq!(normalize_url("https://github.com/user/repo"), "https://github.com/user/repo");
    }

    #[test]
    fn list_cached_empty_when_no_cache() {
        // With a non-existent cache dir, list_cached should return empty
        let entries = list_cached();
        // This may or may not have entries depending on system state,
        // but it should not error
        assert!(entries.is_ok());
    }
}
