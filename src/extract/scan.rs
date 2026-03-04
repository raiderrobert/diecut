use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use super::exclude::should_exclude;
use crate::render::file::is_binary_file;

/// A scanned file from the project directory.
#[derive(Debug, Clone)]
pub struct ScannedFile {
    /// Path relative to the project root.
    pub relative_path: PathBuf,
    /// Absolute path on disk.
    pub absolute_path: PathBuf,
    /// Whether the file is binary.
    pub is_binary: bool,
    /// File content (only loaded for text files).
    pub content: Option<String>,
}

/// Result of scanning a project directory.
#[derive(Debug)]
pub struct ScanResult {
    pub files: Vec<ScannedFile>,
    pub excluded_count: usize,
}

/// Scan a project directory, applying exclude patterns.
///
/// Returns all non-excluded files with their content loaded (for text files).
pub fn scan_project(project_dir: &Path, excludes: &[String]) -> crate::error::Result<ScanResult> {
    let project_dir = project_dir
        .canonicalize()
        .map_err(|e| crate::error::DicecutError::Io {
            context: format!("canonicalizing project directory {}", project_dir.display()),
            source: e,
        })?;

    let mut files = Vec::new();
    let mut excluded_count = 0;

    for entry in WalkDir::new(&project_dir).min_depth(1) {
        let entry = entry.map_err(|e| crate::error::DicecutError::Io {
            context: format!("walking project directory: {}", e),
            source: e
                .into_io_error()
                .unwrap_or_else(|| std::io::Error::other("walkdir error")),
        })?;

        // Skip directories (including symlinks to directories, e.g. pnpm's
        // node_modules/.pnpm uses symlinks that point to directories).
        if entry.file_type().is_dir() {
            continue;
        }
        if entry.path_is_symlink() && entry.path().is_dir() {
            continue;
        }

        let relative_path = entry
            .path()
            .strip_prefix(&project_dir)
            .unwrap_or(entry.path())
            .to_path_buf();

        if should_exclude(&relative_path, excludes) {
            excluded_count += 1;
            continue;
        }

        let absolute_path = entry.path().to_path_buf();

        let (is_binary, content) = if is_binary_file(&absolute_path) {
            (true, None)
        } else {
            match std::fs::read_to_string(&absolute_path) {
                Ok(s) => (false, Some(s)),
                Err(e) if e.kind() == std::io::ErrorKind::InvalidData => (true, None),
                Err(e) => {
                    return Err(crate::error::DicecutError::Io {
                        context: format!("reading file {}", absolute_path.display()),
                        source: e,
                    });
                }
            }
        };

        files.push(ScannedFile {
            relative_path,
            absolute_path,
            is_binary,
            content,
        });
    }

    Ok(ScanResult {
        files,
        excluded_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_project_basic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("README.md"), "# Hello").unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();

        let result = scan_project(dir.path(), &[]).unwrap();
        assert_eq!(result.files.len(), 2);
        assert_eq!(result.excluded_count, 0);
    }

    #[test]
    fn test_scan_project_with_excludes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("README.md"), "# Hello").unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        std::fs::write(dir.path().join(".git/config"), "").unwrap();

        let excludes = vec![".git".to_string()];
        let result = scan_project(dir.path(), &excludes).unwrap();
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.excluded_count, 1);
        assert_eq!(result.files[0].relative_path, PathBuf::from("README.md"));
    }

    #[cfg(unix)]
    #[test]
    fn test_scan_project_skips_symlinks_to_directories() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("real.txt"), "hello").unwrap();

        // Create a subdirectory and a symlink pointing to it
        let subdir = dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("nested.txt"), "nested").unwrap();
        std::os::unix::fs::symlink(&subdir, dir.path().join("link-to-dir")).unwrap();

        let result = scan_project(dir.path(), &[]).unwrap();
        // Should find real.txt and subdir/nested.txt, but NOT choke on link-to-dir
        let paths: Vec<String> = result
            .files
            .iter()
            .map(|f| f.relative_path.to_string_lossy().to_string())
            .collect();
        assert!(paths.contains(&"real.txt".to_string()));
        assert!(paths.contains(&"subdir/nested.txt".to_string()));
        assert!(!paths.iter().any(|p| p.contains("link-to-dir")));
    }

    #[test]
    fn test_scan_project_binary_detection() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("text.txt"), "hello").unwrap();
        std::fs::write(
            dir.path().join("binary.bin"),
            &(0..256).map(|i| i as u8).collect::<Vec<u8>>(),
        )
        .unwrap();

        let result = scan_project(dir.path(), &[]).unwrap();
        let text_file = result
            .files
            .iter()
            .find(|f| f.relative_path.to_string_lossy() == "text.txt")
            .unwrap();
        let binary_file = result
            .files
            .iter()
            .find(|f| f.relative_path.to_string_lossy() == "binary.bin")
            .unwrap();

        assert!(!text_file.is_binary);
        assert!(text_file.content.is_some());
        assert!(binary_file.is_binary);
        assert!(binary_file.content.is_none());
    }
}
