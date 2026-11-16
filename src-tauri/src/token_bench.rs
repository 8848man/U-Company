//! Token measurement (WLG-091): one implementation task, the engine's real teammate request and the
//! real CLI call path, in a fresh copy of a fixed project. `focused` reproduces the request with or
//! without the 2026-10-21 focused-work guidance (`before` / `after`). Nothing here runs in the app.
//!
//! The run writes one JSON line: app-normalized usage (Claude input = input + cache creation + cache
//! read; Codex input already includes cache), wall time, the CLI session id (so
//! `scripts/token-bench/collect.mjs` can count internal responses and cache from the CLI's own log),
//! the hidden acceptance check result and whether only declared files changed.

use crate::model::{Agent, AiConnection, Task};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

fn repo_root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("..") }
fn bench_dir() -> PathBuf { repo_root().join("tests").join("token-bench") }

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(from).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let target = to.join(entry.file_name());
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() { copy_tree(&entry.path(), &target)?; } else { std::fs::copy(entry.path(), &target).map_err(|e| e.to_string())?; }
    }
    Ok(())
}

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git").args(args).current_dir(root).output().map_err(|e| format!("git: {e}"))?;
    if !out.status.success() { return Err(format!("git {args:?}: {}", String::from_utf8_lossy(&out.stderr))); }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn version(executable: &Path) -> String {
    Command::new(executable).arg("--version").output().ok().map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string()).unwrap_or_default()
}

fn connection(provider: &str, executable: Option<&Path>) -> Result<AiConnection, String> {
    if provider == "mock" {
        let node = crate::discovery::find_executable("node").ok_or("Node.js is needed for the mock run")?;
        return crate::mock_ai::connection(&node, &crate::mock_ai::script_path(), &[]);
    }
    if !["claude", "codex"].contains(&provider) { return Err("provider must be claude, codex or mock".into()); }
    let executable = match executable { Some(path) => path.to_path_buf(), None => crate::discovery::find_executable(provider).ok_or(format!("{provider} not found on PATH; pass --exe"))? };
    Ok(AiConnection { id: provider.into(), name: provider.into(), kind: "cli".into(), executable: executable.to_string_lossy().into_owned(), endpoint: String::new(), args: vec![], model_id: String::new(), status: "ready".into(), capabilities: vec![] })
}

/// The fixed task as the PM would hand it over, and the stock implementer it goes to.
fn task_and_agent() -> Result<(Task, Agent, String), String> {
    let spec: Value = serde_json::from_str(&std::fs::read_to_string(bench_dir().join("task.json")).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let task: Task = serde_json::from_value(json!({
        "id": "bench-task", "goalId": "bench-goal", "title": spec["title"], "instructions": spec["instructions"], "files": spec["files"],
        "assigneeId": "developer-a", "status": "running", "dependsOn": [], "result": "", "deliverable": spec["deliverable"],
        "acceptance": spec["acceptance"], "evidence": spec["evidence"], "capability": spec["capability"],
    })).map_err(|e| e.to_string())?;
    let preset = crate::presets::for_seed_id("developer-a");
    let agent = Agent { id: "developer-a".into(), name: "Developer A".into(), role: preset.role, instructions: preset.instructions, reports_to: Some("pm".into()), connection_id: String::new(), model_id: String::new(), x: 0, y: 0, enabled: true, external: None, appearance: None };
    Ok((task, agent, spec["intent"].as_str().unwrap_or_default().to_string()))
}

/// The teammate request for a condition, exactly as the engine builds it.
pub fn prompt(focused: bool) -> Result<String, String> {
    let (task, agent, intent) = task_and_agent()?;
    Ok(crate::engine::worker_prompt(&agent, &task, &intent, &task.title, &[], focused))
}

pub fn run(provider: &str, condition: &str, executable: Option<&Path>, work: &Path) -> Result<Value, String> {
    let focused = match condition { "after" => true, "before" => false, _ => return Err("condition must be before or after".into()) };
    let connection = connection(provider, executable)?;
    let (task, _, _) = task_and_agent()?;
    let prompt = prompt(focused)?;
    let project = work.join(format!("{provider}-{condition}-{}", crate::model::id()));
    copy_tree(&bench_dir().join("project"), &project)?;
    git(&project, &["init", "-q", "-b", "main"])?;
    git(&project, &["config", "core.autocrlf", "false"])?;
    git(&project, &["config", "user.email", "bench@example.invalid"])?;
    git(&project, &["config", "user.name", "bench"])?;
    git(&project, &["add", "."])?;
    git(&project, &["commit", "-q", "-m", "fixture"])?;
    // The plan card would approve `node --test` for this task (ADR-0058); both conditions get the same grant.
    let options = crate::provider::CallOptions { allowed: vec!["Bash(node --test)".into(), "Bash(node --test *)".into()], ..Default::default() };
    let started = Instant::now();
    let result = crate::provider::execute_with_options(&connection, &prompt, &project, true, &options, &mut |_| {}, &mut |_, _| {});
    let seconds = started.elapsed().as_secs_f64();
    let changed: Vec<String> = git(&project, &["status", "--porcelain", "-uall"])?.lines().map(|line| line[3..].trim().trim_matches('"').replace('\\', "/")).collect();
    let undeclared: Vec<&String> = changed.iter().filter(|path| !task.files.contains(path)).collect();
    let node = crate::discovery::find_executable("node").ok_or("Node.js is needed for the acceptance check")?;
    let check = Command::new(node).args(["--test", "--test-reporter=tap"]).arg(bench_dir().join("check.test.mjs")).env("BENCH_PROJECT", &project).output().map_err(|e| e.to_string())?;
    let tap = String::from_utf8_lossy(&check.stdout);
    let count = |key: &str| tap.lines().find_map(|line| line.strip_prefix(&format!("# {key} ")).and_then(|n| n.trim().parse::<u64>().ok())).unwrap_or(0);
    // The run folder and the model's reply stay local: results are meant to be committed (sdd/pr-workflow.md §4).
    eprintln!("run folder: {}", project.display());
    let (session_id, usage, error) = match &result {
        Ok(response) => (response.session_id.clone(), response.usage, None),
        Err(error) => (None, None, Some(crate::runtime::safe_diagnostic(error, 400))),
    };
    Ok(json!({
        "provider": provider, "condition": condition, "focused": focused,
        "cliVersion": version(Path::new(&connection.executable)),
        "startedAt": crate::model::now(), "seconds": (seconds * 10.0).round() / 10.0,
        "sessionId": session_id,
        "appInputTokens": usage.map(|u| u.0), "appOutputTokens": usage.map(|u| u.1),
        "promptChars": prompt.chars().count(),
        "check": { "pass": count("pass"), "fail": count("fail") },
        "changedFiles": changed, "undeclaredChanges": undeclared,
        "error": error,
    }))
}
