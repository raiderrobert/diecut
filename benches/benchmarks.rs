use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::BTreeMap;
use std::path::PathBuf;
use tera::Value;

use diecut::adapter::resolve_template;
use diecut::render::{build_context, plan_render};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn sample_variables() -> BTreeMap<String, Value> {
    let mut vars = BTreeMap::new();
    vars.insert(
        "project_name".to_string(),
        Value::String("bench-project".to_string()),
    );
    vars.insert(
        "author".to_string(),
        Value::String("Benchmark User".to_string()),
    );
    vars.insert("use_docker".to_string(), Value::Bool(true));
    vars.insert("license".to_string(), Value::String("MIT".to_string()));
    vars.insert(
        "project_slug".to_string(),
        Value::String("bench-project".to_string()),
    );
    vars
}

fn bench_template_resolution(c: &mut Criterion) {
    let template_path = fixture_path("basic-template");

    c.bench_function("resolve_template", |b| {
        b.iter(|| {
            let resolved = resolve_template(black_box(&template_path)).unwrap();
            black_box(resolved)
        });
    });
}

fn bench_context_building(c: &mut Criterion) {
    let variables = sample_variables();

    c.bench_function("build_context", |b| {
        b.iter(|| {
            let context = build_context(black_box(&variables));
            black_box(context)
        });
    });
}

fn bench_render_planning(c: &mut Criterion) {
    let template_path = fixture_path("basic-template");
    let resolved = resolve_template(&template_path).unwrap();
    let variables = sample_variables();
    let context = build_context(&variables);

    c.bench_function("plan_render", |b| {
        b.iter(|| {
            let plan = plan_render(
                black_box(&resolved),
                black_box(&variables),
                black_box(&context),
            )
            .unwrap();
            black_box(plan)
        });
    });
}

fn bench_full_template_pipeline(c: &mut Criterion) {
    let template_path = fixture_path("basic-template");

    c.bench_function("full_pipeline (resolve + context + render)", |b| {
        b.iter(|| {
            let resolved = resolve_template(black_box(&template_path)).unwrap();
            let variables = sample_variables();
            let context = build_context(&variables);
            let plan = plan_render(&resolved, &variables, &context).unwrap();
            black_box(plan)
        });
    });
}

criterion_group!(
    benches,
    bench_template_resolution,
    bench_context_building,
    bench_render_planning,
    bench_full_template_pipeline
);
criterion_main!(benches);
