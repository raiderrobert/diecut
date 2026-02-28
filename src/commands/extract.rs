use std::path::PathBuf;

use console::style;

use diecut::error::DicecutError;
use diecut::extract::{execute_extraction, plan_extraction, ExtractOptions};
use miette::Result;

pub fn run(
    source: String,
    vars: Vec<String>,
    output: Option<String>,
    in_place: bool,
    batch: bool,
    dry_run: bool,
    auto: bool,
) -> Result<()> {
    let variables = parse_vars(&vars)?;

    // Default auto to true when no vars are provided
    let auto = auto || variables.is_empty();

    let options = ExtractOptions {
        source_dir: PathBuf::from(&source),
        variables,
        output_dir: output.map(PathBuf::from),
        in_place,
        batch,
        dry_run,
        auto,
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
            .ok_or_else(|| DicecutError::ExtractNoVariables)?;
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

    let templated: Vec<_> = plan.files.iter().filter(|f| f.has_replacements).collect();
    let copied: Vec<_> = plan.files.iter().filter(|f| !f.has_replacements).collect();

    eprintln!("\nTemplated files ({}):", templated.len());
    for file in &templated {
        eprintln!(
            "  {} ({} replacements)",
            file.template_path.display(),
            file.replacement_count
        );
    }

    eprintln!("\nCopied verbatim ({}):", copied.len());
    for file in &copied {
        eprintln!("  {}", file.template_path.display());
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
