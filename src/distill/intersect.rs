use std::collections::HashMap;
use std::path::PathBuf;

use crate::extract::scan::{ScanResult, ScannedFile};

/// A file that appears in all scanned projects, with per-project content aligned by scan index.
pub struct AlignedFile {
    /// Path relative to each project root.
    pub relative_path: PathBuf,
    /// Per-project text content. `None` means binary (or absent, but absent files are excluded).
    pub contents: Vec<Option<String>>,
    /// Per-project raw bytes for binary files. `None` for text files.
    pub raw_bytes: Vec<Option<Vec<u8>>>,
    /// `true` if ANY project's copy of this file is binary.
    pub any_binary: bool,
}

/// Intersect multiple scan results, keeping only files present in ALL scans.
///
/// Returns one `AlignedFile` per common relative path, with per-project content
/// stored in the same order as `scans`. Results are sorted by relative path.
pub fn intersect_scans(scans: &[ScanResult]) -> Vec<AlignedFile> {
    if scans.is_empty() {
        return Vec::new();
    }

    // Count how many scans contain each relative path.
    let mut path_count: HashMap<&PathBuf, usize> = HashMap::new();
    for scan in scans {
        for file in &scan.files {
            *path_count.entry(&file.relative_path).or_insert(0) += 1;
        }
    }

    let num_scans = scans.len();

    // Collect paths present in every scan.
    let mut common_paths: Vec<&PathBuf> = path_count
        .into_iter()
        .filter_map(|(path, count)| if count == num_scans { Some(path) } else { None })
        .collect();

    common_paths.sort();

    // Build an AlignedFile for each common path.
    common_paths
        .into_iter()
        .map(|path| {
            let mut contents: Vec<Option<String>> = Vec::with_capacity(num_scans);
            let mut raw_bytes: Vec<Option<Vec<u8>>> = Vec::with_capacity(num_scans);
            let mut any_binary = false;

            for scan in scans {
                let file: &ScannedFile = scan
                    .files
                    .iter()
                    .find(|f| &f.relative_path == path)
                    .expect("path was counted as present in every scan");

                if file.is_binary {
                    any_binary = true;
                    contents.push(None);
                    let bytes = std::fs::read(&file.absolute_path).ok();
                    raw_bytes.push(bytes);
                } else {
                    contents.push(file.content.clone());
                    raw_bytes.push(None);
                }
            }

            AlignedFile {
                relative_path: path.clone(),
                contents,
                raw_bytes,
                any_binary,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::scan::ScannedFile;
    use rstest::rstest;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn make_text_file(relative: &str, absolute: &str, content: &str) -> ScannedFile {
        ScannedFile {
            relative_path: PathBuf::from(relative),
            absolute_path: PathBuf::from(absolute),
            is_binary: false,
            content: Some(content.to_string()),
        }
    }

    fn make_binary_file(relative: &str, absolute: PathBuf) -> ScannedFile {
        ScannedFile {
            relative_path: PathBuf::from(relative),
            absolute_path: absolute,
            is_binary: true,
            content: None,
        }
    }

    fn make_scan(files: Vec<ScannedFile>) -> ScanResult {
        ScanResult {
            files,
            excluded_count: 0,
        }
    }

    #[test]
    fn common_files_kept_unique_files_discarded() {
        let scans = vec![
            make_scan(vec![
                make_text_file("README.md", "/a/README.md", "# A"),
                make_text_file("only_in_a.txt", "/a/only_in_a.txt", "unique"),
            ]),
            make_scan(vec![
                make_text_file("README.md", "/b/README.md", "# B"),
                make_text_file("only_in_b.txt", "/b/only_in_b.txt", "unique"),
            ]),
        ];

        let result = intersect_scans(&scans);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].relative_path, PathBuf::from("README.md"));
        assert_eq!(result[0].contents[0], Some("# A".to_string()));
        assert_eq!(result[0].contents[1], Some("# B".to_string()));
    }

    #[test]
    fn empty_intersection_when_no_common_files() {
        let scans = vec![
            make_scan(vec![make_text_file("only_a.txt", "/a/only_a.txt", "a")]),
            make_scan(vec![make_text_file("only_b.txt", "/b/only_b.txt", "b")]),
        ];

        let result = intersect_scans(&scans);

        assert!(result.is_empty());
    }

    #[test]
    fn binary_file_detection_any_binary_flag() {
        // Write real binary bytes so std::fs::read works.
        let dir_a = tempdir().unwrap();
        let dir_b = tempdir().unwrap();

        let bin_a = dir_a.path().join("logo.png");
        let bin_b = dir_b.path().join("logo.png");
        std::fs::write(&bin_a, b"\x89PNG\r\n").unwrap();
        std::fs::write(&bin_b, b"\x89PNG\r\n").unwrap();

        let scans = vec![
            make_scan(vec![
                make_binary_file("logo.png", bin_a),
                make_text_file("main.rs", "/a/main.rs", "fn main() {}"),
            ]),
            make_scan(vec![
                make_binary_file("logo.png", bin_b),
                make_text_file("main.rs", "/b/main.rs", "fn main() {}"),
            ]),
        ];

        let result = intersect_scans(&scans);
        assert_eq!(result.len(), 2);

        let logo = result
            .iter()
            .find(|f| f.relative_path == PathBuf::from("logo.png"))
            .unwrap();
        assert!(logo.any_binary);
        assert!(logo.raw_bytes[0].is_some());
        assert!(logo.raw_bytes[1].is_some());
        assert_eq!(logo.contents[0], None);
        assert_eq!(logo.contents[1], None);

        let main_rs = result
            .iter()
            .find(|f| f.relative_path == PathBuf::from("main.rs"))
            .unwrap();
        assert!(!main_rs.any_binary);
    }

    #[rstest]
    #[case("shared.txt", true)]
    #[case("unique_c.txt", false)]
    fn three_project_intersection(#[case] path: &str, #[case] expected_present: bool) {
        let scans = vec![
            make_scan(vec![
                make_text_file("shared.txt", "/a/shared.txt", "content a"),
                make_text_file("only_a.txt", "/a/only_a.txt", "only a"),
            ]),
            make_scan(vec![
                make_text_file("shared.txt", "/b/shared.txt", "content b"),
                make_text_file("only_b.txt", "/b/only_b.txt", "only b"),
            ]),
            make_scan(vec![
                make_text_file("shared.txt", "/c/shared.txt", "content c"),
                make_text_file("unique_c.txt", "/c/unique_c.txt", "unique c"),
            ]),
        ];

        let result = intersect_scans(&scans);

        let found = result
            .iter()
            .any(|f| f.relative_path == PathBuf::from(path));
        assert_eq!(found, expected_present);

        if expected_present {
            let file = result
                .iter()
                .find(|f| f.relative_path == PathBuf::from(path))
                .unwrap();
            assert_eq!(file.contents.len(), 3);
        }
    }

    #[test]
    fn empty_scans_slice_returns_empty() {
        let result = intersect_scans(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn results_sorted_by_relative_path() {
        let scans = vec![
            make_scan(vec![
                make_text_file("z.txt", "/a/z.txt", "z"),
                make_text_file("a.txt", "/a/a.txt", "a"),
                make_text_file("m.txt", "/a/m.txt", "m"),
            ]),
            make_scan(vec![
                make_text_file("z.txt", "/b/z.txt", "z"),
                make_text_file("a.txt", "/b/a.txt", "a"),
                make_text_file("m.txt", "/b/m.txt", "m"),
            ]),
        ];

        let result = intersect_scans(&scans);

        let paths: Vec<&PathBuf> = result.iter().map(|f| &f.relative_path).collect();
        let mut sorted = paths.clone();
        sorted.sort();
        assert_eq!(paths, sorted);
    }
}
