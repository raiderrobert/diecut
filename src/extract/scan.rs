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

/// Count how many files contain `value` and the total number of hits across all files.
pub fn count_occurrences(value: &str, scan_result: &ScanResult) -> (usize, usize) {
    let mut file_count = 0;
    let mut total = 0;

    for file in &scan_result.files {
        let mut counted_file = false;

        if let Some(ref content) = file.content {
            let hits = content.matches(value).count();
            if hits > 0 {
                file_count += 1;
                counted_file = true;
                total += hits;
            }
        }

        let path_str = file.relative_path.to_string_lossy();
        let path_hits = path_str.matches(value).count();
        if path_hits > 0 {
            total += path_hits;
            if !counted_file {
                file_count += 1;
            }
        }
    }

    (file_count, total)
}
