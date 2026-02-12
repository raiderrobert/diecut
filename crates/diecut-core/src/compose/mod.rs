pub mod overlay;

use std::path::Path;

use crate::config::load_config;
use crate::config::schema::TemplateConfig;
use crate::error::{DicecutError, Result};
use crate::template::{get_or_clone, resolve_source, TemplateSource};

use self::overlay::overlay_dir;

/// The result of composing a template with its extends/includes chain.
#[derive(Debug)]
pub struct ComposedTemplate {
    /// Composed template directory (temporary; dropped when this struct is dropped).
    pub dir: tempfile::TempDir,
    /// Merged configuration.
    pub config: TemplateConfig,
}

/// Compose a template by resolving its `extends` and `includes` chain.
///
/// If the config has no `extends` or `includes`, returns `None` (caller should
/// use the original template as-is).
pub fn compose_template(
    template_dir: &Path,
    config: &TemplateConfig,
) -> Result<Option<ComposedTemplate>> {
    let has_extends = config.template.extends.is_some();
    let has_includes = config
        .template
        .includes
        .as_ref()
        .is_some_and(|v| !v.is_empty());

    if !has_extends && !has_includes {
        return Ok(None);
    }

    let composed_dir = tempfile::tempdir().map_err(|e| DicecutError::Io {
        context: "creating temp directory for template composition".into(),
        source: e,
    })?;

    let mut merged_config = config.clone();

    // Handle extends chain
    if let Some(ref extends_source) = config.template.extends {
        let mut chain = vec![config.template.name.clone()];
        resolve_extends(
            extends_source,
            &mut merged_config,
            composed_dir.path(),
            &mut chain,
        )?;
    }

    // Overlay the child template on top of whatever the base produced
    let child_template_dir = template_dir.join("template");
    if child_template_dir.exists() {
        overlay_dir(&child_template_dir, &composed_dir.path().join("template"))?;
    }

    // Copy the child's diecut.toml so the adapter can find it
    let child_config_path = template_dir.join("diecut.toml");
    if child_config_path.exists() {
        std::fs::copy(&child_config_path, composed_dir.path().join("diecut.toml")).map_err(
            |e| DicecutError::Io {
                context: "copying child diecut.toml to composed dir".into(),
                source: e,
            },
        )?;
    }

    // Handle includes
    if let Some(ref includes) = config.template.includes {
        for include in includes {
            resolve_include(include, &mut merged_config, composed_dir.path())?;
        }
    }

    // Clear extends/includes from the merged config to avoid re-composition
    merged_config.template.extends = None;
    merged_config.template.includes = None;

    Ok(Some(ComposedTemplate {
        dir: composed_dir,
        config: merged_config,
    }))
}

/// Recursively resolve the `extends` chain, detecting circular dependencies.
fn resolve_extends(
    source_str: &str,
    merged_config: &mut TemplateConfig,
    composed_dir: &Path,
    chain: &mut Vec<String>,
) -> Result<()> {
    let base_dir = fetch_template(source_str, None)?;
    let base_config = load_config(&base_dir)?;

    // Check for circular extends
    if chain.contains(&base_config.template.name) {
        chain.push(base_config.template.name.clone());
        return Err(DicecutError::CircularExtends {
            chain: chain.clone(),
        });
    }
    chain.push(base_config.template.name.clone());

    // If the base also extends something, resolve that first (depth-first)
    if let Some(ref parent_source) = base_config.template.extends {
        resolve_extends(parent_source, merged_config, composed_dir, chain)?;
    } else {
        // We've reached the root of the chain -- start building from here
        let base_template_dir = base_dir.join("template");
        let composed_template_dir = composed_dir.join("template");
        std::fs::create_dir_all(&composed_template_dir).map_err(|e| DicecutError::Io {
            context: "creating composed template directory".into(),
            source: e,
        })?;
        overlay_dir(&base_template_dir, &composed_template_dir)?;
    }

    // If this is a mid-chain template (has extends and was resolved recursively),
    // overlay its files on top
    if base_config.template.extends.is_some() {
        let base_template_dir = base_dir.join("template");
        let composed_template_dir = composed_dir.join("template");
        overlay_dir(&base_template_dir, &composed_template_dir)?;
    }

    // Merge variables: base first, then child overrides
    merge_variables(&base_config, merged_config);

    Ok(())
}

/// Resolve an include: fetch it and copy its template/ contents into the composed dir.
fn resolve_include(
    include: &crate::config::schema::IncludeConfig,
    merged_config: &mut TemplateConfig,
    composed_dir: &Path,
) -> Result<()> {
    let include_dir = fetch_template(&include.source, include.git_ref.as_deref())?;
    let include_template_dir = include_dir.join("template");

    if !include_template_dir.exists() {
        return Err(DicecutError::CompositionError {
            message: format!(
                "included template '{}' has no template/ directory",
                include.source
            ),
        });
    }

    let dest = if let Some(ref prefix) = include.prefix {
        composed_dir.join("template").join(prefix)
    } else {
        composed_dir.join("template")
    };

    std::fs::create_dir_all(&dest).map_err(|e| DicecutError::Io {
        context: format!("creating include destination {}", dest.display()),
        source: e,
    })?;

    overlay_dir(&include_template_dir, &dest)?;

    // Merge variables from include (child config wins on conflict)
    let include_config = load_config(&include_dir)?;
    merge_variables(&include_config, merged_config);

    Ok(())
}

/// Merge variables from `source` into `target`. Target values take precedence.
fn merge_variables(source: &TemplateConfig, target: &mut TemplateConfig) {
    for (name, var) in &source.variables {
        target
            .variables
            .entry(name.clone())
            .or_insert_with(|| var.clone());
    }
}

/// Fetch a template source, returning the path to it on disk.
fn fetch_template(source_str: &str, git_ref: Option<&str>) -> Result<std::path::PathBuf> {
    let source = resolve_source(source_str)?;
    match source {
        TemplateSource::Local(path) => Ok(path),
        TemplateSource::Git { url, git_ref: sr } => {
            let effective_ref = git_ref.or(sr.as_deref());
            get_or_clone(&url, effective_ref)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// Helper: create a minimal template directory with a diecut.toml and template/ dir.
    fn create_template(
        dir: &Path,
        name: &str,
        extends: Option<&str>,
        files: &[(&str, &str)],
        variables: &[(&str, &str)],
    ) {
        std::fs::create_dir_all(dir.join("template")).unwrap();

        let mut toml_content =
            format!("[template]\nname = \"{name}\"\ntemplates_suffix = \".tera\"\n");
        if let Some(ext) = extends {
            toml_content.push_str(&format!("extends = \"{ext}\"\n"));
        }

        if !variables.is_empty() {
            toml_content.push('\n');
            for (var_name, default) in variables {
                toml_content.push_str(&format!(
                    "[variables.{var_name}]\ntype = \"string\"\ndefault = \"{default}\"\n"
                ));
            }
        }

        std::fs::write(dir.join("diecut.toml"), &toml_content).unwrap();

        for (path, content) in files {
            let file_path = dir.join("template").join(path);
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(file_path, content).unwrap();
        }
    }

    #[test]
    fn no_extends_no_includes_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        create_template(tmp.path(), "simple", None, &[("hello.txt", "hi")], &[]);

        let config = load_config(tmp.path()).unwrap();
        let result = compose_template(tmp.path(), &config).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn extends_child_overrides_parent_file() {
        let parent = tempfile::tempdir().unwrap();
        let child = tempfile::tempdir().unwrap();

        create_template(
            parent.path(),
            "parent",
            None,
            &[("readme.txt", "parent-readme"), ("base.txt", "base-only")],
            &[("author", "parent-default")],
        );

        let parent_path = parent.path().to_str().unwrap();
        create_template(
            child.path(),
            "child",
            Some(parent_path),
            &[("readme.txt", "child-readme")],
            &[("author", "child-default"), ("version", "1.0")],
        );

        let config = load_config(child.path()).unwrap();
        let composed = compose_template(child.path(), &config).unwrap().unwrap();

        // Child file overrides parent
        let readme =
            std::fs::read_to_string(composed.dir.path().join("template/readme.txt")).unwrap();
        assert_eq!(readme, "child-readme");

        // Parent-only file is inherited
        let base = std::fs::read_to_string(composed.dir.path().join("template/base.txt")).unwrap();
        assert_eq!(base, "base-only");

        // Child variable wins
        assert_eq!(
            composed.config.variables["author"]
                .default
                .as_ref()
                .unwrap(),
            &toml::Value::String("child-default".into())
        );
        // Child-only variable is present
        assert!(composed.config.variables.contains_key("version"));
    }

    #[test]
    fn includes_files_placed_under_prefix() {
        let include_tmpl = tempfile::tempdir().unwrap();
        let main_tmpl = tempfile::tempdir().unwrap();

        create_template(
            include_tmpl.path(),
            "partial",
            None,
            &[("helper.txt", "helper-content")],
            &[],
        );

        let include_path = include_tmpl.path().to_str().unwrap();
        let toml_content = format!(
            "[template]\nname = \"main\"\ntemplates_suffix = \".tera\"\n\n\
             [[template.includes]]\nsource = \"{include_path}\"\nprefix = \"lib\"\n"
        );
        std::fs::create_dir_all(main_tmpl.path().join("template")).unwrap();
        std::fs::write(main_tmpl.path().join("diecut.toml"), &toml_content).unwrap();
        std::fs::write(main_tmpl.path().join("template/main.txt"), "main-content").unwrap();

        let config = load_config(main_tmpl.path()).unwrap();
        let composed = compose_template(main_tmpl.path(), &config)
            .unwrap()
            .unwrap();

        // Main file present
        let main_file =
            std::fs::read_to_string(composed.dir.path().join("template/main.txt")).unwrap();
        assert_eq!(main_file, "main-content");

        // Included file under prefix
        let helper =
            std::fs::read_to_string(composed.dir.path().join("template/lib/helper.txt")).unwrap();
        assert_eq!(helper, "helper-content");
    }

    #[test]
    fn includes_no_prefix_goes_to_root() {
        let include_tmpl = tempfile::tempdir().unwrap();
        let main_tmpl = tempfile::tempdir().unwrap();

        create_template(
            include_tmpl.path(),
            "partial",
            None,
            &[("shared.txt", "shared")],
            &[],
        );

        let include_path = include_tmpl.path().to_str().unwrap();
        let toml_content = format!(
            "[template]\nname = \"main\"\ntemplates_suffix = \".tera\"\n\n\
             [[template.includes]]\nsource = \"{include_path}\"\n"
        );
        std::fs::create_dir_all(main_tmpl.path().join("template")).unwrap();
        std::fs::write(main_tmpl.path().join("diecut.toml"), &toml_content).unwrap();

        let config = load_config(main_tmpl.path()).unwrap();
        let composed = compose_template(main_tmpl.path(), &config)
            .unwrap()
            .unwrap();

        let shared =
            std::fs::read_to_string(composed.dir.path().join("template/shared.txt")).unwrap();
        assert_eq!(shared, "shared");
    }

    #[test]
    fn circular_extends_detected() {
        let tmpl_a = tempfile::tempdir().unwrap();
        let tmpl_b = tempfile::tempdir().unwrap();

        let path_a = tmpl_a.path().to_str().unwrap().to_string();
        let path_b = tmpl_b.path().to_str().unwrap().to_string();

        // A extends B, B extends A
        create_template(tmpl_a.path(), "alpha", Some(&path_b), &[], &[]);
        create_template(tmpl_b.path(), "beta", Some(&path_a), &[], &[]);

        let config = load_config(tmpl_a.path()).unwrap();
        let result = compose_template(tmpl_a.path(), &config);

        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            DicecutError::CircularExtends { chain } => {
                assert!(chain.contains(&"alpha".to_string()));
                assert!(chain.contains(&"beta".to_string()));
            }
            other => panic!("expected CircularExtends, got: {other}"),
        }
    }

    #[test]
    fn merge_variables_base_inherited() {
        let mut base = TemplateConfig {
            template: crate::config::schema::TemplateMetadata {
                name: "base".into(),
                version: None,
                description: None,
                min_diecut_version: None,
                templates_suffix: ".tera".into(),
                extends: None,
                includes: None,
            },
            variables: BTreeMap::new(),
            files: Default::default(),
            hooks: Default::default(),
            answers: Default::default(),
        };
        base.variables.insert(
            "author".into(),
            crate::config::variable::VariableConfig {
                var_type: crate::config::variable::VariableType::String,
                prompt: Some("Author?".into()),
                default: Some("base-author".into()),
                choices: None,
                validation: None,
                validation_message: None,
                when: None,
                computed: None,
                secret: false,
            },
        );

        let mut target = TemplateConfig {
            template: crate::config::schema::TemplateMetadata {
                name: "child".into(),
                version: None,
                description: None,
                min_diecut_version: None,
                templates_suffix: ".tera".into(),
                extends: None,
                includes: None,
            },
            variables: BTreeMap::new(),
            files: Default::default(),
            hooks: Default::default(),
            answers: Default::default(),
        };

        merge_variables(&base, &mut target);

        assert!(target.variables.contains_key("author"));
        assert_eq!(
            target.variables["author"].default.as_ref().unwrap(),
            &toml::Value::String("base-author".into())
        );
    }

    #[test]
    fn merge_variables_child_wins() {
        let mut base = TemplateConfig {
            template: crate::config::schema::TemplateMetadata {
                name: "base".into(),
                version: None,
                description: None,
                min_diecut_version: None,
                templates_suffix: ".tera".into(),
                extends: None,
                includes: None,
            },
            variables: BTreeMap::new(),
            files: Default::default(),
            hooks: Default::default(),
            answers: Default::default(),
        };
        base.variables.insert(
            "author".into(),
            crate::config::variable::VariableConfig {
                var_type: crate::config::variable::VariableType::String,
                prompt: Some("Author?".into()),
                default: Some("base-author".into()),
                choices: None,
                validation: None,
                validation_message: None,
                when: None,
                computed: None,
                secret: false,
            },
        );

        let mut target = TemplateConfig {
            template: crate::config::schema::TemplateMetadata {
                name: "child".into(),
                version: None,
                description: None,
                min_diecut_version: None,
                templates_suffix: ".tera".into(),
                extends: None,
                includes: None,
            },
            variables: BTreeMap::new(),
            files: Default::default(),
            hooks: Default::default(),
            answers: Default::default(),
        };
        target.variables.insert(
            "author".into(),
            crate::config::variable::VariableConfig {
                var_type: crate::config::variable::VariableType::String,
                prompt: Some("Author?".into()),
                default: Some("child-author".into()),
                choices: None,
                validation: None,
                validation_message: None,
                when: None,
                computed: None,
                secret: false,
            },
        );

        merge_variables(&base, &mut target);

        // Child value should remain
        assert_eq!(
            target.variables["author"].default.as_ref().unwrap(),
            &toml::Value::String("child-author".into())
        );
    }
}
