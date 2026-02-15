use std::collections::BTreeMap;

use tera::{Context, Value};

pub fn build_context(variables: &BTreeMap<String, Value>) -> Context {
    let mut context = Context::new();
    for (key, value) in variables {
        context.insert(key, value);
    }
    context
}
