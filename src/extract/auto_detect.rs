use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;
use std::sync::LazyLock;

use regex_lite::Regex;

static GO_MOD_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^module\s+(\S+)").unwrap());

static TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"[a-zA-Z][a-zA-Z0-9]*(?:[-_.][a-zA-Z0-9]+)+|[A-Z][a-z]+(?:[A-Z][a-z]+)+|[a-z]+(?:[A-Z][a-z]+)+|[A-Z]{2,}(?:_[A-Z]{2,})+",
    )
    .unwrap()
});

use super::scan::ScanResult;
use super::variants::split_into_words;

/// Confidence tier indicating how a candidate variable was detected.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfidenceTier {
    DirectoryName,
    ConfigFile,
    GitMetadata,
    FrequencyAnalysis,
}

impl std::fmt::Display for ConfidenceTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfidenceTier::DirectoryName => write!(f, "directory name"),
            ConfidenceTier::ConfigFile => write!(f, "config file"),
            ConfidenceTier::GitMetadata => write!(f, "git metadata"),
            ConfidenceTier::FrequencyAnalysis => write!(f, "frequency analysis"),
        }
    }
}

/// A candidate variable detected by auto-detection.
#[derive(Debug, Clone)]
pub struct DetectedCandidate {
    pub suggested_name: String,
    pub value: String,
    pub tier: ConfidenceTier,
    pub confidence: f64,
    pub reason: String,
    pub file_count: usize,
    pub total_occurrences: usize,
}

/// Result of running auto-detection.
#[derive(Debug)]
pub struct AutoDetectResult {
    pub candidates: Vec<DetectedCandidate>,
}

// ── Entry point ──────────────────────────────────────────────────────────

/// Run all 4 auto-detection tiers against a scanned project.
pub fn auto_detect(project_dir: &Path, scan_result: &ScanResult) -> AutoDetectResult {
    let mut candidates = Vec::new();

    // Tier 1: Directory name
    candidates.extend(detect_directory_name(project_dir, scan_result));

    // Tier 2: Ecosystem config files
    candidates.extend(detect_config_files(project_dir, scan_result));

    // Tier 3: Git metadata
    candidates.extend(detect_git_metadata(project_dir, scan_result));

    // Collect values already covered by tiers 1-3
    let covered_values: HashSet<String> =
        candidates.iter().map(|c| c.value.to_lowercase()).collect();

    // Tier 4: Frequency analysis
    candidates.extend(detect_frequency(scan_result, &covered_values));

    // Deduplicate by normalized word list, keeping highest confidence
    deduplicate_candidates(&mut candidates);

    // Sort by confidence descending
    candidates.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));

    AutoDetectResult { candidates }
}

// ── Tier 1: Directory name ───────────────────────────────────────────────

const GENERIC_DIR_NAMES: &[&str] = &[
    "src",
    "app",
    "project",
    "tmp",
    "temp",
    "build",
    "dist",
    "out",
    "output",
    "lib",
    "bin",
    "test",
    "tests",
    "example",
    "examples",
    "docs",
    "doc",
    "assets",
    "public",
    "static",
    "vendor",
    "node_modules",
    "target",
    "pkg",
    "cmd",
    "internal",
    "api",
    "web",
    "server",
    "client",
    "frontend",
    "backend",
    "service",
    "services",
    "workspace",
    "repo",
    "code",
];

fn detect_directory_name(project_dir: &Path, scan_result: &ScanResult) -> Vec<DetectedCandidate> {
    let dir_name = match project_dir.file_name() {
        Some(name) => name.to_string_lossy().to_string(),
        None => return vec![],
    };

    if GENERIC_DIR_NAMES.contains(&dir_name.to_lowercase().as_str()) {
        return vec![];
    }

    // Must have at least 2 chars
    if dir_name.len() < 2 {
        return vec![];
    }

    let (file_count, total_occurrences) = count_occurrences(&dir_name, scan_result);

    vec![DetectedCandidate {
        suggested_name: "project_name".to_string(),
        value: dir_name.clone(),
        tier: ConfidenceTier::DirectoryName,
        confidence: 0.95,
        reason: format!("directory name \"{}\"", dir_name),
        file_count,
        total_occurrences,
    }]
}

// ── Tier 2: Ecosystem config files ───────────────────────────────────────

fn detect_config_files(project_dir: &Path, scan_result: &ScanResult) -> Vec<DetectedCandidate> {
    let mut candidates = Vec::new();

    if let Some(mut c) = parse_cargo_toml(project_dir, scan_result) {
        candidates.append(&mut c);
    }
    if let Some(mut c) = parse_package_json(project_dir, scan_result) {
        candidates.append(&mut c);
    }
    if let Some(mut c) = parse_pyproject_toml(project_dir, scan_result) {
        candidates.append(&mut c);
    }
    if let Some(mut c) = parse_go_mod(project_dir, scan_result) {
        candidates.append(&mut c);
    }

    candidates
}

fn push_config_candidate(
    candidates: &mut Vec<DetectedCandidate>,
    value: &str,
    suggested_name: &str,
    confidence: f64,
    reason: &str,
    scan_result: &ScanResult,
) {
    let (file_count, total_occurrences) = count_occurrences(value, scan_result);
    candidates.push(DetectedCandidate {
        suggested_name: suggested_name.to_string(),
        value: value.to_string(),
        tier: ConfidenceTier::ConfigFile,
        confidence,
        reason: reason.to_string(),
        file_count,
        total_occurrences,
    });
}

fn parse_cargo_toml(
    project_dir: &Path,
    scan_result: &ScanResult,
) -> Option<Vec<DetectedCandidate>> {
    let path = project_dir.join("Cargo.toml");
    let content = std::fs::read_to_string(&path).ok()?;
    let parsed: toml::Value = content.parse().ok()?;

    let mut candidates = Vec::new();

    if let Some(name) = parsed
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
    {
        push_config_candidate(
            &mut candidates,
            name,
            "project_name",
            0.90,
            "Cargo.toml [package].name",
            scan_result,
        );
    }

    if let Some(version) = parsed
        .get("package")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
    {
        if !version.is_empty() {
            push_config_candidate(
                &mut candidates,
                version,
                "version",
                0.85,
                "Cargo.toml [package].version",
                scan_result,
            );
        }
    }

    if let Some(authors) = parsed
        .get("package")
        .and_then(|p| p.get("authors"))
        .and_then(|a| a.as_array())
    {
        if let Some(first) = authors.first().and_then(|a| a.as_str()) {
            let author = strip_email(first);
            if !author.is_empty() {
                push_config_candidate(
                    &mut candidates,
                    &author,
                    "author",
                    0.85,
                    "Cargo.toml [package].authors[0]",
                    scan_result,
                );
            }
        }
    }

    Some(candidates)
}

fn parse_package_json(
    project_dir: &Path,
    scan_result: &ScanResult,
) -> Option<Vec<DetectedCandidate>> {
    let path = project_dir.join("package.json");
    let content = std::fs::read_to_string(&path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;

    let mut candidates = Vec::new();

    if let Some(name) = parsed.get("name").and_then(|n| n.as_str()) {
        let clean_name = strip_npm_scope(name);
        push_config_candidate(
            &mut candidates,
            clean_name,
            "project_name",
            0.90,
            "package.json \"name\"",
            scan_result,
        );
    }

    if let Some(version) = parsed.get("version").and_then(|v| v.as_str()) {
        if !version.is_empty() {
            push_config_candidate(
                &mut candidates,
                version,
                "version",
                0.85,
                "package.json \"version\"",
                scan_result,
            );
        }
    }

    if let Some(author) = parsed.get("author") {
        let author_str = match author {
            serde_json::Value::String(s) => Some(strip_email(s)),
            serde_json::Value::Object(obj) => {
                obj.get("name").and_then(|n| n.as_str()).map(String::from)
            }
            _ => None,
        };
        if let Some(author_name) = author_str {
            if !author_name.is_empty() {
                push_config_candidate(
                    &mut candidates,
                    &author_name,
                    "author",
                    0.85,
                    "package.json \"author\"",
                    scan_result,
                );
            }
        }
    }

    Some(candidates)
}

fn parse_pyproject_toml(
    project_dir: &Path,
    scan_result: &ScanResult,
) -> Option<Vec<DetectedCandidate>> {
    let path = project_dir.join("pyproject.toml");
    let content = std::fs::read_to_string(&path).ok()?;
    let parsed: toml::Value = content.parse().ok()?;

    let mut candidates = Vec::new();

    if let Some(name) = parsed
        .get("project")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
    {
        push_config_candidate(
            &mut candidates,
            name,
            "project_name",
            0.90,
            "pyproject.toml [project].name",
            scan_result,
        );
    }

    if let Some(version) = parsed
        .get("project")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
    {
        if !version.is_empty() {
            push_config_candidate(
                &mut candidates,
                version,
                "version",
                0.85,
                "pyproject.toml [project].version",
                scan_result,
            );
        }
    }

    if let Some(authors) = parsed
        .get("project")
        .and_then(|p| p.get("authors"))
        .and_then(|a| a.as_array())
    {
        if let Some(first) = authors.first() {
            let author_name = first
                .get("name")
                .and_then(|n| n.as_str())
                .or_else(|| first.as_str())
                .map(strip_email);
            if let Some(name) = author_name {
                if !name.is_empty() {
                    push_config_candidate(
                        &mut candidates,
                        &name,
                        "author",
                        0.85,
                        "pyproject.toml [project].authors[0].name",
                        scan_result,
                    );
                }
            }
        }
    }

    Some(candidates)
}

fn parse_go_mod(project_dir: &Path, scan_result: &ScanResult) -> Option<Vec<DetectedCandidate>> {
    let path = project_dir.join("go.mod");
    let content = std::fs::read_to_string(&path).ok()?;

    let module_path = GO_MOD_RE.captures(&content)?.get(1)?.as_str();

    let segments: Vec<&str> = module_path.split('/').collect();

    // Extract last path segment as project name
    let name = segments.last().copied()?;
    if name.is_empty() {
        return None;
    }

    let mut candidates = Vec::new();

    push_config_candidate(
        &mut candidates,
        name,
        "project_name",
        0.90,
        &format!("go.mod module \"{}\"", module_path),
        scan_result,
    );

    // Extract org name (second-to-last segment for github.com/org/repo patterns)
    if segments.len() >= 3 {
        let org = segments[segments.len() - 2];
        if !org.is_empty() && org != name {
            let (_, org_total_occurrences) = count_occurrences(org, scan_result);
            if org_total_occurrences > 0 {
                push_config_candidate(
                    &mut candidates,
                    org,
                    "org_name",
                    0.85,
                    &format!("go.mod module org \"{}\"", org),
                    scan_result,
                );
            }
        }
    }

    Some(candidates)
}

// ── Tier 3: Git metadata ─────────────────────────────────────────────────

fn detect_git_metadata(project_dir: &Path, scan_result: &ScanResult) -> Vec<DetectedCandidate> {
    let mut candidates = Vec::new();

    // Try to get remote origin URL
    if let Some(url) = git_config_get(project_dir, "remote.origin.url") {
        if let Some(org) = parse_org_from_url(&url) {
            let (file_count, total_occurrences) = count_occurrences(&org, scan_result);
            // Only include if org name actually appears in files
            if total_occurrences > 0 {
                candidates.push(DetectedCandidate {
                    suggested_name: "org_name".to_string(),
                    value: org.clone(),
                    tier: ConfidenceTier::GitMetadata,
                    confidence: 0.70,
                    reason: format!("git remote org \"{}\"", org),
                    file_count,
                    total_occurrences,
                });
            }
        }
    }

    // Try to get user name
    if let Some(user_name) = git_config_get(project_dir, "user.name") {
        if !user_name.is_empty() {
            let (file_count, total_occurrences) = count_occurrences(&user_name, scan_result);
            candidates.push(DetectedCandidate {
                suggested_name: "author".to_string(),
                value: user_name.clone(),
                tier: ConfidenceTier::GitMetadata,
                confidence: 0.65,
                reason: format!("git config user.name \"{}\"", user_name),
                file_count,
                total_occurrences,
            });
        }
    }

    candidates
}

fn git_config_get(project_dir: &Path, key: &str) -> Option<String> {
    let output = Command::new("git")
        .arg("config")
        .arg("--get")
        .arg(key)
        .current_dir(project_dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn parse_org_from_url(url: &str) -> Option<String> {
    // SSH: git@github.com:org/repo.git
    if let Some(rest) = url.strip_prefix("git@") {
        let after_colon = rest.split(':').nth(1)?;
        let org = after_colon.split('/').next()?;
        if !org.is_empty() {
            return Some(org.to_string());
        }
    }

    // HTTPS: https://github.com/org/repo.git
    if url.starts_with("https://") || url.starts_with("http://") {
        let parts: Vec<&str> = url.split('/').collect();
        // https://host/org/repo → parts[3] is org
        if parts.len() >= 4 && !parts[3].is_empty() {
            return Some(parts[3].to_string());
        }
    }

    None
}

// ── Tier 4: Frequency analysis ───────────────────────────────────────────

fn detect_frequency(
    scan_result: &ScanResult,
    covered_values: &HashSet<String>,
) -> Vec<DetectedCandidate> {
    // Tokenize all text file content
    let mut token_file_map: HashMap<String, HashSet<usize>> = HashMap::new();
    let mut token_counts: HashMap<String, usize> = HashMap::new();

    for (file_idx, file) in scan_result.files.iter().enumerate() {
        if let Some(ref content) = file.content {
            for mat in TOKEN_RE.find_iter(content) {
                let token = mat.as_str().to_string();
                token_file_map
                    .entry(token.clone())
                    .or_default()
                    .insert(file_idx);
                *token_counts.entry(token).or_insert(0) += 1;
            }
        }
    }

    // Group tokens by normalized word list to find multi-variant clusters
    struct Cluster {
        literals: Vec<String>,
        total_occurrences: usize,
        files: HashSet<usize>,
    }

    let mut clusters: HashMap<String, Cluster> = HashMap::new();

    for (token, count) in &token_counts {
        let words = split_into_words(token);
        let normalized_key = words.join(" ");

        // Token must be at least 4 chars
        if token.len() < 4 {
            continue;
        }

        let cluster = clusters.entry(normalized_key).or_insert_with(|| Cluster {
            literals: Vec::new(),
            total_occurrences: 0,
            files: HashSet::new(),
        });

        if !cluster.literals.contains(token) {
            cluster.literals.push(token.clone());
        }
        cluster.total_occurrences += count;
        if let Some(file_set) = token_file_map.get(token) {
            cluster.files.extend(file_set);
        }
    }

    // Filter and convert to candidates
    let mut freq_candidates: Vec<DetectedCandidate> = Vec::new();

    for cluster in clusters.values() {
        // Must have ≥2 distinct case variants (the key multi-variant heuristic)
        if cluster.literals.len() < 2 {
            continue;
        }

        // Must have ≥3 total occurrences
        if cluster.total_occurrences < 3 {
            continue;
        }

        // Must appear in ≥2 files
        if cluster.files.len() < 2 {
            continue;
        }

        // Skip if already covered by higher tiers
        if cluster
            .literals
            .iter()
            .any(|l| covered_values.contains(&l.to_lowercase()))
        {
            continue;
        }

        let best_literal = &cluster.literals[0];
        let words = split_into_words(best_literal);
        let suggested_name = if words.len() <= 3 {
            words.join("_")
        } else {
            words[..3].join("_")
        };

        let file_count = cluster.files.len();
        freq_candidates.push(DetectedCandidate {
            suggested_name,
            value: best_literal.clone(),
            tier: ConfidenceTier::FrequencyAnalysis,
            confidence: 0.60,
            reason: format!(
                "{} occurrences across {} files, {} variant(s)",
                cluster.total_occurrences,
                file_count,
                cluster.literals.len()
            ),
            file_count,
            total_occurrences: cluster.total_occurrences,
        });
    }

    // Sort by file_count * total_occurrences descending, take top 5
    freq_candidates.sort_by(|a, b| {
        let score_a = a.file_count * a.total_occurrences;
        let score_b = b.file_count * b.total_occurrences;
        score_b.cmp(&score_a)
    });
    freq_candidates.truncate(5);

    freq_candidates
}

// ── Helpers ──────────────────────────────────────────────────────────────

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

pub fn strip_email(s: &str) -> String {
    // "Jane Doe <jane@example.com>" → "Jane Doe"
    if let Some(idx) = s.find('<') {
        s[..idx].trim().to_string()
    } else if s.contains('@') {
        // Bare email — use part before @
        s.split('@').next().unwrap_or("").trim().to_string()
    } else {
        s.trim().to_string()
    }
}

fn strip_npm_scope(name: &str) -> &str {
    if let Some(rest) = name.strip_prefix('@') {
        rest.split('/').nth(1).unwrap_or(name)
    } else {
        name
    }
}

fn deduplicate_candidates(candidates: &mut Vec<DetectedCandidate>) {
    // Only deduplicate by value (same literal from multiple tiers → keep highest confidence).
    // Name collisions (e.g., two different "author" candidates) are preserved
    // for the interactive/yes layer to resolve.
    let mut seen_value: HashMap<String, usize> = HashMap::new();
    let mut to_remove = Vec::new();

    for (i, candidate) in candidates.iter().enumerate() {
        let value_key = candidate.value.to_lowercase();
        if let Some(&prev_idx) = seen_value.get(&value_key) {
            if candidate.confidence > candidates[prev_idx].confidence {
                to_remove.push(prev_idx);
                seen_value.insert(value_key, i);
            } else {
                to_remove.push(i);
            }
        } else {
            seen_value.insert(value_key, i);
        }
    }

    to_remove.sort_unstable();
    to_remove.dedup();
    for idx in to_remove.into_iter().rev() {
        candidates.remove(idx);
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::scan::ScannedFile;
    use std::path::PathBuf;

    fn make_scan_result(files: Vec<(&str, &str)>) -> ScanResult {
        ScanResult {
            files: files
                .into_iter()
                .map(|(path, content)| ScannedFile {
                    relative_path: PathBuf::from(path),
                    absolute_path: PathBuf::from(path),
                    is_binary: false,
                    content: Some(content.to_string()),
                })
                .collect(),
            excluded_count: 0,
        }
    }

    // ── Tier 1 tests ─────────────────────────────────────────────────

    #[test]
    fn test_tier1_basic_dir_name() {
        let scan = make_scan_result(vec![
            ("README.md", "# my-widget\nA widget project"),
            ("src/lib.rs", "// my-widget core"),
        ]);
        let dir = PathBuf::from("/projects/my-widget");
        let candidates = detect_directory_name(&dir, &scan);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].value, "my-widget");
        assert_eq!(candidates[0].suggested_name, "project_name");
        assert_eq!(candidates[0].confidence, 0.95);
        assert!(candidates[0].total_occurrences >= 2);
    }

    #[test]
    fn test_tier1_generic_name_skipped() {
        let scan = make_scan_result(vec![("main.rs", "fn main() {}")]);
        let dir = PathBuf::from("/projects/src");
        let candidates = detect_directory_name(&dir, &scan);
        assert!(candidates.is_empty());
    }

    #[test]
    fn test_tier1_occurrence_counting() {
        let scan = make_scan_result(vec![
            ("a.txt", "hello hello hello"),
            ("b.txt", "hello world"),
        ]);
        let dir = PathBuf::from("/projects/hello");
        let candidates = detect_directory_name(&dir, &scan);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].file_count, 2);
        assert!(candidates[0].total_occurrences >= 4);
    }

    // ── Tier 2 tests ─────────────────────────────────────────────────

    #[test]
    fn test_tier2_cargo_toml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"data-pipeline\"\nversion = \"0.3.1\"\nauthors = [\"Alice <alice@example.com>\"]\n",
        )
        .unwrap();

        let scan = make_scan_result(vec![("src/main.rs", "data-pipeline runs here")]);
        let candidates = parse_cargo_toml(dir.path(), &scan).unwrap();

        assert!(candidates.iter().any(|c| c.value == "data-pipeline"));
        assert!(candidates
            .iter()
            .any(|c| c.value == "0.3.1" && c.suggested_name == "version" && c.confidence == 0.85));
        assert!(candidates.iter().any(|c| c.value == "Alice"));
    }

    #[test]
    fn test_tier2_package_json_with_scope() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name": "@myorg/cool-widget", "version": "2.1.0", "author": "Bob Smith <bob@example.com>"}"#,
        )
        .unwrap();

        let scan = make_scan_result(vec![("index.js", "cool-widget stuff")]);
        let candidates = parse_package_json(dir.path(), &scan).unwrap();

        let name_candidate = candidates
            .iter()
            .find(|c| c.suggested_name == "project_name")
            .unwrap();
        assert_eq!(name_candidate.value, "cool-widget");

        let version_candidate = candidates
            .iter()
            .find(|c| c.suggested_name == "version")
            .unwrap();
        assert_eq!(version_candidate.value, "2.1.0");
        assert_eq!(version_candidate.confidence, 0.85);

        let author_candidate = candidates
            .iter()
            .find(|c| c.suggested_name == "author")
            .unwrap();
        assert_eq!(author_candidate.value, "Bob Smith");
    }

    #[test]
    fn test_tier2_pyproject_toml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"my-tool\"\nversion = \"1.0.0\"\n\n[[project.authors]]\nname = \"Charlie\"\n",
        )
        .unwrap();

        let scan = make_scan_result(vec![("setup.py", "my-tool setup")]);
        let candidates = parse_pyproject_toml(dir.path(), &scan).unwrap();

        assert!(candidates.iter().any(|c| c.value == "my-tool"));
        assert!(candidates
            .iter()
            .any(|c| c.value == "1.0.0" && c.suggested_name == "version" && c.confidence == 0.85));
        assert!(candidates.iter().any(|c| c.value == "Charlie"));
    }

    #[test]
    fn test_tier2_go_mod() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module github.com/acme/my-service\n\ngo 1.21\n",
        )
        .unwrap();

        let scan = make_scan_result(vec![("main.go", "package main // my-service by acme")]);
        let candidates = parse_go_mod(dir.path(), &scan).unwrap();

        let project = candidates
            .iter()
            .find(|c| c.suggested_name == "project_name");
        assert!(project.is_some());
        assert_eq!(project.unwrap().value, "my-service");

        let org = candidates.iter().find(|c| c.suggested_name == "org_name");
        assert!(org.is_some(), "should extract org from go.mod module path");
        assert_eq!(org.unwrap().value, "acme");
    }

    #[test]
    fn test_tier2_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let scan = make_scan_result(vec![]);

        assert!(parse_cargo_toml(dir.path(), &scan).is_none());
        assert!(parse_package_json(dir.path(), &scan).is_none());
        assert!(parse_pyproject_toml(dir.path(), &scan).is_none());
        assert!(parse_go_mod(dir.path(), &scan).is_none());
    }

    #[test]
    fn test_tier2_malformed_cargo_toml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "this is not valid toml {{{}}}",
        )
        .unwrap();
        let scan = make_scan_result(vec![]);
        assert!(parse_cargo_toml(dir.path(), &scan).is_none());
    }

    #[test]
    fn test_tier2_version_missing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"no-version-crate\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name": "no-version-pkg"}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"no-version-py\"\n",
        )
        .unwrap();

        let scan = make_scan_result(vec![]);

        let cargo = parse_cargo_toml(dir.path(), &scan).unwrap();
        assert!(!cargo.iter().any(|c| c.suggested_name == "version"));

        let pkg = parse_package_json(dir.path(), &scan).unwrap();
        assert!(!pkg.iter().any(|c| c.suggested_name == "version"));

        let pyproj = parse_pyproject_toml(dir.path(), &scan).unwrap();
        assert!(!pyproj.iter().any(|c| c.suggested_name == "version"));
    }

    // ── Tier 3 tests ─────────────────────────────────────────────────

    #[test]
    fn test_parse_org_from_url_ssh() {
        assert_eq!(
            parse_org_from_url("git@github.com:acme-corp/my-repo.git"),
            Some("acme-corp".to_string())
        );
    }

    #[test]
    fn test_parse_org_from_url_https() {
        assert_eq!(
            parse_org_from_url("https://github.com/acme-corp/my-repo.git"),
            Some("acme-corp".to_string())
        );
    }

    #[test]
    fn test_strip_email_with_angle_brackets() {
        assert_eq!(strip_email("Jane Doe <jane@example.com>"), "Jane Doe");
    }

    #[test]
    fn test_strip_email_bare_email() {
        assert_eq!(strip_email("jane@example.com"), "jane");
    }

    #[test]
    fn test_strip_email_no_email() {
        assert_eq!(strip_email("Jane Doe"), "Jane Doe");
    }

    // ── Tier 4 tests ─────────────────────────────────────────────────

    #[test]
    fn test_frequency_finds_repeated_identifier() {
        let scan = make_scan_result(vec![
            ("a.txt", "data-pipeline is great\ndata-pipeline rocks"),
            ("b.txt", "use data_pipeline here\ndata_pipeline again"),
            ("c.txt", "DataPipeline class\nDataPipeline impl"),
            ("d.txt", "DATA_PIPELINE env var\nDATA_PIPELINE config"),
        ]);

        let covered = HashSet::new();
        let candidates = detect_frequency(&scan, &covered);

        assert!(!candidates.is_empty());
        // Should find "data-pipeline" cluster (multi-variant)
        let found = candidates.iter().any(|c| {
            let words = split_into_words(&c.value);
            words == vec!["data", "pipeline"]
        });
        assert!(
            found,
            "should find data-pipeline cluster, got: {:?}",
            candidates
        );
    }

    #[test]
    fn test_frequency_filters_short_tokens() {
        let scan = make_scan_result(vec![("a.txt", "ab cd ef gh"), ("b.txt", "ab cd ef gh")]);

        let covered = HashSet::new();
        let candidates = detect_frequency(&scan, &covered);

        assert!(candidates.is_empty(), "short tokens should be filtered");
    }

    #[test]
    fn test_frequency_skips_covered_values() {
        let scan = make_scan_result(vec![
            ("a.txt", "my-widget rocks"),
            ("b.txt", "my-widget is great"),
            ("c.txt", "my_widget too"),
        ]);

        let mut covered = HashSet::new();
        covered.insert("my-widget".to_string());
        let candidates = detect_frequency(&scan, &covered);

        let has_widget = candidates
            .iter()
            .any(|c| c.value.to_lowercase().contains("widget"));
        assert!(!has_widget, "covered values should be skipped");
    }

    #[test]
    fn test_frequency_requires_multi_variant() {
        // Single variant only — should NOT be detected even with many occurrences
        let scan = make_scan_result(vec![
            ("a.txt", "async_handler async_handler async_handler"),
            ("b.txt", "async_handler async_handler"),
            ("c.txt", "async_handler"),
        ]);

        let covered = HashSet::new();
        let candidates = detect_frequency(&scan, &covered);

        assert!(
            candidates.is_empty(),
            "single-variant tokens should be filtered, got: {:?}",
            candidates
        );
    }

    // ── Helper tests ─────────────────────────────────────────────────

    #[test]
    fn test_deduplication_keeps_highest_confidence() {
        let mut candidates = vec![
            DetectedCandidate {
                suggested_name: "project_name".to_string(),
                value: "my-app".to_string(),
                tier: ConfidenceTier::ConfigFile,
                confidence: 0.90,
                reason: "Cargo.toml".to_string(),
                file_count: 3,
                total_occurrences: 10,
            },
            DetectedCandidate {
                suggested_name: "project_name".to_string(),
                value: "my-app".to_string(),
                tier: ConfidenceTier::DirectoryName,
                confidence: 0.95,
                reason: "directory name".to_string(),
                file_count: 3,
                total_occurrences: 10,
            },
        ];

        deduplicate_candidates(&mut candidates);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].confidence, 0.95);
    }

    #[test]
    fn test_name_collisions_preserved() {
        let mut candidates = vec![
            DetectedCandidate {
                suggested_name: "author".to_string(),
                value: "Alice Johnson".to_string(),
                tier: ConfidenceTier::ConfigFile,
                confidence: 0.85,
                reason: "package.json".to_string(),
                file_count: 3,
                total_occurrences: 5,
            },
            DetectedCandidate {
                suggested_name: "author".to_string(),
                value: "Robert Roskam".to_string(),
                tier: ConfidenceTier::GitMetadata,
                confidence: 0.65,
                reason: "git config".to_string(),
                file_count: 0,
                total_occurrences: 0,
            },
        ];

        deduplicate_candidates(&mut candidates);
        assert_eq!(
            candidates.len(),
            2,
            "name collisions should be preserved for interactive resolution"
        );
    }

    #[test]
    fn test_strip_npm_scope() {
        assert_eq!(strip_npm_scope("@myorg/cool-widget"), "cool-widget");
        assert_eq!(strip_npm_scope("plain-package"), "plain-package");
    }

    #[test]
    fn test_auto_detect_integration() {
        let dir = tempfile::tempdir().unwrap();
        let project_dir = dir.path().join("my-widget");
        std::fs::create_dir(&project_dir).unwrap();
        std::fs::write(
            project_dir.join("README.md"),
            "# my-widget\nWelcome to my-widget",
        )
        .unwrap();
        std::fs::write(
            project_dir.join("lib.rs"),
            "pub mod my_widget;\nstruct MyWidget;",
        )
        .unwrap();

        let scan = crate::extract::scan::scan_project(&project_dir, &[], None).unwrap();
        let result = auto_detect(&project_dir, &scan);

        assert!(!result.candidates.is_empty());
        let project_name = result
            .candidates
            .iter()
            .find(|c| c.suggested_name == "project_name");
        assert!(project_name.is_some(), "should detect project_name");
        assert_eq!(project_name.unwrap().value, "my-widget");
    }
}
