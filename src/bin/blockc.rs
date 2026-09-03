use block_native::{bytecode, model::Project};
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
        return Err("usage: blockc <project.json> [-o output.bcode]".into());
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
    let project: Project = serde_json::from_str(&source)?;
    let bytes = bytecode::compile(&project)?;
    fs::write(&output, bytes)?;
    println!("compiled {} -> {}", input.display(), output.display());
    Ok(())
}
