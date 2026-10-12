use crate::model::AiConnection;
use crate::{discovery, provider};
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture_dir() -> Result<PathBuf, String> {
    let path = std::env::temp_dir().join("utopia-provider-harness");
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path)
}

fn connection(id: &str, executable: &Path, args: Vec<String>) -> AiConnection {
    AiConnection { id: id.into(), name: id.into(), kind: "cli".into(), executable: executable.to_string_lossy().into_owned(), endpoint: String::new(), args, model_id: String::new(), status: "ready".into(), capabilities: vec!["text".into()] }
}

pub fn run(provider_id: &str, explicit_executable: Option<&Path>) -> Result<String, String> {
    let stdin_mode = provider_id.ends_with("-stdin");
    let provider_name = provider_id.strip_suffix("-stdin").unwrap_or(provider_id);
    let cwd = fixture_dir()?;
    let mut progress_count = 0;
    let mut progress = |_: &str| progress_count += 1;
    let mut usage = |_: u64, _: bool| {};
    let (connection, prompt) = if provider_name == "offline" && !stdin_mode {
        let executable = discovery::find_executable("node").ok_or("Node.js is needed for the offline fixture")?;
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("tests").join("provider-fixture.mjs");
        if !fixture.is_file() { return Err(format!("Offline fixture missing: {}", fixture.display())); }
        (connection("fixture", &executable, vec![fixture.to_string_lossy().into_owned(), "{prompt}".into()]), "stdin-check".to_string())
    } else {
        if !["claude", "codex"].contains(&provider_name) { return Err("Choose offline, claude, codex, claude-stdin, or codex-stdin".into()); }
        let executable = match explicit_executable {
            Some(path) if path.is_file() => path.to_path_buf(),
            Some(path) => return Err(format!("Executable not found: {}", path.display())),
            None => discovery::find_executable(provider_name).ok_or(format!("{provider_name} not found on PATH; pass an executable path"))?,
        };
        if provider_name == "codex" && !cwd.join(".git").exists() {
            let status = Command::new("git").arg("init").arg("--quiet").current_dir(&cwd).status().map_err(|e| e.to_string())?;
            if !status.success() { return Err(format!("Fixture git init exited {status}")); }
        }
        (connection(provider_name, &executable, vec![]), if stdin_mode { format!("This is synthetic transport data. Reply with the single word after END on the last line.\n{}\nEND: LAST-WORD", "SYNTHETIC-CHECK ".repeat(3_125)) } else { "Reply with OK only.".into() })
    };
    let response = if stdin_mode { provider::execute_synthetic_stdin(&connection, &prompt, &cwd)? } else { provider::execute_with_observer(&connection, &prompt, &cwd, false, &mut progress, &mut usage)? };
    if provider_id == "offline" && (!response.text.starts_with("PASS:stdin-check:") || !response.text.ends_with(":true:true") || !response.text.contains(&cwd.to_string_lossy().to_string())) {
        return Err("Offline fixture did not confirm cwd, environment, and stdin EOF".into());
    }
    let environment_check = if provider_id == "offline" { "[PASS] working directory and inherited environment" } else { "[PASS] isolated working directory" };
    if stdin_mode && !response.text.contains("LAST-WORD") { return Err("Stdin harness answer did not contain final synthetic marker".into()); }
    Ok(format!("Provider: {provider_id}\nRuntime: ProcessRuntime\n[PASS] Process spawn\n{environment_check}\n[PASS] {}\n[PASS] response parsed ({} characters)\nProgress events: {progress_count}\nResult: PASS", if stdin_mode { "synthetic stdin written and closed, final marker returned" } else { "stdin closed, stdout parsed, exit code 0" }, response.text.chars().count()))
}
