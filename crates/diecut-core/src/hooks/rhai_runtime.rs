use std::collections::BTreeMap;
use std::path::Path;

use rhai::{Engine, Scope};
use tera::Value;

/// Create a sandboxed Rhai engine with diecut-specific functions.
pub fn create_engine() -> Engine {
    let mut engine = Engine::new();

    // Limit recursion and operations for safety
    engine.set_max_call_levels(32);
    engine.set_max_operations(100_000);
    engine.set_max_string_size(10 * 1024 * 1024); // 10MB

    engine
}

/// Build a Rhai scope from template variables.
pub fn build_scope<'a>(
    variables: &BTreeMap<String, Value>,
    output_dir: Option<&Path>,
) -> Scope<'a> {
    let mut scope = Scope::new();

    for (key, value) in variables {
        match value {
            Value::String(s) => {
                scope.push(key.clone(), s.clone());
            }
            Value::Bool(b) => {
                scope.push(key.clone(), *b);
            }
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    scope.push(key.clone(), i);
                } else if let Some(f) = n.as_f64() {
                    scope.push(key.clone(), f);
                }
            }
            _ => {
                scope.push(key.clone(), value.to_string());
            }
        }
    }

    if let Some(dir) = output_dir {
        scope.push("output_dir".to_string(), dir.to_string_lossy().to_string());
    }

    scope
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_engine() {
        let engine = create_engine();
        // Engine should be able to evaluate basic expressions
        let result: i64 = engine.eval("1 + 2").unwrap();
        assert_eq!(result, 3);
    }

    #[test]
    fn test_build_scope_with_variables() {
        let mut vars = BTreeMap::new();
        vars.insert("name".to_string(), Value::String("test".to_string()));
        vars.insert(
            "count".to_string(),
            Value::Number(serde_json::Number::from(42)),
        );
        vars.insert("flag".to_string(), Value::Bool(true));

        let scope = build_scope(&vars, None);
        assert!(scope.contains("name"));
        assert!(scope.contains("count"));
        assert!(scope.contains("flag"));
    }

    #[test]
    fn test_build_scope_with_output_dir() {
        let vars = BTreeMap::new();
        let scope = build_scope(&vars, Some(Path::new("/tmp/output")));
        assert!(scope.contains("output_dir"));
    }

    #[test]
    fn test_engine_with_scope() {
        let mut vars = BTreeMap::new();
        vars.insert(
            "project_name".to_string(),
            Value::String("my-project".to_string()),
        );

        let engine = create_engine();
        let mut scope = build_scope(&vars, None);

        let result: String = engine.eval_with_scope(&mut scope, "project_name").unwrap();
        assert_eq!(result, "my-project");
    }

    #[test]
    fn test_engine_max_operations() {
        let engine = create_engine();
        // This should fail due to max operations limit
        let result = engine.run("let x = 0; while true { x += 1; }");
        assert!(result.is_err());
    }
}
