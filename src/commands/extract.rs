use std::path::PathBuf;

use console::style;

use diecut::error::DicecutError;
use diecut::extract::{execute_extraction, plan_extraction, ExtractOptions};
use miette::Result;

#[allow(clippy::too_many_arguments)]
pub fn run(
    source: String,
    vars: Vec<String>,
    output: Option<String>,
    in_place: bool,
    yes: bool,
    min_confidence: f64,
    stub_depth: usize,
    dry_run: bool,
) -> Result<()> {
    let variables = parse_vars(&vars)?;

    let options = ExtractOptions {
        source_dir: PathBuf::from(&source),
        variables,
        output_dir: output.map(PathBuf::from),
        in_place,
        yes,
        min_confidence,
        stub_depth,
        dry_run,
    };

    let plan = plan_extraction(&options)?;

    if dry_run {
        print_dry_run(&plan);
        return Ok(());
    }

    execute_extraction(&plan, in_place)?;

    Ok(())
}

fn parse_vars(vars: &[String]) -> diecut::error::Result<Vec<(String, String)>> {
    let mut parsed = Vec::new();

    for var in vars {
        let (key, value) = var
            .split_once('=')
            .ok_or_else(|| DicecutError::ExtractInvalidVar { input: var.clone() })?;
        parsed.push((key.trim().to_string(), value.trim().to_string()));
    }

    Ok(parsed)
}

fn print_dry_run(plan: &diecut::extract::ExtractionPlan) {
    eprintln!(
        "\n{} Dry run — no files will be written\n",
        style("⚡").yellow().bold()
    );

    eprintln!(
        "Output directory: {}",
        style(plan.output_dir.display()).cyan()
    );

    let templated: Vec<_> = plan.files.iter().filter(|f| f.has_replacements()).collect();
    let boilerplate: Vec<_> = plan
        .files
        .iter()
        .filter(|f| !f.has_replacements() && !f.stubbed)
        .collect();
    let stubbed: Vec<_> = plan.files.iter().filter(|f| f.stubbed).collect();

    eprintln!("\nTemplated files ({}):", templated.len());
    for file in &templated {
        eprintln!(
            "  {} ({} replacements)",
            file.template_path.display(),
            file.replacement_count()
        );
    }

    eprintln!("\nBoilerplate ({}):", boilerplate.len());
    for file in &boilerplate {
        eprintln!("  {}", file.template_path.display());
    }

    if !stubbed.is_empty() {
        eprintln!("\nStubbed ({}):", stubbed.len());
        for file in &stubbed {
            eprintln!("  {}", file.template_path.display());
        }
    }

    if plan.dropped_count > 0 {
        eprintln!("\nDropped ({}):", plan.dropped_count);
        for path in &plan.dropped_paths {
            eprintln!("  {}", path.display());
        }
    }

    eprintln!("\nVariables:");
    for var in &plan.variables {
        eprintln!("  {} = {:?}", var.name, var.value);
        for variant in &var.variants {
            if variant.name != "verbatim" {
                eprintln!("    {} → {}", variant.name, variant.literal);
            }
        }
    }

    eprintln!("\nGenerated diecut.toml:");
    eprintln!("{}", style("─".repeat(60)).dim());
    eprint!("{}", plan.config_toml);
    eprintln!("{}", style("─".repeat(60)).dim());
}
