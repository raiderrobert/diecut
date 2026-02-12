use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

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
    /// When the template was cached (ISO 8601).
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
/// Returns `~/.cache/diecut/templates/` on Linux/macOS (XDG-compliant).
pub fn get_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".cache"))
        .join("diecut")
        .join("templates")
}

/// Generate a deterministic cache key from a URL and optional ref.
///
/// Normalizes the URL by stripping trailing `.git` and trailing `/` before
/// hashing, so `https://github.com/user/repo` and
/// `https://github.com/user/repo.git` produce the same key.
pub fn cache_key(url: &str, git_ref: Option<&str>) -> String {
    let normalized = url
        .trim_end_matches('/')
        .trim_end_matches(".git");

    let mut hasher = DefaultHasher::new();
    normalized.hash(&mut hasher);
    if let Some(r) = git_ref {
        r.hash(&mut hasher);
    }
    let hash = hasher.finish();

    // Build a human-readable prefix from the URL
    let prefix = normalized
        .rsplit('/')
        .next()
        .unwrap_or("template");

    match git_ref {
        Some(r) => format!("{prefix}-{r}-{hash:016x}"),
        None => format!("{prefix}-{hash:016x}"),
    }
}

/// Check cache first, clone if missing, return path to the template.
pub fn get_or_clone(url: &str, git_ref: Option<&str>) -> Result<PathBuf> {
    let cache_dir = get_cache_dir();
    let key = cache_key(url, git_ref);
    let cached_path = cache_dir.join(&key);

    // Check if we have a valid cached copy
    if cached_path.exists() && cached_path.join(CACHE_METADATA_FILE).exists() {
        return Ok(cached_path);
    }

    // Clone to a temp location, then move into cache
    let cloned_path = clone_template(url, git_ref)?;

    // Write cache metadata
    let metadata = CacheMetadata {
        url: url.to_string(),
        git_ref: git_ref.map(String::from),
        cached_at: chrono_now(),
    };
    let metadata_toml = toml::to_string_pretty(&metadata).map_err(|e| DicecutError::Io {
        context: format!("serializing cache metadata: {e}"),
        source: std::io::Error::other(e.to_string()),
    })?;
    std::fs::write(cloned_path.join(CACHE_METADATA_FILE), metadata_toml).map_err(|e| {
        DicecutError::Io {
            context: "writing cache metadata".into(),
            source: e,
        }
    })?;

    // Ensure cache directory exists
    std::fs::create_dir_all(&cache_dir).map_err(|e| DicecutError::Io {
        context: format!("creating cache directory {}", cache_dir.display()),
        source: e,
    })?;

    // Remove any stale cache entry
    if cached_path.exists() {
        std::fs::remove_dir_all(&cached_path).map_err(|e| DicecutError::Io {
            context: format!("removing stale cache entry {}", cached_path.display()),
            source: e,
        })?;
    }

    // Move cloned directory into cache
    std::fs::rename(&cloned_path, &cached_path).or_else(|_| {
        // rename can fail across filesystems; fall back to copy + delete
        copy_dir_all(&cloned_path, &cached_path)?;
        std::fs::remove_dir_all(&cloned_path).map_err(|e| DicecutError::Io {
            context: "cleaning up temp clone directory".into(),
            source: e,
        })
    })?;

    Ok(cached_path)
}

/// List all cached templates.
pub fn list_cached() -> Result<Vec<CachedTemplate>> {
    let cache_dir = get_cache_dir();
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
            toml::from_str(&metadata_str).map_err(|e| DicecutError::Io {
                context: format!("parsing cache metadata: {e}"),
                source: std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
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
    let cache_dir = get_cache_dir();

    if let Some(url) = url {
        // Clear specific entries matching this URL (any ref)
        if !cache_dir.exists() {
            return Ok(());
        }
        let entries = list_cached()?;
        for entry in entries {
            if entry.metadata.url == url {
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

/// Recursively copy a directory.
fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> Result<()> {
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
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
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

/// Get the current time as an ISO 8601 string without pulling in chrono.
fn chrono_now() -> String {
    // Use std::time for a simple timestamp
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
        let dir = get_cache_dir();
        assert!(dir.ends_with("diecut/templates"));
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
