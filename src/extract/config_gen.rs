/// A prompted variable entry for the generated config.
pub struct PromptedVariable {
    pub name: String,
    pub default_value: String,
    pub prompt: String,
}

/// A computed variable entry for the generated config.
pub struct ComputedVariable {
    pub name: String,
    pub expression: String,
}

/// A conditional file entry for the generated config.
#[derive(Debug, Clone)]
pub struct ConditionalEntry {
    pub patterns: Vec<String>,
    pub variable: String,
    pub description: String,
}

/// Options for generating the diecut.toml config file.
pub struct ConfigGenOptions {
    pub template_name: String,
    pub prompted_variables: Vec<PromptedVariable>,
    pub computed_variables: Vec<ComputedVariable>,
    pub exclude_patterns: Vec<String>,
    pub copy_without_render: Vec<String>,
    pub conditional_entries: Vec<ConditionalEntry>,
}

/// Generate a diecut.toml config string with comments for readability.
///
/// Uses manual TOML string building because the `toml` crate can't serialize comments,
/// and users need to read and edit this file.
pub fn generate_config_toml(options: &ConfigGenOptions) -> String {
    let mut out = String::new();

    // [template] section
    out.push_str("[template]\n");
    out.push_str(&format!(
        "name = {}\n",
        escape_toml_string(&options.template_name)
    ));
    out.push_str("version = \"1.0.0\"\n");
    out.push_str("# description = \"A project template\"\n");
    out.push('\n');

    // [variables] section — prompted variables first
    if !options.prompted_variables.is_empty() || !options.computed_variables.is_empty() {
        out.push_str("# ── Variables ──────────────────────────────────────────\n");
        out.push_str("# Prompted variables are asked during `diecut new`.\n");
        out.push_str("# Computed variables are auto-derived and never prompted.\n");
        out.push('\n');
    }

    for var in &options.prompted_variables {
        out.push_str(&format!("[variables.{}]\n", var.name));
        out.push_str("type = \"string\"\n");
        out.push_str(&format!("prompt = {}\n", escape_toml_string(&var.prompt)));
        out.push_str(&format!(
            "default = {}\n",
            escape_toml_string(&var.default_value)
        ));
        out.push('\n');
    }

    // Conditional file boolean variables
    for entry in &options.conditional_entries {
        out.push_str(&format!("# {} ({})\n", entry.variable, entry.description));
        out.push_str(&format!("[variables.{}]\n", entry.variable));
        out.push_str("type = \"bool\"\n");
        out.push_str(&format!(
            "prompt = {}\n",
            escape_toml_string(&format!("Include {}?", entry.description.to_lowercase()))
        ));
        out.push_str("default = true\n");
        out.push('\n');
    }

    // Computed variables
    for var in &options.computed_variables {
        out.push_str(&format!("[variables.{}]\n", var.name));
        out.push_str("type = \"string\"\n");
        out.push_str(&format!(
            "computed = {}\n",
            escape_toml_string(&var.expression)
        ));
        out.push('\n');
    }

    // [files] section
    out.push_str("# ── Files ─────────────────────────────────────────────\n");
    out.push_str("[files]\n");

    if !options.exclude_patterns.is_empty() {
        out.push_str("exclude = [\n");
        for pattern in &options.exclude_patterns {
            out.push_str(&format!("    {},\n", escape_toml_string(pattern)));
        }
        out.push_str("]\n");
    }

    if !options.copy_without_render.is_empty() {
        out.push_str("copy_without_render = [\n");
        for pattern in &options.copy_without_render {
            out.push_str(&format!("    {},\n", escape_toml_string(pattern)));
        }
        out.push_str("]\n");
    }

    out.push('\n');

    // [[files.conditional]] entries
    for entry in &options.conditional_entries {
        for pattern in &entry.patterns {
            out.push_str(&format!("# {}\n", entry.description));
            out.push_str("[[files.conditional]]\n");
            out.push_str(&format!("pattern = {}\n", escape_toml_string(pattern)));
            out.push_str(&format!("when = {}\n", escape_toml_string(&entry.variable)));
            out.push('\n');
        }
    }

    // [hooks] section
    out.push_str("# ── Hooks ─────────────────────────────────────────────\n");
    out.push_str("# [hooks]\n");
    out.push_str("# post_create = \"echo 'Project created!'\"\n");

    out
}

/// Escape a string for TOML output.
fn escape_toml_string(s: &str) -> String {
    toml::Value::String(s.to_string()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_config_basic() {
        let options = ConfigGenOptions {
            template_name: "my-template".to_string(),
            prompted_variables: vec![PromptedVariable {
                name: "project_name".to_string(),
                default_value: "my-app".to_string(),
                prompt: "Project name".to_string(),
            }],
            computed_variables: vec![ComputedVariable {
                name: "project_name_snake".to_string(),
                expression: "project_name | replace(from=\"-\", to=\"_\")".to_string(),
            }],
            exclude_patterns: vec![".git/".to_string()],
            copy_without_render: vec!["*.png".to_string()],
            conditional_entries: vec![],
        };

        let toml = generate_config_toml(&options);

        assert!(toml.contains("[template]"));
        assert!(toml.contains("name = \"my-template\""));
        assert!(toml.contains("[variables.project_name]"));
        assert!(toml.contains("type = \"string\""));
        assert!(toml.contains("[variables.project_name_snake]"));
        assert!(toml.contains("computed ="));
        assert!(toml.contains("[files]"));
        assert!(toml.contains("\".git/\""));
        assert!(toml.contains("\"*.png\""));
    }

    #[test]
    fn test_generate_config_with_conditionals() {
        let options = ConfigGenOptions {
            template_name: "test".to_string(),
            prompted_variables: vec![],
            computed_variables: vec![],
            exclude_patterns: vec![],
            copy_without_render: vec![],
            conditional_entries: vec![ConditionalEntry {
                patterns: vec![".github/**".to_string()],
                variable: "use_github_actions".to_string(),
                description: "GitHub Actions CI".to_string(),
            }],
        };

        let toml = generate_config_toml(&options);

        assert!(toml.contains("[variables.use_github_actions]"));
        assert!(toml.contains("type = \"bool\""));
        assert!(toml.contains("default = true"));
        assert!(toml.contains("[[files.conditional]]"));
        assert!(toml.contains("pattern = \".github/**\""));
        assert!(toml.contains("when = \"use_github_actions\""));
    }

    #[test]
    fn test_escape_toml_string() {
        assert_eq!(escape_toml_string("hello"), "\"hello\"");
        // toml crate uses multi-line strings for values containing quotes
        let escaped = escape_toml_string("it's \"fine\"");
        assert!(escaped.contains("it's"));
        assert!(escaped.contains("fine"));
    }
}
