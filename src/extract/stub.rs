use std::path::Path;

/// Whether a file is boilerplate (copy in full) or content (stub to placeholder).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileRole {
    /// Config, dotfiles, CI — copy verbatim into the template.
    Boilerplate,
    /// Prose, docs, source — stub to minimal placeholder.
    Content,
}

/// Filenames (case-insensitive) that are always boilerplate.
const BOILERPLATE_FILENAMES: &[&str] = &[
    ".gitignore",
    ".gitattributes",
    ".editorconfig",
    ".prettierrc",
    ".npmrc",
    ".nvmrc",
    ".gitkeep",
    "makefile",
    "dockerfile",
    "justfile",
    "license",
    "licence",
    "procfile",
];

/// Extensions (case-insensitive, without dot) that are always boilerplate.
const BOILERPLATE_EXTENSIONS: &[&str] = &[
    "toml", "yaml", "yml", "json", "jsonc", "json5", "xml", "sh", "bash", "zsh", "bat", "cmd",
    "ps1", "cfg", "ini", "conf",
];

/// Directory prefixes — files under these dirs are boilerplate.
const BOILERPLATE_DIR_PREFIXES: &[&str] = &[".github/", ".gitlab/", ".circleci/", ".vscode/"];

/// Classify a file as boilerplate or content based on its relative path.
///
/// Only called for text files with 0 template replacements.
pub fn classify_file(path: &Path) -> FileRole {
    let path_str = path.to_string_lossy();

    // Check directory prefix
    for prefix in BOILERPLATE_DIR_PREFIXES {
        if path_str.starts_with(prefix) {
            return FileRole::Boilerplate;
        }
    }

    // Check filename (case-insensitive)
    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
        let lower = filename.to_lowercase();
        if BOILERPLATE_FILENAMES.contains(&lower.as_str()) {
            return FileRole::Boilerplate;
        }
    }

    // Check extension (case-insensitive)
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let lower = ext.to_lowercase();
        if BOILERPLATE_EXTENSIONS.contains(&lower.as_str()) {
            return FileRole::Boilerplate;
        }
    }

    FileRole::Content
}

/// Generate a minimal stub for a content file.
///
/// - `.md` files get `# {Title}\n` where Title is derived from the filename.
/// - Everything else gets an empty string.
pub fn generate_stub(path: &Path) -> String {
    let is_md = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("md"));

    if is_md {
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled");
        // Title-case: capitalize first letter, leave rest as-is
        let title = title_case(title);
        format!("# {title}\n")
    } else {
        String::new()
    }
}

/// Convert a filename stem like "craft" or "SKILL" into title case.
///
/// Splits on `-` and `_`, capitalizes each word's first letter.
fn title_case(s: &str) -> String {
    s.split(['-', '_'])
        .filter(|w| !w.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    let rest: String = chars.collect::<String>().to_lowercase();
                    format!("{}{rest}", first.to_uppercase())
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // ── classify_file ────────────────────────────────────────────────

    #[rstest]
    #[case(".gitignore", FileRole::Boilerplate)]
    #[case(".editorconfig", FileRole::Boilerplate)]
    #[case("Makefile", FileRole::Boilerplate)]
    #[case("Dockerfile", FileRole::Boilerplate)]
    #[case("LICENSE", FileRole::Boilerplate)]
    #[case("Procfile", FileRole::Boilerplate)]
    fn classify_boilerplate_filenames(#[case] filename: &str, #[case] expected: FileRole) {
        assert_eq!(classify_file(Path::new(filename)), expected);
    }

    #[rstest]
    #[case("Cargo.toml", FileRole::Boilerplate)]
    #[case("config.yaml", FileRole::Boilerplate)]
    #[case("settings.yml", FileRole::Boilerplate)]
    #[case("package.json", FileRole::Boilerplate)]
    #[case("tsconfig.json", FileRole::Boilerplate)]
    #[case("setup.cfg", FileRole::Boilerplate)]
    #[case("build.sh", FileRole::Boilerplate)]
    #[case("deploy.ps1", FileRole::Boilerplate)]
    #[case("app.conf", FileRole::Boilerplate)]
    fn classify_boilerplate_extensions(#[case] filename: &str, #[case] expected: FileRole) {
        assert_eq!(classify_file(Path::new(filename)), expected);
    }

    #[rstest]
    #[case(".github/workflows/ci.yml", FileRole::Boilerplate)]
    #[case(".github/CODEOWNERS", FileRole::Boilerplate)]
    #[case(".gitlab/ci/deploy.yml", FileRole::Boilerplate)]
    #[case(".circleci/config.yml", FileRole::Boilerplate)]
    #[case(".vscode/settings.json", FileRole::Boilerplate)]
    fn classify_boilerplate_directories(#[case] path: &str, #[case] expected: FileRole) {
        assert_eq!(classify_file(Path::new(path)), expected);
    }

    #[rstest]
    #[case("README.md")]
    #[case("docs/guide.md")]
    #[case("src/main.rs")]
    #[case("src/lib.py")]
    #[case("index.html")]
    #[case("app.css")]
    #[case("skills/convention-mining/SKILL.md")]
    fn classify_content(#[case] path: &str) {
        assert_eq!(classify_file(Path::new(path)), FileRole::Content);
    }

    // ── generate_stub ────────────────────────────────────────────────

    #[rstest]
    #[case("README.md", "# Readme\n")]
    #[case("craft.md", "# Craft\n")]
    #[case("SKILL.md", "# Skill\n")]
    #[case("getting-started.md", "# Getting Started\n")]
    #[case("my_notes.md", "# My Notes\n")]
    fn stub_md_files(#[case] filename: &str, #[case] expected: &str) {
        assert_eq!(generate_stub(Path::new(filename)), expected);
    }

    #[rstest]
    #[case("src/main.rs")]
    #[case("index.html")]
    #[case("app.css")]
    #[case("data.txt")]
    fn stub_non_md_files(#[case] filename: &str) {
        assert_eq!(generate_stub(Path::new(filename)), "");
    }

    // ── title_case ───────────────────────────────────────────────────

    #[rstest]
    #[case("craft", "Craft")]
    #[case("SKILL", "Skill")]
    #[case("getting-started", "Getting Started")]
    #[case("my_notes", "My Notes")]
    #[case("README", "Readme")]
    fn test_title_case(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(title_case(input), expected);
    }
}
