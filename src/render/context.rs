use std::collections::{BTreeMap, HashMap};

use tera::{Context, Tera, Value};

pub fn build_context(variables: &BTreeMap<String, Value>) -> Context {
    let mut context = Context::new();
    for (key, value) in variables {
        context.insert(key, value);
    }
    context
}

/// Create a Tera instance with custom filters registered.
///
/// This should be used instead of `Tera::default()` anywhere templates or
/// computed expressions are evaluated, so that custom filters like `camelcase`
/// are available.
pub fn tera_with_filters() -> Tera {
    let mut tera = Tera::default();
    tera.register_filter("camelcase", camelcase_filter);
    tera
}

/// Custom Tera filter: convert a separated string to camelCase.
///
/// Usage: `{{ value | camelcase }}` or `{{ value | camelcase(sep="-") }}`
///
/// Splits on the separator (default `-`), lowercases the first word,
/// title-cases the rest, and joins them.
fn camelcase_filter(value: &Value, args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    let s = value
        .as_str()
        .ok_or_else(|| tera::Error::msg("camelcase filter requires a string value"))?;

    let sep = args.get("sep").and_then(|v| v.as_str()).unwrap_or("-");

    let words: Vec<&str> = s.split(sep).collect();
    if words.is_empty() {
        return Ok(Value::String(String::new()));
    }

    let mut result = words[0].to_lowercase();
    for word in &words[1..] {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            result.extend(first.to_uppercase());
            result.push_str(&chars.as_str().to_lowercase());
        }
    }

    Ok(Value::String(result))
}

/// Evaluate a Tera boolean expression against a variable context.
///
/// Returns `Ok(true)` if the expression evaluates to true, `Ok(false)` otherwise.
/// Returns `Err` if the expression fails to parse or render.
pub fn eval_bool_expr(expr: &str, context: &Context) -> std::result::Result<bool, tera::Error> {
    let mut tera = tera_with_filters();
    let template_str = format!("{{% if {expr} %}}true{{% else %}}false{{% endif %}}");
    tera.add_raw_template("__when__", &template_str)?;
    let result = tera.render("__when__", context)?;
    Ok(result.trim() == "true")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camelcase_filter_kebab() {
        let val = Value::String("my-cool-app".to_string());
        let args = HashMap::new();
        let result = camelcase_filter(&val, &args).unwrap();
        assert_eq!(result, Value::String("myCoolApp".to_string()));
    }

    #[test]
    fn test_camelcase_filter_custom_sep() {
        let val = Value::String("my_cool_app".to_string());
        let mut args = HashMap::new();
        args.insert("sep".to_string(), Value::String("_".to_string()));
        let result = camelcase_filter(&val, &args).unwrap();
        assert_eq!(result, Value::String("myCoolApp".to_string()));
    }

    #[test]
    fn test_camelcase_filter_single_word() {
        let val = Value::String("hello".to_string());
        let args = HashMap::new();
        let result = camelcase_filter(&val, &args).unwrap();
        assert_eq!(result, Value::String("hello".to_string()));
    }
}
