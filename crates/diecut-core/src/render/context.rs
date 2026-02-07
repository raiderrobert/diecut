use std::collections::BTreeMap;

use tera::{Context, Value};

/// Build a Tera context from collected variable values.
pub fn build_context(variables: &BTreeMap<String, Value>) -> Context {
    let mut context = Context::new();
    for (key, value) in variables {
        context.insert(key, value);
    }
    context
}

/// Build a Tera context with an optional namespace.
/// Variables are always inserted flat (for computed expression evaluation).
/// If a namespace is provided, variables are also nested under that key
/// (e.g. `cookiecutter.project_name` for cookiecutter templates).
pub fn build_context_with_namespace(
    variables: &BTreeMap<String, Value>,
    namespace: &Option<String>,
) -> Context {
    let mut context = build_context(variables);

    if let Some(ns) = namespace {
        let nested: serde_json::Map<String, Value> = variables
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        context.insert(ns, &nested);
    }

    context
}
