use std::path::PathBuf;

use crate::error::{DicecutError, Result};

/// Clone a git repository to a temporary directory and return the path.
///
/// If `git_ref` is provided, the repository is checked out at that ref
/// (branch, tag, or commit). The caller is responsible for cleaning up
/// the temporary directory.
pub fn clone_template(url: &str, git_ref: Option<&str>) -> Result<PathBuf> {
    let tmp_dir = tempfile::tempdir().map_err(|e| DicecutError::Io {
        context: "creating temporary directory for git clone".into(),
        source: e,
    })?;

    let clone_path = tmp_dir.path().to_path_buf();

    // Use prepare_clone (with worktree) so we get a checked-out working copy
    let mut prepare = gix::prepare_clone(url, &clone_path).map_err(|e| {
        DicecutError::GitClone {
            url: url.to_string(),
            reason: e.to_string(),
        }
    })?;

    // If a specific ref is requested, configure it before fetching
    if let Some(ref_name) = git_ref {
        prepare = prepare.with_ref_name(Some(ref_name)).map_err(|e| {
            DicecutError::GitCheckout {
                git_ref: ref_name.to_string(),
                reason: e.to_string(),
            }
        })?;
    }

    // Fetch and prepare for checkout
    let (mut checkout, _outcome) = prepare
        .fetch_then_checkout(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
        .map_err(|e| DicecutError::GitClone {
            url: url.to_string(),
            reason: e.to_string(),
        })?;

    // Checkout the main worktree
    let (_repo, _outcome) =
        checkout
            .main_worktree(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
            .map_err(|e| DicecutError::GitClone {
                url: url.to_string(),
                reason: format!("worktree checkout failed: {e}"),
            })?;

    // Persist the tempdir so it isn't deleted when this function returns.
    // The caller (or a higher-level cleanup mechanism) owns the path.
    let _ = tmp_dir.keep();

    Ok(clone_path)
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
        // Network call that should fail quickly on a non-routable address
        let result = clone_template("https://nonexistent.invalid/repo.git", None);
        assert!(result.is_err());
    }
}
