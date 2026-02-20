use std::collections::BTreeMap;

use tera::{Context, Tera, Value};

pub fn build_context(variables: &BTreeMap<String, Value>) -> Context {
    let mut context = Context::new();
    for (key, value) in variables {
        context.insert(key, value);
    }
    context
}

/// Evaluate a Tera boolean expression against a variable context.
///
/// Returns `Ok(true)` if the expression evaluates to true, `Ok(false)` otherwise.
/// Returns `Err` if the expression fails to parse or render.
pub fn eval_bool_expr(expr: &str, context: &Context) -> std::result::Result<bool, tera::Error> {
    let mut tera = Tera::default();
    let template_str = format!("{{% if {expr} %}}true{{% else %}}false{{% endif %}}");
    tera.add_raw_template("__when__", &template_str)?;
    let result = tera.render("__when__", context)?;
    Ok(result.trim() == "true")
}
