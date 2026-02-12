#![allow(unused_assignments)]

use std::path::PathBuf;

use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum DicecutError {
    #[error("Template config not found at {path}")]
    #[diagnostic(help("Ensure the template directory contains a diecut.toml file"))]
    ConfigNotFound { path: PathBuf },

    #[error("Failed to parse diecut.toml")]
    #[diagnostic(help("Check the TOML syntax in your diecut.toml file"))]
    ConfigParse {
        #[source]
        source: toml::de::Error,
    },

    #[error("Invalid variable definition for '{name}': {reason}")]
    ConfigInvalidVariable { name: String, reason: String },

    #[error("Validation failed for variable '{name}': {message}")]
    ValidationFailed { name: String, message: String },

    #[error("Template rendering failed")]
    #[diagnostic(help("Check your Tera template syntax"))]
    RenderError {
        #[source]
        source: tera::Error,
    },

    #[error("Failed to render filename: {filename}")]
    FilenameRenderError {
        filename: String,
        #[source]
        source: tera::Error,
    },

    #[error("Output directory already exists: {path}")]
    #[diagnostic(help("Use --overwrite to replace the existing directory"))]
    OutputExists { path: PathBuf },

    #[error("Template directory not found: {path}")]
    #[diagnostic(help("The template must contain a 'template/' subdirectory"))]
    TemplateDirectoryMissing { path: PathBuf },

    #[error("IO error: {context}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Glob pattern error: {pattern}")]
    GlobPattern {
        pattern: String,
        #[source]
        source: globset::Error,
    },

    #[error("Prompt cancelled by user")]
    PromptCancelled,

    #[error("Invalid 'when' expression for variable '{name}'")]
    WhenEvaluation {
        name: String,
        #[source]
        source: tera::Error,
    },

    #[error("Invalid computed expression for variable '{name}'")]
    ComputedEvaluation {
        name: String,
        #[source]
        source: tera::Error,
    },

    #[error("Failed to parse cookiecutter.json")]
    #[diagnostic(help("Check the JSON syntax in your cookiecutter.json file"))]
    ConfigParseCookiecutter {
        #[source]
        source: serde_json::Error,
    },

    #[error("Cookiecutter template directory not found in {path}")]
    #[diagnostic(help(
        "Cookiecutter templates must contain a directory named {{{{cookiecutter.*}}}}"
    ))]
    CookiecutterTemplateDir { path: PathBuf },

    #[error("No supported template config found in {path}")]
    #[diagnostic(help(
        "The directory must contain diecut.toml (native) or cookiecutter.json (cookiecutter)"
    ))]
    UnsupportedFormat { path: PathBuf },

    #[error("Invalid template abbreviation: {input}")]
    #[diagnostic(help(
        "Supported abbreviations: gh:user/repo, gl:user/repo, bb:user/repo, sr:~user/repo"
    ))]
    InvalidAbbreviation { input: String },

    #[error("Hook '{hook}' failed: {message}")]
    #[diagnostic(help("Check the Rhai script for errors"))]
    HookError { hook: String, message: String },

    #[error("Cache metadata error: {context}")]
    #[diagnostic(help("Try clearing the cache with `diecut cache clear`"))]
    CacheMetadata { context: String },

    #[error("Unsafe URL scheme in '{url}': {reason}")]
    #[diagnostic(help("Use https:// URLs for remote templates"))]
    UnsafeUrl { url: String, reason: String },

    #[error("Git clone failed for {url}")]
    #[diagnostic(help("Check the URL and your network connection"))]
    GitClone { url: String, reason: String },

    #[error("Git checkout failed for ref '{git_ref}'")]
    #[diagnostic(help("Ensure the branch, tag, or commit exists in the repository"))]
    GitCheckout { git_ref: String, reason: String },

    #[error("Template composition error: {message}")]
    #[diagnostic(help("Check the extends/includes configuration in diecut.toml"))]
    CompositionError { message: String },

    #[error("Circular extends detected: {}", chain.join(" -> "))]
    #[diagnostic(help("Remove the circular dependency in your template extends chain"))]
    CircularExtends { chain: Vec<String> },
}

pub type Result<T> = std::result::Result<T, DicecutError>;
