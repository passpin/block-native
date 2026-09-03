use block_native::{
    bytecode,
    model::Project,
    package::{build_package, PackedAsset},
    parser,
};
use std::{env, fs, path::PathBuf, process};

fn main() {
    if let Err(error) = run() {
        eprintln!("blockc: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let Some(input) = args.next() else {
        return Err("usage: blockc <project.json|project.bn> [-o output.bcode|output.bnp]".into());
    };
    let input = PathBuf::from(input);

    let mut output = input.with_extension("bcode");
    while let Some(arg) = args.next() {
        if arg == "-o" || arg == "--output" {
            let Some(path) = args.next() else {
                return Err("missing path after -o/--output".into());
            };
            output = PathBuf::from(path);
        } else {
            return Err(format!("unknown argument: {}", arg.to_string_lossy()).into());
        }
    }

    let source = fs::read_to_string(&input)?;
    let project = match input.extension().and_then(|value| value.to_str()) {
        Some(extension) if extension.eq_ignore_ascii_case("bn") => parser::parse_project(&source)?,
        Some(extension) if extension.eq_ignore_ascii_case("json") => Project::from_json_str(&source)?,
        _ => return Err("input extension must be .json or .bn".into()),
    };

    let program = bytecode::compile(&project)?;
    let is_package = output
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("bnp"));

    if is_package {
        let base = input.parent().unwrap_or_else(|| std::path::Path::new("."));
        let mut assets = Vec::with_capacity(project.assets.len());
        for asset in &project.assets {
            let declared = std::path::Path::new(&asset.path);
            let path = if declared.is_absolute() {
                declared.to_path_buf()
            } else {
                base.join(declared)
            };
            let bytes = fs::read(&path).map_err(|error| {
                format!(
                    "failed to read asset '{}' from {}: {error}",
                    asset.name,
                    path.display()
                )
            })?;
            assets.push(PackedAsset {
                name: asset.name.clone(),
                kind: asset.kind.clone(),
                bytes,
            });
        }
        fs::write(&output, build_package(&program, &assets)?)?;
    } else {
        fs::write(&output, program)?;
    }

    println!("compiled {} -> {}", input.display(), output.display());
    Ok(())
}
