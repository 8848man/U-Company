use std::path::Path;

fn main() {
    let mut args = std::env::args().skip(1);
    let provider = args.next().unwrap_or_else(|| "offline".into());
    let path = args.next();
    if args.next().is_some() {
        eprintln!("Usage: cargo run --bin provider-harness -- [offline|claude|codex] [executable-path]");
        std::process::exit(2);
    }
    match ai_organization_os_lib::provider_harness::run(&provider, path.as_deref().map(Path::new)) {
        Ok(report) => println!("{report}"),
        Err(error) => { eprintln!("Provider: {provider}\nResult: FAIL\n{error}"); std::process::exit(1); }
    }
}
