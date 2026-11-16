use std::io::Write;
use std::path::PathBuf;

const USAGE: &str = "Usage:\n  cargo run --bin token-bench -- run <claude|codex|mock> <before|after> [--exe PATH] [--out FILE.jsonl] [--work DIR]\n  cargo run --bin token-bench -- prompt <before|after>";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| args.iter().position(|item| item == name).and_then(|index| args.get(index + 1)).map(PathBuf::from);
    let result = match args.first().map(String::as_str) {
        Some("prompt") if args.len() == 2 => ai_organization_os_lib::token_bench::prompt(args[1] == "after").map(|text| { println!("{text}"); }),
        Some("run") if args.len() >= 3 => {
            let work = flag("--work").unwrap_or_else(|| std::env::temp_dir().join("utopia-token-bench"));
            match ai_organization_os_lib::token_bench::run(&args[1], &args[2], flag("--exe").as_deref(), &work) {
                Ok(line) => {
                    println!("{line}");
                    match flag("--out") {
                        Some(out) => std::fs::OpenOptions::new().create(true).append(true).open(&out).and_then(|mut file| writeln!(file, "{line}")).map_err(|e| format!("write {}: {e}", out.display())),
                        None => Ok(()),
                    }
                }
                Err(error) => Err(error),
            }
        }
        _ => Err(USAGE.to_string()),
    };
    if let Err(error) = result { eprintln!("{error}"); std::process::exit(1); }
}
