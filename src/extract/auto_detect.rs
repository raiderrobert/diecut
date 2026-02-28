use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;

use regex_lite::Regex;

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

struct TokenCluster {
    normalized: Vec<String>,
    literals: Vec<String>,
    total_occurrences: usize,
    file_count: usize,
    matches_dir_name: bool,
    in_config_value: bool,
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

    // Collect config values for frequency analysis boosting
    let config_values: HashSet<String> = candidates
        .iter()
        .filter(|c| c.tier == ConfidenceTier::ConfigFile)
        .map(|c| c.value.to_lowercase())
        .collect();

    let dir_name = project_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // Tier 4: Frequency analysis
    candidates.extend(detect_frequency(
        scan_result,
        &covered_values,
        &config_values,
        &dir_name,
    ));

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
        let (file_count, total_occurrences) = count_occurrences(name, scan_result);
        candidates.push(DetectedCandidate {
            suggested_name: "project_name".to_string(),
            value: name.to_string(),
            tier: ConfidenceTier::ConfigFile,
            confidence: 0.90,
            reason: "Cargo.toml [package].name".to_string(),
            file_count,
            total_occurrences,
        });
    }

    if let Some(authors) = parsed
        .get("package")
        .and_then(|p| p.get("authors"))
        .and_then(|a| a.as_array())
    {
        if let Some(first) = authors.first().and_then(|a| a.as_str()) {
            let author = strip_email(first);
            if !author.is_empty() {
                let (file_count, total_occurrences) = count_occurrences(&author, scan_result);
                candidates.push(DetectedCandidate {
                    suggested_name: "author".to_string(),
                    value: author.clone(),
                    tier: ConfidenceTier::ConfigFile,
                    confidence: 0.85,
                    reason: "Cargo.toml [package].authors[0]".to_string(),
                    file_count,
                    total_occurrences,
                });
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
        // Strip npm scope @org/
        let clean_name = strip_npm_scope(name);
        let (file_count, total_occurrences) = count_occurrences(clean_name, scan_result);
        candidates.push(DetectedCandidate {
            suggested_name: "project_name".to_string(),
            value: clean_name.to_string(),
            tier: ConfidenceTier::ConfigFile,
            confidence: 0.90,
            reason: "package.json \"name\"".to_string(),
            file_count,
            total_occurrences,
        });
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
                let (file_count, total_occurrences) = count_occurrences(&author_name, scan_result);
                candidates.push(DetectedCandidate {
                    suggested_name: "author".to_string(),
                    value: author_name,
                    tier: ConfidenceTier::ConfigFile,
                    confidence: 0.85,
                    reason: "package.json \"author\"".to_string(),
                    file_count,
                    total_occurrences,
                });
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
        let (file_count, total_occurrences) = count_occurrences(name, scan_result);
        candidates.push(DetectedCandidate {
            suggested_name: "project_name".to_string(),
            value: name.to_string(),
            tier: ConfidenceTier::ConfigFile,
            confidence: 0.90,
            reason: "pyproject.toml [project].name".to_string(),
            file_count,
            total_occurrences,
        });
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
                    let (file_count, total_occurrences) = count_occurrences(&name, scan_result);
                    candidates.push(DetectedCandidate {
                        suggested_name: "author".to_string(),
                        value: name,
                        tier: ConfidenceTier::ConfigFile,
                        confidence: 0.85,
                        reason: "pyproject.toml [project].authors[0].name".to_string(),
                        file_count,
                        total_occurrences,
                    });
                }
            }
        }
    }

    Some(candidates)
}

fn parse_go_mod(project_dir: &Path, scan_result: &ScanResult) -> Option<Vec<DetectedCandidate>> {
    let path = project_dir.join("go.mod");
    let content = std::fs::read_to_string(&path).ok()?;

    let re = Regex::new(r"^module\s+(\S+)").unwrap();
    let module_path = re.captures(&content)?.get(1)?.as_str();

    let segments: Vec<&str> = module_path.split('/').collect();

    // Extract last path segment as project name
    let name = segments.last().copied()?;
    if name.is_empty() {
        return None;
    }

    let mut candidates = Vec::new();

    let (file_count, total_occurrences) = count_occurrences(name, scan_result);
    candidates.push(DetectedCandidate {
        suggested_name: "project_name".to_string(),
        value: name.to_string(),
        tier: ConfidenceTier::ConfigFile,
        confidence: 0.90,
        reason: format!("go.mod module \"{}\"", module_path),
        file_count,
        total_occurrences,
    });

    // Extract org name (second-to-last segment for github.com/org/repo patterns)
    if segments.len() >= 3 {
        let org = segments[segments.len() - 2];
        if !org.is_empty() && org != name {
            let (org_file_count, org_total_occurrences) = count_occurrences(org, scan_result);
            if org_total_occurrences > 0 {
                candidates.push(DetectedCandidate {
                    suggested_name: "org_name".to_string(),
                    value: org.to_string(),
                    tier: ConfidenceTier::ConfigFile,
                    confidence: 0.85,
                    reason: format!("go.mod module org \"{}\"", org),
                    file_count: org_file_count,
                    total_occurrences: org_total_occurrences,
                });
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
    config_values: &HashSet<String>,
    dir_name: &str,
) -> Vec<DetectedCandidate> {
    // Tokenize all text file content
    let token_re = Regex::new(
        r"[a-zA-Z][a-zA-Z0-9]*(?:[-_.][a-zA-Z0-9]+)+|[A-Z][a-z]+(?:[A-Z][a-z]+)+|[a-z]+(?:[A-Z][a-z]+)+|[A-Z]{2,}(?:_[A-Z]{2,})+"
    ).unwrap();

    let mut token_file_map: HashMap<String, HashSet<usize>> = HashMap::new();
    let mut token_counts: HashMap<String, usize> = HashMap::new();

    for (file_idx, file) in scan_result.files.iter().enumerate() {
        if let Some(ref content) = file.content {
            for mat in token_re.find_iter(content) {
                let token = mat.as_str().to_string();
                token_file_map
                    .entry(token.clone())
                    .or_default()
                    .insert(file_idx);
                *token_counts.entry(token).or_insert(0) += 1;
            }
        }
    }

    // Build clusters by normalized word list
    let mut clusters: HashMap<String, TokenCluster> = HashMap::new();

    for (token, count) in &token_counts {
        let words = split_into_words(token);

        // Filter noise
        if words.iter().all(|w| w.len() < 3) {
            continue;
        }
        if is_noise_token(token, &words) {
            continue;
        }

        let normalized_key = words.join(" ");

        let file_count = token_file_map.get(token).map(|s| s.len()).unwrap_or(0);

        // Skip single-occurrence-single-file tokens
        if *count == 1 && file_count <= 1 {
            continue;
        }

        let matches_dir =
            normalized_key == split_into_words(dir_name).join(" ") && !dir_name.is_empty();
        let in_config = config_values.contains(&token.to_lowercase());

        let cluster = clusters
            .entry(normalized_key.clone())
            .or_insert_with(|| TokenCluster {
                normalized: words.clone(),
                literals: Vec::new(),
                total_occurrences: 0,
                file_count: 0,
                matches_dir_name: false,
                in_config_value: false,
            });

        if !cluster.literals.contains(token) {
            cluster.literals.push(token.clone());
        }
        cluster.total_occurrences += count;
        // Merge file sets for accurate file_count
        let files_for_token = token_file_map.get(token).map(|s| s.len()).unwrap_or(0);
        if files_for_token > cluster.file_count {
            cluster.file_count = files_for_token;
        }
        cluster.matches_dir_name = cluster.matches_dir_name || matches_dir;
        cluster.in_config_value = cluster.in_config_value || in_config;
    }

    // Merge near-misses using Levenshtein distance
    merge_similar_clusters(&mut clusters);

    // Score and convert to candidates
    let mut freq_candidates: Vec<DetectedCandidate> = Vec::new();

    for (key, cluster) in &clusters {
        // Skip if already covered by higher tiers
        if cluster
            .literals
            .iter()
            .any(|l| covered_values.contains(&l.to_lowercase()))
        {
            continue;
        }

        let score = score_cluster(cluster);

        // Filter low-scoring candidates
        if score < 0.30 {
            continue;
        }

        let best_literal = &cluster.literals[0];
        let suggested_name = suggest_variable_name(&cluster.normalized, key);

        freq_candidates.push(DetectedCandidate {
            suggested_name,
            value: best_literal.clone(),
            tier: ConfidenceTier::FrequencyAnalysis,
            confidence: score,
            reason: format!(
                "{} occurrences across {} files, {} variant(s)",
                cluster.total_occurrences,
                cluster.file_count,
                cluster.literals.len()
            ),
            file_count: cluster.file_count,
            total_occurrences: cluster.total_occurrences,
        });
    }

    // Sort by confidence, take top 5
    freq_candidates.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
    freq_candidates.truncate(5);

    freq_candidates
}

fn score_cluster(cluster: &TokenCluster) -> f64 {
    // Occurrence count (log-scaled, 0.0..1.0)
    let occ_score = (cluster.total_occurrences as f64).ln_1p() / 10.0_f64.ln_1p();
    let occ_score = occ_score.min(1.0);

    // File spread (log-scaled, 0.0..1.0)
    let file_score = (cluster.file_count as f64).ln_1p() / 10.0_f64.ln_1p();
    let file_score = file_score.min(1.0);

    // Variant diversity
    let variant_score = match cluster.literals.len() {
        0 | 1 => 0.0,
        2 => 0.5,
        3 => 0.75,
        _ => 1.0,
    };

    // Directory name match (binary)
    let dir_score = if cluster.matches_dir_name { 1.0 } else { 0.0 };

    // Config value match (binary)
    let config_score = if cluster.in_config_value { 1.0 } else { 0.0 };

    0.15 * occ_score
        + 0.20 * file_score
        + 0.35 * variant_score
        + 0.20 * dir_score
        + 0.10 * config_score
}

fn merge_similar_clusters(clusters: &mut HashMap<String, TokenCluster>) {
    let keys: Vec<String> = clusters.keys().cloned().collect();
    let mut merge_map: HashMap<String, String> = HashMap::new();

    for i in 0..keys.len() {
        for j in (i + 1)..keys.len() {
            if merge_map.contains_key(&keys[j]) {
                continue;
            }
            let dist = strsim::levenshtein(&keys[i], &keys[j]);
            if dist <= 1 {
                let size_i = clusters
                    .get(&keys[i])
                    .map(|c| c.total_occurrences)
                    .unwrap_or(0);
                let size_j = clusters
                    .get(&keys[j])
                    .map(|c| c.total_occurrences)
                    .unwrap_or(0);
                if size_i >= size_j {
                    merge_map.insert(keys[j].clone(), keys[i].clone());
                } else {
                    merge_map.insert(keys[i].clone(), keys[j].clone());
                }
            }
        }
    }

    // Resolve merge chains: if A→B and B→C, then A→C
    let resolved: HashMap<String, String> = merge_map
        .keys()
        .map(|k| {
            let mut target = merge_map[k].clone();
            while let Some(next) = merge_map.get(&target) {
                target = next.clone();
            }
            (k.clone(), target)
        })
        .collect();

    for (from, to) in &resolved {
        if let Some(removed) = clusters.remove(from) {
            if let Some(target) = clusters.get_mut(to) {
                for lit in removed.literals {
                    if !target.literals.contains(&lit) {
                        target.literals.push(lit);
                    }
                }
                target.total_occurrences += removed.total_occurrences;
                if removed.file_count > target.file_count {
                    target.file_count = removed.file_count;
                }
                target.matches_dir_name = target.matches_dir_name || removed.matches_dir_name;
                target.in_config_value = target.in_config_value || removed.in_config_value;
            }
        }
    }
}

fn suggest_variable_name(words: &[String], _key: &str) -> String {
    if words.len() <= 3 {
        words.join("_")
    } else {
        // Truncate long names
        words[..3].join("_")
    }
}

// ── Noise filtering ──────────────────────────────────────────────────────

fn is_noise_token(token: &str, words: &[String]) -> bool {
    let lower = token.to_lowercase();

    // Too short
    if lower.len() < 3 {
        return true;
    }

    // Language keywords
    if LANGUAGE_KEYWORDS.contains(&lower.as_str()) {
        return true;
    }

    // Common library names
    if COMMON_LIBRARIES.contains(&lower.as_str()) {
        return true;
    }

    // Stopwords (individual words)
    if words.len() == 1 && STOPWORDS.contains(&lower.as_str()) {
        return true;
    }

    // All words are stopwords, file-format words, or very short
    if words.iter().all(|w| {
        w.len() < 3 || STOPWORDS.contains(&w.as_str()) || FILE_FORMAT_WORDS.contains(&w.as_str())
    }) {
        return true;
    }

    false
}

const FILE_FORMAT_WORDS: &[&str] = &[
    "toml", "json", "yaml", "yml", "xml", "csv", "html", "css", "md", "txt", "log", "cfg", "ini",
    "env", "lock", "mod", "rs", "js", "ts", "py", "go", "rb", "java", "kt", "swift", "cpp", "hpp",
    "vue", "jsx", "tsx",
];

const LANGUAGE_KEYWORDS: &[&str] = &[
    // Rust
    "async",
    "await",
    "break",
    "const",
    "continue",
    "crate",
    "dyn",
    "else",
    "enum",
    "extern",
    "false",
    "fn",
    "for",
    "if",
    "impl",
    "in",
    "let",
    "loop",
    "match",
    "mod",
    "move",
    "mut",
    "pub",
    "ref",
    "return",
    "self",
    "static",
    "struct",
    "super",
    "trait",
    "true",
    "type",
    "unsafe",
    "use",
    "where",
    "while",
    "yield",
    // JS/TS
    "abstract",
    "arguments",
    "boolean",
    "byte",
    "case",
    "catch",
    "char",
    "class",
    "debugger",
    "default",
    "delete",
    "do",
    "double",
    "eval",
    "export",
    "extends",
    "final",
    "finally",
    "float",
    "function",
    "goto",
    "implements",
    "import",
    "instanceof",
    "int",
    "interface",
    "long",
    "native",
    "new",
    "null",
    "package",
    "private",
    "protected",
    "public",
    "short",
    "switch",
    "synchronized",
    "this",
    "throw",
    "throws",
    "transient",
    "try",
    "typeof",
    "undefined",
    "var",
    "void",
    "volatile",
    "with",
    // Python
    "and",
    "as",
    "assert",
    "class",
    "def",
    "del",
    "elif",
    "except",
    "exec",
    "from",
    "global",
    "is",
    "lambda",
    "nonlocal",
    "not",
    "or",
    "pass",
    "print",
    "raise",
    "with",
    "yield",
    // Go
    "chan",
    "defer",
    "fallthrough",
    "go",
    "goroutine",
    "interface",
    "map",
    "range",
    "select",
    "func",
];

const COMMON_LIBRARIES: &[&str] = &[
    "react",
    "redux",
    "webpack",
    "babel",
    "eslint",
    "prettier",
    "jest",
    "mocha",
    "chai",
    "express",
    "fastify",
    "next",
    "nuxt",
    "vue",
    "angular",
    "svelte",
    "serde",
    "tokio",
    "actix",
    "axum",
    "clap",
    "anyhow",
    "thiserror",
    "tracing",
    "reqwest",
    "hyper",
    "warp",
    "rocket",
    "diesel",
    "sqlx",
    "django",
    "flask",
    "fastapi",
    "pytest",
    "numpy",
    "pandas",
    "scipy",
    "spring",
    "hibernate",
    "junit",
    "maven",
    "gradle",
    "gin",
    "echo",
    "fiber",
    "gorm",
    "lodash",
    "axios",
    "moment",
    "dayjs",
    "ramda",
    "underscore",
    "tailwind",
    "bootstrap",
    "material",
    "typescript",
    "javascript",
    "python",
    "golang",
    "rustlang",
];

const STOPWORDS: &[&str] = &[
    // English stopwords
    "the",
    "and",
    "for",
    "are",
    "but",
    "not",
    "you",
    "all",
    "can",
    "had",
    "her",
    "was",
    "one",
    "our",
    "out",
    "get",
    "set",
    "has",
    "his",
    "how",
    "its",
    "let",
    "may",
    "new",
    "now",
    "old",
    "see",
    "way",
    "who",
    "did",
    "got",
    "has",
    "him",
    "into",
    "just",
    "like",
    "make",
    "many",
    "some",
    "than",
    "them",
    "then",
    "very",
    "when",
    "with",
    "have",
    "from",
    "been",
    "also",
    "each",
    "that",
    "this",
    "will",
    "your",
    "what",
    "which",
    "their",
    "about",
    "would",
    "there",
    "could",
    "other",
    "after",
    "first",
    "these",
    "those",
    "being",
    "where",
    "should",
    "because",
    // Short generic words common in code identifiers
    "my",
    "no",
    "is",
    "on",
    "in",
    "to",
    "by",
    "do",
    "up",
    "so",
    "or",
    "app",
    "run",
    "dry",
    "log",
    "cmd",
    "arg",
    "env",
    "dir",
    "key",
    "map",
    "max",
    "min",
    "raw",
    "ref",
    "src",
    "str",
    "tmp",
    "url",
    "var",
    "buf",
    "msg",
    "req",
    "res",
    "err",
    "pkg",
    "lib",
    "bin",
    "fmt",
    "ctx",
    "cfg",
    "opt",
    "val",
    "idx",
    "len",
    "ptr",
    "num",
    "std",
    "gen",
    "pre",
    "sub",
    // Programming type/concept words
    "string",
    "number",
    "bool",
    "boolean",
    "array",
    "object",
    "value",
    "result",
    "error",
    "option",
    "none",
    "some",
    "true",
    "false",
    "null",
    "undefined",
    "file",
    "path",
    "name",
    "type",
    "data",
    "info",
    "list",
    "item",
    "node",
    "index",
    "count",
    "size",
    "length",
    "config",
    "settings",
    "options",
    "input",
    "output",
    "source",
    "target",
    "test",
    "main",
    "init",
    "setup",
    "todo",
    "fixme",
    "hack",
    "note",
    "warning",
    "debug",
    "trace",
    "level",
    "mode",
    "flag",
    "status",
    "state",
    "cache",
    "hook",
    "hooks",
];

// ── Helpers ──────────────────────────────────────────────────────────────

fn count_occurrences(value: &str, scan_result: &ScanResult) -> (usize, usize) {
    let mut file_count = 0;
    let mut total = 0;

    for file in &scan_result.files {
        if let Some(ref content) = file.content {
            let hits = content.matches(value).count();
            if hits > 0 {
                file_count += 1;
                total += hits;
            }
        }
        // Also check path
        let path_str = file.relative_path.to_string_lossy();
        let path_hits = path_str.matches(value).count();
        total += path_hits;
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
            "[package]\nname = \"data-pipeline\"\nauthors = [\"Alice <alice@example.com>\"]\n",
        )
        .unwrap();

        let scan = make_scan_result(vec![("src/main.rs", "data-pipeline runs here")]);
        let candidates = parse_cargo_toml(dir.path(), &scan).unwrap();

        assert!(candidates.iter().any(|c| c.value == "data-pipeline"));
        assert!(candidates.iter().any(|c| c.value == "Alice"));
    }

    #[test]
    fn test_tier2_package_json_with_scope() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name": "@myorg/cool-widget", "author": "Bob Smith <bob@example.com>"}"#,
        )
        .unwrap();

        let scan = make_scan_result(vec![("index.js", "cool-widget stuff")]);
        let candidates = parse_package_json(dir.path(), &scan).unwrap();

        let name_candidate = candidates
            .iter()
            .find(|c| c.suggested_name == "project_name")
            .unwrap();
        assert_eq!(name_candidate.value, "cool-widget");

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
            "[project]\nname = \"my-tool\"\n\n[[project.authors]]\nname = \"Charlie\"\n",
        )
        .unwrap();

        let scan = make_scan_result(vec![("setup.py", "my-tool setup")]);
        let candidates = parse_pyproject_toml(dir.path(), &scan).unwrap();

        assert!(candidates.iter().any(|c| c.value == "my-tool"));
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
        let config_vals = HashSet::new();
        let candidates = detect_frequency(&scan, &covered, &config_vals, "");

        assert!(!candidates.is_empty());
        // Should find "data-pipeline" cluster
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
    fn test_frequency_filters_keywords() {
        let scan = make_scan_result(vec![
            ("a.rs", "fn async_handler() {}"),
            ("b.rs", "fn async_handler() {}"),
            ("c.rs", "fn async_handler() {}"),
        ]);

        let covered = HashSet::new();
        let config_vals = HashSet::new();
        let candidates = detect_frequency(&scan, &covered, &config_vals, "");

        // "async" alone should be filtered
        for c in &candidates {
            let lower = c.value.to_lowercase();
            assert!(
                !LANGUAGE_KEYWORDS.contains(&lower.as_str())
                    || c.value.contains('-')
                    || c.value.contains('_')
            );
        }
    }

    #[test]
    fn test_frequency_filters_short_tokens() {
        let scan = make_scan_result(vec![("a.txt", "ab cd ef gh"), ("b.txt", "ab cd ef gh")]);

        let covered = HashSet::new();
        let config_vals = HashSet::new();
        let candidates = detect_frequency(&scan, &covered, &config_vals, "");

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
        let config_vals = HashSet::new();
        let candidates = detect_frequency(&scan, &covered, &config_vals, "");

        let has_widget = candidates
            .iter()
            .any(|c| c.value.to_lowercase().contains("widget"));
        assert!(!has_widget, "covered values should be skipped");
    }

    #[test]
    fn test_score_cluster_multi_variant_boost() {
        let single_variant = TokenCluster {
            normalized: vec!["my".into(), "app".into()],
            literals: vec!["my-app".into()],
            total_occurrences: 10,
            file_count: 5,
            matches_dir_name: false,
            in_config_value: false,
        };

        let multi_variant = TokenCluster {
            normalized: vec!["my".into(), "app".into()],
            literals: vec!["my-app".into(), "my_app".into(), "MyApp".into()],
            total_occurrences: 10,
            file_count: 5,
            matches_dir_name: false,
            in_config_value: false,
        };

        assert!(score_cluster(&multi_variant) > score_cluster(&single_variant));
    }

    #[test]
    fn test_score_cluster_dir_name_boost() {
        let no_dir = TokenCluster {
            normalized: vec!["my".into(), "app".into()],
            literals: vec!["my-app".into()],
            total_occurrences: 5,
            file_count: 3,
            matches_dir_name: false,
            in_config_value: false,
        };

        let with_dir = TokenCluster {
            normalized: vec!["my".into(), "app".into()],
            literals: vec!["my-app".into()],
            total_occurrences: 5,
            file_count: 3,
            matches_dir_name: true,
            in_config_value: false,
        };

        assert!(score_cluster(&with_dir) > score_cluster(&no_dir));
    }

    #[test]
    fn test_levenshtein_merging() {
        let mut clusters = HashMap::new();
        clusters.insert(
            "data pipeline".to_string(),
            TokenCluster {
                normalized: vec!["data".into(), "pipeline".into()],
                literals: vec!["data-pipeline".into()],
                total_occurrences: 10,
                file_count: 5,
                matches_dir_name: false,
                in_config_value: false,
            },
        );
        clusters.insert(
            "data pipelin".to_string(), // typo / near miss
            TokenCluster {
                normalized: vec!["data".into(), "pipelin".into()],
                literals: vec!["data-pipelin".into()],
                total_occurrences: 2,
                file_count: 1,
                matches_dir_name: false,
                in_config_value: false,
            },
        );

        merge_similar_clusters(&mut clusters);

        // Should merge into one cluster
        assert_eq!(clusters.len(), 1);
        let remaining = clusters.values().next().unwrap();
        assert_eq!(remaining.total_occurrences, 12);
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
    fn test_suggest_variable_name() {
        assert_eq!(
            suggest_variable_name(&["my".into(), "app".into()], "my app"),
            "my_app"
        );
        assert_eq!(
            suggest_variable_name(
                &["very".into(), "long".into(), "name".into(), "here".into()],
                "very long name here"
            ),
            "very_long_name"
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

        let scan = crate::extract::scan::scan_project(&project_dir, &[]).unwrap();
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
