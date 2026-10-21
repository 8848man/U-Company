use crate::model::AiConnection;
use serde_json::{json, Value};
use std::io::{BufRead, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub use crate::provider::AiResponse as RuntimeResult;

#[cfg(test)]
pub fn execute(connection: &AiConnection, prompt: &str, cwd: &Path, writable: bool) -> Result<RuntimeResult, String> {
    execute_with_progress(connection, prompt, cwd, writable, &mut |_| {})
}

#[cfg(test)]
pub fn execute_with_progress(connection: &AiConnection, prompt: &str, cwd: &Path, writable: bool, progress: &mut dyn FnMut(&str)) -> Result<RuntimeResult, String> {
    execute_with_observer(connection, prompt, cwd, writable, progress, &mut |_, _| {})
}

#[cfg(test)]
pub fn execute_with_observer(connection: &AiConnection, prompt: &str, cwd: &Path, writable: bool, progress: &mut dyn FnMut(&str), usage_progress: &mut dyn FnMut(u64, bool)) -> Result<RuntimeResult, String> {
    crate::provider::execute_with_observer(connection, prompt, cwd, writable, progress, usage_progress)
}

pub(crate) fn token_usage(value: &Value) -> Option<(u64, u64)> {
    let usage = value.get("usage").or_else(|| value.pointer("/result/usage"))?;
    let input = usage.get("input_tokens").and_then(Value::as_u64)?;
    let output = usage.get("output_tokens").and_then(Value::as_u64)?;
    let cached = usage.get("cache_creation_input_tokens").and_then(Value::as_u64).unwrap_or(0)
        + usage.get("cache_read_input_tokens").and_then(Value::as_u64).unwrap_or(0);
    Some((input + cached, output))
}

pub(crate) fn capture_with_timeout(mut command: Command, timeout: Duration, progress: &mut dyn FnMut(&str), usage_progress: &mut dyn FnMut(u64, bool), provider: &str, input: Option<&[u8]>) -> Result<(String, String), String> {
    let mut child = command.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::piped()).spawn().map_err(|e| format!("process spawn: {e}"))?;
    // Drain output in parallel with a potentially large stdin write. Dropping the
    // handle after write_all guarantees EOF to one-shot clients.
    let writer = if let Some(bytes) = input {
        let mut stdin = child.stdin.take().ok_or("No stdin")?;
        let owned = bytes.to_vec();
        Some(thread::spawn(move || stdin.write_all(&owned).map_err(|e| e.to_string())))
    } else { drop(child.stdin.take()); None };
    let out = child.stdout.take().ok_or("No stdout")?;
    let mut err = child.stderr.take().ok_or("No stderr")?;
    let (line_tx, line_rx) = mpsc::channel();
    let out_handle = thread::spawn(move || {
        let mut reader = std::io::BufReader::new(out);
        let mut bytes = Vec::new();
        loop {
            let mut line = Vec::new();
            match reader.read_until(b'\n', &mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    bytes.extend_from_slice(&line);
                    let _ = line_tx.send(String::from_utf8_lossy(&line).into_owned());
                }
            }
        }
        bytes
    });
    let err_handle = thread::spawn(move || { let mut bytes = Vec::new(); let _ = err.read_to_end(&mut bytes); bytes });
    let deadline = Instant::now() + timeout;
    let mut emitted = 0;
    let mut last_summary = String::new();
    let mut observed_output = 0_u64;
    let mut drain_progress = |line_rx: &mpsc::Receiver<String>| {
        while let Ok(line) = line_rx.try_recv() {
            if let Ok(value) = serde_json::from_str::<Value>(&line) {
                if let Some((_, output)) = token_usage(&value) { usage_progress(output, true); }
                else if let Some(text) = (value.pointer("/item/type").and_then(Value::as_str) == Some("agent_message")).then(|| value.pointer("/item/text").and_then(Value::as_str)).flatten()
                    .or_else(|| (value.get("type").and_then(Value::as_str) == Some("assistant")).then(|| value.pointer("/message/content/0/text").and_then(Value::as_str)).flatten()) {
                    observed_output = observed_output.max(text.chars().count().div_ceil(4) as u64);
                    usage_progress(observed_output, false);
                }
            }
            if emitted >= 80 { continue; }
            if let Some(summary) = summarize_runtime_event(&line) {
                if summary != last_summary {
                    progress(&summary);
                    last_summary = summary;
                    emitted += 1;
                }
            }
        }
    };
    let status = loop {
        drain_progress(&line_rx);
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(100)),
            Ok(None) => { let _ = child.kill(); let _ = child.wait(); return Err(format!("process timeout: {provider} did not exit within {} seconds", timeout.as_secs())); }
            Err(e) => return Err(format!("process wait: {e}")),
        }
    };
    let stdout = String::from_utf8_lossy(&out_handle.join().map_err(|_| "stdout reader failed")?).into_owned();
    let stderr = String::from_utf8_lossy(&err_handle.join().map_err(|_| "stderr reader failed")?).into_owned();
    if let Some(writer) = writer { writer.join().map_err(|_| "stdin writer failed")?.map_err(|e| format!("stdin write: {e}"))?; }
    drain_progress(&line_rx);
    if !status.success() {
        let structured = structured_error_hint(&stdout);
        let detail = safe_diagnostic(&stderr, 500);
        return Err(format!("process exit: code={} provider={provider}; stderr={detail}; stdout_error={structured}", status.code().map_or_else(|| "terminated".into(), |code| code.to_string())));
    }
    Ok((stdout, stderr))
}

fn structured_error_hint(stdout: &str) -> String {
    stdout.lines().filter_map(|line| serde_json::from_str::<Value>(line).ok()).filter_map(|value| {
        let kind = value.get("type").and_then(Value::as_str)?;
        if !["error", "turn.failed"].contains(&kind) { return None; }
        let message = value.get("message").and_then(Value::as_str)
            .or_else(|| value.pointer("/error/message").and_then(Value::as_str)).unwrap_or("");
        Some(format!("{kind}: {}", safe_diagnostic(message, 250)))
    }).last().unwrap_or_else(|| "none".into())
}

pub(crate) fn safe_diagnostic(value: &str, max: usize) -> String {
    if ["sk-", "Bearer ", "api_key", "apiKey", "Authorization:"].iter().any(|needle| value.contains(needle)) {
        return "[redacted sensitive diagnostic]".into();
    }
    let flattened = value.replace(['\r', '\n'], " ");
    flattened.chars().rev().take(max).collect::<Vec<_>>().into_iter().rev().collect()
}

fn summarize_runtime_event(line: &str) -> Option<String> {
    let value: Value = serde_json::from_str(line).ok()?;
    match value.get("type")?.as_str()? {
        "turn.started" => Some("응답 생성 시작".into()),
        "turn.completed" | "result" => Some("응답 생성 완료".into()),
        "item.started" | "item.completed" => {
            let started = value.get("type")?.as_str()? == "item.started";
            match value.pointer("/item/type")?.as_str()? {
                "command_execution" => Some(if started { "명령 실행 중".into() } else {
                    let code = value.pointer("/item/exit_code").and_then(Value::as_i64);
                    code.map(|code| format!("명령 종료 · 코드 {code}")).unwrap_or_else(|| "명령 실행 완료".into())
                }),
                "file_change" => Some(if started { "파일 변경 중" } else { "파일 변경 완료" }.into()),
                "agent_message" if !started => Some("작업 결과 정리 완료".into()),
                _ => None,
            }
        }
        "assistant" => {
            let content = value.pointer("/message/content")?.as_array()?;
            let name = content.iter().find(|part| part.get("type").and_then(Value::as_str) == Some("tool_use"))?.get("name")?.as_str()?;
            let safe_name = match name { "Read" | "Edit" | "Write" | "Bash" | "Grep" | "Glob" | "Task" | "WebSearch" => name, _ => "Tool" };
            Some(format!("도구 실행 · {safe_name}"))
        }
        "user" if value.pointer("/message/content")?.as_array()?.iter().any(|part| part.get("type").and_then(Value::as_str) == Some("tool_result")) => Some("도구 결과 확인".into()),
        _ => None,
    }
}

pub(crate) fn send_json(connection: &AiConnection, url: &str, body: &Value) -> Result<Value, String> {
    let mut request = ureq::post(url).config().timeout_global(Some(Duration::from_secs(120))).build();
    if let Some(key_name) = connection.args.first() {
        if !key_name.is_empty() {
            let token = std::env::var(key_name).map_err(|_| format!("authentication: credential environment variable {key_name} is not set"))?;
            request = request.header("Authorization", &format!("Bearer {token}"));
        }
    }
    let mut response = request.send_json(body).map_err(|e| format!("HTTP request: {e}"))?;
    response.body_mut().read_json().map_err(|e| format!("HTTP response: {e}"))
}

pub(crate) fn run_plugin(connection: &AiConnection, prompt: &str, cwd: &Path, writable: bool) -> Result<RuntimeResult, String> {
    let mut command = Command::new(&connection.executable);
    command.args(&connection.args).current_dir(cwd).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
    // A console adapter (e.g. node.exe for the mock AI) must not open a window over the app.
    #[cfg(windows)] { use std::os::windows::process::CommandExt; command.creation_flags(0x0800_0000); }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let mut stdin = child.stdin.take().ok_or("Plugin has no stdin")?;
    let stdout = child.stdout.take().ok_or("Plugin has no stdout")?;
    let (tx, rx) = mpsc::channel();
    let reader = thread::spawn(move || {
        let mut reader = std::io::BufReader::new(stdout);
        let _ = tx.send(read_json_line(&mut reader));
        let _ = tx.send(read_json_line(&mut reader));
    });
    let init = json!({"id": 1, "method": "initialize", "params": {"protocolVersion": 1}});
    writeln!(stdin, "{init}").map_err(|e| e.to_string())?;
    let init_result = match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => { let _ = child.kill(); let _ = child.wait(); let _ = reader.join(); return Err(error); }
        Err(_) => { let _ = child.kill(); let _ = child.wait(); let _ = reader.join(); return Err("Plugin initialization timed out".into()); }
    };
    if init_result.get("error").is_some() { let _ = child.kill(); let _ = child.wait(); let _ = reader.join(); return Err(format!("Plugin initialization failed: {init_result}")); }
    if init_result.pointer("/result/protocolVersion").and_then(Value::as_u64) != Some(1) { let _ = child.kill(); let _ = child.wait(); let _ = reader.join(); return Err("Plugin protocol major version must be 1".into()); }
    let request = json!({"id": 2, "method": "send_task", "params": {"prompt": prompt, "cwd": cwd.to_string_lossy(), "modelId": connection.model_id, "writable": writable}});
    writeln!(stdin, "{request}").map_err(|e| e.to_string())?;
    let result = match rx.recv_timeout(Duration::from_secs(30 * 60)) {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => { let _ = child.kill(); let _ = child.wait(); let _ = reader.join(); return Err(error); }
        Err(_) => { let _ = child.kill(); let _ = child.wait(); let _ = reader.join(); return Err("Plugin task timed out".into()); }
    };
    let _ = child.kill();
    let _ = child.wait();
    let _ = reader.join();
    let text = result.pointer("/result/text").and_then(Value::as_str).ok_or_else(|| format!("Plugin returned no result: {result}"))?.to_string();
    let usage = token_usage(&result);
    Ok(RuntimeResult { text, session_id: result.pointer("/result/sessionId").and_then(Value::as_str).map(str::to_string), usage })
}

fn read_json_line(reader: &mut impl std::io::BufRead) -> Result<Value, String> {
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| e.to_string())?;
    serde_json::from_str(&line).map_err(|e| e.to_string())
}

#[cfg(test)]
pub fn extract_text(stdout: &str) -> String {
    crate::provider::extract_text(stdout)
}

pub(crate) fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::{capture_with_timeout, extract_text, execute, execute_with_progress, safe_diagnostic, summarize_runtime_event, token_usage};
    use std::process::Command;
    use std::time::Duration;
    #[test]
    fn process_error_keeps_exit_and_structured_failure_without_leaking_token() {
        let Some(node) = crate::discovery::find_executable("node") else { return; };
        let mut command = Command::new(node);
        command.arg("-e").arg("console.log(JSON.stringify({type:'error',message:'request failed'})); console.error('Reading additional input from stdin...'); process.exit(1)");
        let error = capture_with_timeout(command, Duration::from_secs(3), &mut |_| {}, &mut |_, _| {}, "Codex", None).unwrap_err();
        assert!(error.contains("process exit: code=1 provider=Codex"));
        assert!(error.contains("stdout_error=error: request failed"));
        assert!(error.contains("Reading additional input from stdin"));
        assert_eq!(safe_diagnostic("Bearer sk-secret", 100), "[redacted sensitive diagnostic]");
    }
    #[test]
    fn process_timeout_is_separate_from_nonzero_exit() {
        let Some(node) = crate::discovery::find_executable("node") else { return; };
        let mut command = Command::new(node);
        command.arg("-e").arg("setTimeout(() => {}, 3000)");
        let error = capture_with_timeout(command, Duration::from_millis(100), &mut |_| {}, &mut |_, _| {}, "Fixture", None).unwrap_err();
        assert!(error.contains("process timeout"));
        assert!(!error.contains("process exit"));
    }
    #[test]
    fn synthetic_large_stdin_is_written_and_closed_without_deadlock() {
        let Some(node) = crate::discovery::find_executable("node") else { return; };
        let mut command = Command::new(node);
        command.arg("-e").arg("let s='';process.stdin.setEncoding('utf8');process.stdin.on('data',x=>s+=x);process.stdin.on('end',()=>console.log(JSON.stringify({len:s.length,last:s.slice(-9)})))");
        let input = format!("{}LAST-WORD", "x".repeat(100_000));
        let (stdout, _) = capture_with_timeout(command, Duration::from_secs(10), &mut |_| {}, &mut |_, _| {}, "synthetic", Some(input.as_bytes())).unwrap();
        assert!(stdout.contains("\"len\":100009"));
        assert!(stdout.contains("LAST-WORD"));
    }
    #[test]
    fn parses_provider_usage_without_double_counting_codex_cache() {
        let codex = serde_json::json!({"type":"turn.completed","usage":{"input_tokens":100,"cached_input_tokens":40,"output_tokens":20}});
        let claude = serde_json::json!({"type":"result","usage":{"input_tokens":10,"cache_creation_input_tokens":5,"cache_read_input_tokens":7,"output_tokens":3}});
        assert_eq!(token_usage(&codex), Some((100,20)));
        assert_eq!(token_usage(&claude), Some((22,3)));
    }
    use crate::model::AiConnection;
    use crate::discovery::find_executable;
    #[cfg(windows)]
    #[test]
    fn npm_codex_entrypoint_receives_complex_goal_without_batch_shell() {
        let dir = std::env::temp_dir().join(format!("utopia-codex-wrapper-{}", crate::model::id()));
        let bin = dir.join("node_modules").join("@openai").join("codex").join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(dir.join("codex.cmd"), "@echo off\r\necho wrapper should not run\r\n").unwrap();
        std::fs::write(bin.join("codex.js"), "const args=process.argv.slice(2); console.log(JSON.stringify({type:'item.started',item:{type:'command_execution',command:'echo SECRET'}})); console.log(JSON.stringify({type:'item.completed',item:{type:'agent_message',text:args.at(-1)}}));").unwrap();
        let connection = AiConnection { id: "codex".into(), name: "Codex".into(), kind: "cli".into(), executable: dir.join("codex.cmd").to_string_lossy().into_owned(), endpoint: String::new(), args: vec![], model_id: String::new(), status: "ready".into(), capabilities: vec![] };
        let prompt = "팀 협업 구조를 설명하는 HTML 작성 & 검증 | `echo` ^ %PATH% \"인용\"";
        let mut milestones = Vec::new();
        let result = execute_with_progress(&connection, prompt, &dir, false, &mut |message| milestones.push(message.to_string())).unwrap();
        assert_eq!(result.text, prompt);
        assert!(milestones.iter().any(|message| message == "명령 실행 중"));
        assert!(!milestones.iter().any(|message| message.contains("SECRET")));
        let resolved = dir.canonicalize().unwrap();
        assert!(resolved.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(resolved.file_name().unwrap().to_string_lossy().starts_with("utopia-codex-wrapper-"));
        std::fs::remove_dir_all(resolved).unwrap();
    }
    #[test]
    fn extracts_codex_final_message() {
        let output = "{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"done\"}}\n";
        assert_eq!(extract_text(output), "done");
    }

    #[test]
    fn progress_summaries_do_not_copy_commands_or_reasoning() {
        let command = r#"{"type":"item.started","item":{"type":"command_execution","command":"echo SECRET_VALUE"}}"#;
        assert_eq!(summarize_runtime_event(command).as_deref(), Some("명령 실행 중"));
        let thought = r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"private thought"}]}}"#;
        assert_eq!(summarize_runtime_event(thought), None);
        let tool = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"secret.txt"}}]}}"#;
        assert_eq!(summarize_runtime_event(tool).as_deref(), Some("도구 실행 · Read"));
    }

    #[test]
    fn plugin_protocol_v1_can_write_only_when_requested() {
        let Some(node) = find_executable("node") else { return; };
        let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("tests").join("mock-adapter.mjs");
        let dir = std::env::temp_dir().join(format!("utopia-adapter-test-{}", crate::model::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let connection = AiConnection { id: "mock".into(), name: "Mock".into(), kind: "plugin".into(), executable: node.to_string_lossy().into_owned(), endpoint: String::new(), args: vec![script.to_string_lossy().into_owned()], model_id: String::new(), status: "ready".into(), capabilities: vec![] };
        let read = execute(&connection, "review", &dir, false).unwrap();
        assert!(read.text.starts_with("PASS"));
        assert!(!dir.join("adapter-result.txt").exists());
        let write = execute(&connection, "implement", &dir, true).unwrap();
        assert_eq!(write.session_id.as_deref(), Some("mock-session"));
        assert!(dir.join("adapter-result.txt").exists());
        let mut incompatible = connection.clone();
        incompatible.args.push("--invalid-version".into());
        assert!(execute(&incompatible, "review", &dir, false).unwrap_err().contains("protocol major version"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn custom_cli_receives_goal_text_as_one_literal_argument() {
        let node = find_executable("node").expect("Node is required for the test fixture");
        let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("tests").join("echo-cli.mjs");
        let dir = std::env::temp_dir().join(format!("utopia-literal-cli-{}", crate::model::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let connection = AiConnection {
            id: "literal".into(), name: "Literal CLI".into(), kind: "cli".into(),
            executable: node.to_string_lossy().into_owned(), endpoint: String::new(),
            args: vec![script.to_string_lossy().into_owned(), "{prompt}".into()],
            model_id: String::new(), status: "ready".into(), capabilities: vec![],
        };
        let payload = "CEO text; node -e \"require('fs').writeFileSync('canary.txt','bad')\"; $(echo x) `echo y`";
        let result = execute(&connection, payload, &dir, false).unwrap();
        assert_eq!(result.text, payload);
        assert!(!dir.join("canary.txt").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
