use crate::error::{DicecutError, Result};

#[derive(Debug)]
pub struct CloneResult {
    pub dir: tempfile::TempDir,
    pub commit_sha: Option<String>,
}

/// Clone a git repository to a temporary directory, optionally checking out a
/// specific ref. Rejects `file://` URLs and warns on `http://`.
pub fn clone_template(url: &str, git_ref: Option<&str>) -> Result<CloneResult> {
    if url.starts_with("file://") {
        return Err(DicecutError::UnsafeUrl {
            url: url.to_string(),
            reason: "file:// URLs are not allowed for remote templates".into(),
        });
    }

    if url.starts_with("http://") {
        eprintln!("warning: using insecure http:// URL; consider using https:// instead");
    }

    let tmp_dir = tempfile::tempdir().map_err(|e| DicecutError::Io {
        context: "creating temporary directory for git clone".into(),
        source: e,
    })?;

    let mut prepare =
        gix::prepare_clone(url, tmp_dir.path()).map_err(|e| DicecutError::GitClone {
            url: url.to_string(),
            reason: e.to_string(),
        })?;

    if let Some(ref_name) = git_ref {
        prepare = prepare
            .with_ref_name(Some(ref_name))
            .map_err(|e| DicecutError::GitCheckout {
                git_ref: ref_name.to_string(),
                reason: e.to_string(),
            })?;
    }

    let (mut checkout, _outcome) = prepare
        .fetch_then_checkout(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
        .map_err(|e| DicecutError::GitClone {
            url: url.to_string(),
            reason: e.to_string(),
        })?;

    let (repo, _outcome) = checkout
        .main_worktree(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
        .map_err(|e| DicecutError::GitClone {
            url: url.to_string(),
            reason: format!("worktree checkout failed: {e}"),
        })?;

    let commit_sha = repo.head_id().ok().map(|id| id.to_string());

    Ok(CloneResult {
        dir: tmp_dir,
        commit_sha,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_rejects_invalid_url() {
        let result = clone_template("://bad", None);
        assert!(result.is_err());
        match result.unwrap_err() {
            DicecutError::GitClone { url, .. } => {
                assert_eq!(url, "://bad");
            }
            other => panic!("expected GitClone error, got: {other:?}"),
        }
    }

    #[test]
    fn clone_fails_on_unreachable_host() {
        let result = clone_template("https://nonexistent.invalid/repo.git", None);
        assert!(result.is_err());
    }

    #[test]
    fn clone_rejects_file_url() {
        let result = clone_template("file:///tmp/repo", None);
        assert!(result.is_err());
        match result.unwrap_err() {
            DicecutError::UnsafeUrl { url, .. } => {
                assert_eq!(url, "file:///tmp/repo");
            }
            other => panic!("expected UnsafeUrl error, got: {other:?}"),
        }
    }

    #[test]
    fn clone_result_has_expected_fields() {
        // Verify the struct shape compiles with expected fields
        let tmp = tempfile::tempdir().unwrap();
        let result = CloneResult {
            dir: tmp,
            commit_sha: Some("abc123".to_string()),
        };
        assert!(result.commit_sha.is_some());
        assert!(result.dir.path().exists());
    }
}
