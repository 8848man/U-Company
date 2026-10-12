use crate::model::AiConnection;
use crate::runtime;
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;

#[derive(Debug)]
pub struct AiResponse {
    pub text: String,
    pub session_id: Option<String>,
    pub usage: Option<(u64, u64)>,
}

struct AiRequest<'a> {
    prompt: &'a str,
    cwd: &'a Path,
    writable: bool,
}

trait AiProvider {
    fn execute(&self, request: &AiRequest<'_>, progress: &mut dyn FnMut(&str), usage_progress: &mut dyn FnMut(u64, bool)) -> Result<AiResponse, String>;
}

struct CliProvider<'a>(&'a AiConnection);
struct HttpProvider<'a>(&'a AiConnection);
struct PluginProvider<'a>(&'a AiConnection);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PromptDelivery { Argument, Stdin }

/// Production switches remain off until the corresponding synthetic Windows
/// provider harness is recorded as passing (MAM-014 D1).
fn prompt_delivery(stem: &str) -> PromptDelivery {
    match stem {
        "claude" => PromptDelivery::Argument,
        "codex" => PromptDelivery::Argument,
        _ => PromptDelivery::Argument,
    }
}

impl AiProvider for CliProvider<'_> {
    fn execute(&self, request: &AiRequest<'_>, progress: &mut dyn FnMut(&str), usage_progress: &mut dyn FnMut(u64, bool)) -> Result<AiResponse, String> {
        run_cli(self.0, request.prompt, request.cwd, request.writable, progress, usage_progress, None)
    }
}

impl AiProvider for HttpProvider<'_> {
    fn execute(&self, request: &AiRequest<'_>, _: &mut dyn FnMut(&str), _: &mut dyn FnMut(u64, bool)) -> Result<AiResponse, String> {
        if request.writable { return Err("capability: HTTP models need a connected agent harness for file work".into()); }
        if self.0.model_id.is_empty() { return Err("configuration: select a model ID".into()); }
        let url = format!("{}/chat/completions", self.0.endpoint.trim_end_matches('/'));
        let body = json!({ "model": self.0.model_id, "messages": [{ "role": "user", "content": request.prompt }] });
        let result = runtime::send_json(self.0, &url, &body)?;
        let text = result.pointer("/choices/0/message/content").and_then(Value::as_str).ok_or("response: HTTP model returned no message content")?.to_string();
        let usage = runtime::token_usage(&result);
        Ok(AiResponse { text, session_id: result.get("id").and_then(Value::as_str).map(str::to_string), usage })
    }
}

impl AiProvider for PluginProvider<'_> {
    fn execute(&self, request: &AiRequest<'_>, _: &mut dyn FnMut(&str), _: &mut dyn FnMut(u64, bool)) -> Result<AiResponse, String> {
        runtime::run_plugin(self.0, request.prompt, request.cwd, request.writable)
    }
}

pub fn execute_with_observer(connection: &AiConnection, prompt: &str, cwd: &Path, writable: bool, progress: &mut dyn FnMut(&str), usage_progress: &mut dyn FnMut(u64, bool)) -> Result<AiResponse, String> {
    let request = AiRequest { prompt, cwd, writable };
    let result = match connection.kind.as_str() {
        "cli" => CliProvider(connection).execute(&request, progress, usage_progress),
        "http" => HttpProvider(connection).execute(&request, progress, usage_progress),
        "plugin" => PluginProvider(connection).execute(&request, progress, usage_progress),
        _ => Err("Unknown provider transport".into()),
    };
    let transport = match connection.kind.as_str() { "cli" => "ProcessRuntime", "http" => "HttpRuntime", "plugin" => "AdapterProcessRuntime", _ => "UnknownRuntime" };
    result.map_err(|error| format!("Provider={} Runtime={transport} Operation=execute: {}", connection.id, runtime::safe_diagnostic(&error, 700)))
}

fn run_cli(connection: &AiConnection, prompt: &str, cwd: &Path, writable: bool, progress: &mut dyn FnMut(&str), usage_progress: &mut dyn FnMut(u64, bool), delivery_override: Option<PromptDelivery>) -> Result<AiResponse, String> {
    let mut cmd = crate::discovery::runtime_command(connection).map_err(|error| format!("configuration: {error}"))?;
    let stem = Path::new(&connection.executable).file_stem().unwrap_or_default().to_string_lossy().to_ascii_lowercase();
    let delivery = delivery_override.unwrap_or_else(|| prompt_delivery(&stem));
    if delivery == PromptDelivery::Stdin && !["claude", "codex"].contains(&stem.as_str()) { return Err("stdin delivery is not verified for this provider".into()); }
    if stem == "codex" {
        cmd.args(["exec", "--json", "--skip-git-repo-check", "--sandbox", if writable { "workspace-write" } else { "read-only" }, "--cd"])
            .arg(cwd);
        if !connection.model_id.is_empty() { cmd.arg("--model").arg(&connection.model_id); }
        cmd.arg(if delivery == PromptDelivery::Stdin { "-" } else { prompt });
    } else if stem == "claude" {
        cmd.arg("-p");
        if delivery == PromptDelivery::Argument { cmd.arg(prompt); }
        cmd.args(["--output-format", "stream-json", "--verbose", "--permission-mode", if writable { "acceptEdits" } else { "plan" }]);
        if !connection.model_id.is_empty() { cmd.arg("--model").arg(&connection.model_id); }
    } else if stem == "gemini" {
        cmd.args(["-p", prompt, "--output-format", "stream-json"]);
        if !connection.model_id.is_empty() { cmd.arg("--model").arg(&connection.model_id); }
    } else if stem == "opencode" {
        cmd.args(["run", "--format", "json", prompt]);
        if !connection.model_id.is_empty() { cmd.arg("--model").arg(&connection.model_id); }
    } else {
        let mut has_prompt = false;
        for arg in &connection.args {
            if arg.contains("{prompt}") { has_prompt = true; }
            cmd.arg(arg.replace("{prompt}", prompt).replace("{cwd}", &cwd.to_string_lossy()).replace("{model}", &connection.model_id));
        }
        if !has_prompt { cmd.arg(prompt); }
    }
    cmd.current_dir(cwd);
    if delivery == PromptDelivery::Argument {
        let length: usize = cmd.get_args().map(|arg| crate::meeting::command_arg_len(&arg.to_string_lossy()) + 1).sum::<usize>() + connection.executable.encode_utf16().count() + 1;
        if length > crate::meeting::PACKET_CHAR_CAP { return Err(format!("prompt command length {length} exceeds {} UTF-16 units; no content was truncated", crate::meeting::PACKET_CHAR_CAP)); }
    }
    let input = (delivery == PromptDelivery::Stdin).then_some(prompt.as_bytes());
    let (stdout, stderr) = runtime::capture_with_timeout(cmd, Duration::from_secs(30 * 60), progress, usage_progress, &connection.name, input)?;
    let text = extract_text(&stdout);
    if text.trim().is_empty() { return Err(format!("response: no answer; stderr={}", runtime::truncate(&stderr, 300))); }
    let session_id = stdout.lines().filter_map(|line| serde_json::from_str::<Value>(line).ok()).find_map(|v| v.get("thread_id").or_else(|| v.get("session_id")).and_then(Value::as_str).map(str::to_string));
    let usage = stdout.lines().filter_map(|line| serde_json::from_str::<Value>(line).ok()).filter_map(|value| runtime::token_usage(&value)).last();
    Ok(AiResponse { text, session_id, usage })
}

/// Harness-only path. The caller must supply synthetic text from a disposable
/// directory; this does not change the production provider table.
pub(crate) fn execute_synthetic_stdin(connection: &AiConnection, prompt: &str, cwd: &Path) -> Result<AiResponse, String> {
    run_cli(connection, prompt, cwd, false, &mut |_| {}, &mut |_, _| {}, Some(PromptDelivery::Stdin))
}

pub fn extract_text(stdout: &str) -> String {
    let mut messages = Vec::new();
    for line in stdout.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else { continue; };
        if let Some(text) = value.pointer("/item/text").and_then(Value::as_str) {
            if value.get("type").and_then(Value::as_str) == Some("item.completed") { messages.push(text.to_string()); }
        } else if let Some(text) = value.get("response").and_then(Value::as_str) { messages.push(text.to_string()); }
        else if let Some(text) = value.pointer("/message/content/0/text").and_then(Value::as_str) { messages.push(text.to_string()); }
        else if let Some(text) = value.get("text").and_then(Value::as_str) { messages.push(text.to_string()); }
    }
    if messages.is_empty() { stdout.trim().to_string() } else { messages.last().cloned().unwrap_or_default() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;

    #[test]
    fn production_prompt_delivery_is_argument_until_provider_harness_passes() {
        for stem in ["claude", "codex", "gemini", "opencode", "custom"] {
            assert_eq!(prompt_delivery(stem), PromptDelivery::Argument);
        }
    }

    #[test]
    fn http_provider_normalizes_response_and_rejects_file_work() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(socket.try_clone().unwrap());
            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();
            assert!(request_line.contains("POST /chat/completions"));
            let mut length = 0;
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                if header == "\r\n" || header.is_empty() { break; }
                if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:") { length = value.trim().parse().unwrap(); }
            }
            let mut body_bytes = vec![0; length];
            reader.read_exact(&mut body_bytes).unwrap();
            let body = r#"{"id":"reply-1","choices":[{"message":{"content":"hello"}}],"usage":{"input_tokens":4,"output_tokens":2}}"#;
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        });
        let connection = AiConnection { id: "local".into(), name: "Local HTTP".into(), kind: "http".into(), executable: String::new(), endpoint: format!("http://{address}"), args: vec![], model_id: "test-model".into(), status: "ready".into(), capabilities: vec!["text".into()] };
        let cwd = std::env::temp_dir();
        let response = execute_with_observer(&connection, "hello", &cwd, false, &mut |_| {}, &mut |_, _| {}).unwrap();
        assert_eq!(response.text, "hello");
        assert_eq!(response.session_id.as_deref(), Some("reply-1"));
        assert_eq!(response.usage, Some((4, 2)));
        let denied = execute_with_observer(&connection, "write", &cwd, true, &mut |_| {}, &mut |_, _| {}).unwrap_err();
        assert!(denied.contains("Provider=local Runtime=HttpRuntime Operation=execute: capability"));
        server.join().unwrap();
    }

    #[test]
    fn missing_cli_reports_provider_and_process_layer() {
        let missing = std::env::temp_dir().join(format!("utopia-missing-provider-{}", crate::model::id()));
        let connection = AiConnection { id: "claude".into(), name: "Claude".into(), kind: "cli".into(), executable: missing.to_string_lossy().into_owned(), endpoint: String::new(), args: vec![], model_id: String::new(), status: "ready".into(), capabilities: vec![] };
        let error = execute_with_observer(&connection, "test", &std::env::temp_dir(), false, &mut |_| {}, &mut |_, _| {}).unwrap_err();
        assert!(error.contains("Provider=claude Runtime=ProcessRuntime Operation=execute: process spawn"));
    }
}
