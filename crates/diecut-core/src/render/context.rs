use std::collections::BTreeMap;

use tera::{Context, Value};

pub fn build_context(variables: &BTreeMap<String, Value>) -> Context {
    let mut context = Context::new();
    for (key, value) in variables {
        context.insert(key, value);
    }
    context
}

/// Variables are always inserted flat. If a namespace is provided, they're also
/// nested under that key (e.g. `cookiecutter.project_name`).
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
