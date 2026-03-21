use crate::extract::variants::CaseVariant;

use super::intersect::AlignedFile;

/// A distill variable candidate: its name, the value from project[0], and its case variants.
pub struct DistillVariable {
    pub name: String,
    pub value_in_p0: String,
    pub variants: Vec<CaseVariant>,
}

/// Check whether a variable actually varies across projects.
///
/// Returns `true` if, for at least one non-binary aligned file:
/// - Any variant literal appears in project[0]'s content, AND
/// - At least one other project has different content for that file.
///
/// Returns `false` if no file qualifies (variable is suppressed).
pub fn is_variable_active(var: &DistillVariable, aligned_files: &[AlignedFile]) -> bool {
    for file in aligned_files {
        // Skip binary files
        if file.any_binary {
            continue;
        }

        // Get project[0]'s content; skip if absent
        let Some(Some(p0_content)) = file.contents.first() else {
            continue;
        };

        // Check if any variant literal appears in project[0]'s content
        let literal_in_p0 = var.variants.iter().any(|v| p0_content.contains(&v.literal));

        if !literal_in_p0 {
            continue;
        }

        // Check if at least one other project has different content for this file
        let any_differs = file
            .contents
            .iter()
            .skip(1)
            .any(|c| c.as_deref() != Some(p0_content.as_str()));

        if any_differs {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use rstest::rstest;

    use super::*;
    use crate::extract::variants::CaseVariant;

    fn make_variant(literal: &str) -> CaseVariant {
        CaseVariant {
            name: "verbatim",
            literal: literal.to_string(),
            tera_expr: format!("{{{{ var }}}}"),
        }
    }

    fn make_var(value: &str) -> DistillVariable {
        DistillVariable {
            name: "project_name".to_string(),
            value_in_p0: value.to_string(),
            variants: vec![make_variant(value)],
        }
    }

    fn make_file(contents: Vec<Option<&str>>, any_binary: bool) -> AlignedFile {
        AlignedFile {
            relative_path: PathBuf::from("README.md"),
            contents: contents
                .into_iter()
                .map(|c| c.map(|s| s.to_string()))
                .collect(),
            raw_bytes: vec![],
            any_binary,
        }
    }

    #[test]
    fn active_when_content_differs_across_projects() {
        let var = make_var("my-app");
        let file = make_file(
            vec![
                Some("# my-app\nA project."),
                Some("# other-proj\nA project."),
            ],
            false,
        );
        assert!(is_variable_active(&var, &[file]));
    }

    #[test]
    fn suppressed_when_all_content_identical() {
        let var = make_var("my-app");
        let file = make_file(
            vec![Some("# my-app\nA project."), Some("# my-app\nA project.")],
            false,
        );
        assert!(!is_variable_active(&var, &[file]));
    }

    #[test]
    fn suppressed_when_literal_not_in_p0() {
        let var = make_var("my-app");
        // p0 does not contain "my-app"
        let file = make_file(
            vec![
                Some("# something-else\nNo match here."),
                Some("# other-proj\nDifferent content."),
            ],
            false,
        );
        assert!(!is_variable_active(&var, &[file]));
    }

    #[rstest]
    #[case(
        vec![Some("# my-app"), Some("# my-app"), Some("# other-proj")],
        true,
        "active with 3 projects where only one differs"
    )]
    #[case(
        vec![Some("# my-app"), Some("# my-app"), Some("# my-app")],
        false,
        "suppressed when all 3 identical"
    )]
    fn three_project_cases(
        #[case] contents: Vec<Option<&str>>,
        #[case] expected: bool,
        #[case] _label: &str,
    ) {
        let var = make_var("my-app");
        let file = make_file(contents, false);
        assert_eq!(is_variable_active(&var, &[file]), expected);
    }

    #[test]
    fn binary_files_are_skipped() {
        let var = make_var("my-app");
        // Mark the only file as binary — should not count even if content differs
        let file = make_file(
            vec![
                Some("# my-app\nA project."),
                Some("# other-proj\nA project."),
            ],
            true,
        );
        assert!(!is_variable_active(&var, &[file]));
    }

    #[test]
    fn active_when_one_file_qualifies_among_many() {
        let var = make_var("my-app");
        let binary_file = make_file(
            vec![Some("# my-app content"), Some("# other-proj content")],
            true, // binary, skipped
        );
        let no_literal_file = make_file(vec![Some("no match here"), Some("also no match")], false);
        let qualifying_file = make_file(
            vec![Some("project: my-app"), Some("project: other-proj")],
            false,
        );
        assert!(is_variable_active(
            &var,
            &[binary_file, no_literal_file, qualifying_file]
        ));
    }
}
