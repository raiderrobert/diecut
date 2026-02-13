use diecut::GenerateOptions;
use miette::Result;

pub fn run(
    template: String,
    output: Option<String>,
    data: Vec<String>,
    defaults: bool,
    overwrite: bool,
    no_hooks: bool,
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

    diecut::generate(options)?;

    Ok(())
}
