use std::path::Path;

use walkdir::WalkDir;

use crate::error::{DicecutError, Result};

/// Copy all files from `src` into `dst`, overwriting existing files.
/// Directories are created as needed. Symlinks are skipped.
pub fn overlay_dir(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }

    for entry in WalkDir::new(src)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let file_type = entry.file_type();
        if file_type.is_symlink() {
            continue;
        }

        let rel = entry
            .path()
            .strip_prefix(src)
            .expect("entry must be under src");
        let dest_path = dst.join(rel);

        if file_type.is_dir() {
            std::fs::create_dir_all(&dest_path).map_err(|e| DicecutError::Io {
                context: format!("creating directory {}", dest_path.display()),
                source: e,
            })?;
        } else {
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| DicecutError::Io {
                    context: format!("creating directory {}", parent.display()),
                    source: e,
                })?;
            }
            std::fs::copy(entry.path(), &dest_path).map_err(|e| DicecutError::Io {
                context: format!(
                    "copying {} to {}",
                    entry.path().display(),
                    dest_path.display()
                ),
                source: e,
            })?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_copies_files() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();

        std::fs::write(src.path().join("a.txt"), "alpha").unwrap();
        std::fs::write(src.path().join("b.txt"), "bravo").unwrap();

        overlay_dir(src.path(), dst.path()).unwrap();

        assert_eq!(
            std::fs::read_to_string(dst.path().join("a.txt")).unwrap(),
            "alpha"
        );
        assert_eq!(
            std::fs::read_to_string(dst.path().join("b.txt")).unwrap(),
            "bravo"
        );
    }

    #[test]
    fn overlay_overwrites_existing() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();

        std::fs::write(dst.path().join("a.txt"), "old").unwrap();
        std::fs::write(src.path().join("a.txt"), "new").unwrap();

        overlay_dir(src.path(), dst.path()).unwrap();

        assert_eq!(
            std::fs::read_to_string(dst.path().join("a.txt")).unwrap(),
            "new"
        );
    }

    #[test]
    fn overlay_creates_nested_dirs() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();

        std::fs::create_dir_all(src.path().join("sub/deep")).unwrap();
        std::fs::write(src.path().join("sub/deep/file.txt"), "deep").unwrap();

        overlay_dir(src.path(), dst.path()).unwrap();

        assert_eq!(
            std::fs::read_to_string(dst.path().join("sub/deep/file.txt")).unwrap(),
            "deep"
        );
    }

    #[test]
    fn overlay_noop_when_src_missing() {
        let dst = tempfile::tempdir().unwrap();
        let missing = dst.path().join("nonexistent");
        overlay_dir(&missing, dst.path()).unwrap();
    }
}
