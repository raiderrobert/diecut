use std::collections::HashSet;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::error::{DicecutError, Result};

pub fn collect_files(dir: &Path) -> Result<HashSet<PathBuf>> {
    let mut files = HashSet::new();
    if !dir.exists() {
        return Ok(files);
    }

    for entry in WalkDir::new(dir)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let rel = entry
                .path()
                .strip_prefix(dir)
                .expect("entry must be under dir");

            if rel.to_string_lossy() == ".diecut-answers.toml" {
                continue;
            }

            files.insert(rel.to_path_buf());
        }
    }

    Ok(files)
}

pub fn files_equal(path_a: &Path, path_b: &Path) -> Result<bool> {
    let content_a = read_file(path_a)?;
    let content_b = read_file(path_b)?;
    Ok(content_a == content_b)
}

fn read_file(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|e| DicecutError::Io {
        context: format!("reading {}", path.display()),
        source: e,
    })
}

pub fn unified_diff(old: &str, new: &str, path: &Path) -> String {
    use similar::TextDiff;

    let diff = TextDiff::from_lines(old, new);
    let mut output = String::new();

    output.push_str(&format!(
        "--- a/{}\n+++ b/{}\n",
        path.display(),
        path.display()
    ));

    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        output.push_str(&format!("{hunk}"));
    }

    output
}
