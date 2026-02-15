use console::style;
use diecut::GenerateOptions;
use miette::Result;

#[allow(clippy::too_many_arguments)]
pub fn run(
    template: String,
    output: Option<String>,
    data: Vec<String>,
    defaults: bool,
    overwrite: bool,
    no_hooks: bool,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    let data_pairs: Vec<(String, String)> = data
        .into_iter()
        .filter_map(|kv| {
            let mut parts = kv.splitn(2, '=');
            let key = parts.next()?.to_string();
            let value = parts.next()?.to_string();
            Some((key, value))
        })
        .collect();

    let options = GenerateOptions {
        template,
        output,
        data: data_pairs,
        defaults,
        overwrite,
        no_hooks,
    };

    if dry_run {
        let plan = diecut::plan_generation(options)?;

        let rendered_count = plan.render_plan.files.iter().filter(|f| !f.is_copy).count();
        let copied_count = plan.render_plan.files.iter().filter(|f| f.is_copy).count();

        println!(
            "\n{} Dry run \u{2014} files that would be generated in {}:",
            style("==>").cyan().bold(),
            style(plan.output_dir.display()).cyan()
        );

        for file in &plan.render_plan.files {
            let action = if file.is_copy { "copy  " } else { "create" };
            println!(
                "  {} {}",
                style(action).green(),
                file.relative_path.display()
            );

            if verbose {
                println!("  {}", style("──────").dim());
                if file.is_copy {
                    println!(
                        "  {}",
                        style(format!("[binary file, {} bytes]", file.content.len())).dim()
                    );
                } else {
                    let content = String::from_utf8_lossy(&file.content);
                    for line in content.lines() {
                        println!("  {}", line);
                    }
                }
                println!("  {}", style("──────").dim());
                println!();
            }
        }

        println!(
            "\nSummary: {} rendered, {} copied",
            rendered_count, copied_count
        );

        println!(
            "\n{} Dry run \u{2014} no files written.",
            style("\u{2139}").blue().bold()
        );
    } else {
        diecut::generate(options)?;
    }

    Ok(())
}
