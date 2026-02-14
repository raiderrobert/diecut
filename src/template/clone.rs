use std::process::Command;

use crate::error::{DicecutError, Result};

#[derive(Debug)]
pub struct CloneResult {
    pub dir: tempfile::TempDir,
    pub commit_sha: Option<String>,
}

/// Classify git stderr output into a user-friendly error message with
/// actionable suggestions for common failure modes.
fn classify_clone_error(stderr: &str) -> String {
    if stderr.contains("Authentication failed") || stderr.contains("could not read Username") {
        format!("authentication failed — configure git credentials with `gh auth login` or set up SSH keys\n\ngit output:\n{stderr}")
    } else if stderr.contains("Repository not found")
        || (stderr.contains("not found") && stderr.contains("repository"))
    {
        format!("repository not found — check the URL; if private, ensure git credentials are configured\n\ngit output:\n{stderr}")
    } else if stderr.contains("Host key verification failed") {
        format!("SSH host key verification failed — try: ssh-keyscan github.com >> ~/.ssh/known_hosts\n\ngit output:\n{stderr}")
    } else if stderr.contains("Could not resolve host") || stderr.contains("Connection refused") {
        format!(
            "network error — check your connection and the repository URL\n\ngit output:\n{stderr}"
        )
    } else {
        stderr.to_string()
    }
}

/// Clone a git repository to a temporary directory, optionally checking out a
/// specific ref. Rejects `file://` URLs and warns on `http://`.
///
/// Uses the system `git` binary so that the user's full credential stack
/// (macOS Keychain, SSH agent, `gh auth`, credential helpers, etc.) is
/// inherited automatically.
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

    Command::new("git")
        .arg("--version")
        .output()
        .map_err(|_| DicecutError::GitNotFound)?;

    let tmp_dir = tempfile::tempdir().map_err(|e| DicecutError::Io {
        context: "creating temporary directory for git clone".into(),
        source: e,
    })?;

    let mut cmd = Command::new("git");
    cmd.env("GIT_TERMINAL_PROMPT", "0")
        .arg("clone")
        .arg("--depth")
        .arg("1");

    if let Some(ref_name) = git_ref {
        cmd.arg("--branch").arg(ref_name);
    }

    cmd.arg(url).arg(tmp_dir.path());

    let output = cmd.output().map_err(|e| DicecutError::Io {
        context: "running git clone".into(),
        source: e,
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DicecutError::GitClone {
            url: url.to_string(),
            reason: classify_clone_error(stderr.trim()),
        });
    }

    let rev_output = Command::new("git")
        .arg("-C")
        .arg(tmp_dir.path())
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .map_err(|e| DicecutError::Io {
            context: "running git rev-parse HEAD".into(),
            source: e,
        })?;

    let commit_sha = if rev_output.status.success() {
        Some(
            String::from_utf8_lossy(&rev_output.stdout)
                .trim()
                .to_string(),
        )
    } else {
        None
    };

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
        let tmp = tempfile::tempdir().unwrap();
        let result = CloneResult {
            dir: tmp,
            commit_sha: Some("abc123".to_string()),
        };
        assert!(result.commit_sha.is_some());
        assert!(result.dir.path().exists());
    }

    #[test]
    fn classify_auth_failure() {
        let msg = classify_clone_error(
            "fatal: Authentication failed for 'https://github.com/org/repo.git'",
        );
        assert!(msg.contains("configure git credentials"));
    }

    #[test]
    fn classify_repo_not_found() {
        let msg =
            classify_clone_error("fatal: repository 'https://github.com/org/repo.git/' not found");
        assert!(msg.contains("repository not found"));
    }

    #[test]
    fn classify_host_key_failure() {
        let msg = classify_clone_error("Host key verification failed.");
        assert!(msg.contains("ssh-keyscan"));
    }

    #[test]
    fn classify_network_error() {
        let msg =
            classify_clone_error("fatal: unable to access: Could not resolve host: github.com");
        assert!(msg.contains("network error"));
    }

    #[test]
    fn classify_terminal_prompts_disabled() {
        let msg = classify_clone_error(
            "fatal: could not read Username for 'https://github.com': terminal prompts disabled",
        );
        assert!(msg.contains("configure git credentials"));
    }

    #[test]
    fn classify_unknown_error() {
        let msg = classify_clone_error("fatal: something unexpected happened");
        assert_eq!(msg, "fatal: something unexpected happened");
    }
}
