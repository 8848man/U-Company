use crate::model::{AiConnection, DiscoveredAi};
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// `--version` of a freshly installed large native CLI can take several seconds on Windows
/// (first-run antivirus scan); `auth status` may also refresh a token over the network.
const VERSION_PROBE_TIMEOUT: Duration = Duration::from_secs(15);
const AUTH_PROBE_TIMEOUT: Duration = Duration::from_secs(20);

pub fn find_executable(name: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    find_executable_in(name, env::split_paths(&paths))
}

fn find_executable_in(name: &str, dirs: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    let extensions: &[&str] = if cfg!(windows) { &[".exe", ".cmd", ".bat"] } else { &[""] };
    for dir in dirs {
        for ext in extensions {
            let candidate = dir.join(format!("{name}{ext}"));
            if candidate.is_file() { return Some(candidate); }
        }
    }
    None
}

pub fn probe_executable(path: &Path) -> bool {
    probe_version(path).is_ok()
}

pub fn runtime_command(connection: &AiConnection) -> Result<Command, String> {
    let path = Path::new(&connection.executable);
    let batch = cfg!(windows) && matches!(path.extension().and_then(|s| s.to_str()).map(str::to_ascii_lowercase).as_deref(), Some("cmd" | "bat"));
    if !batch { return Ok(Command::new(path)); }
    let parent = path.parent().ok_or("CLI 경로의 상위 폴더를 찾지 못했습니다")?;
    match connection.id.as_str() {
        "claude" => {
            let native = parent.join("node_modules").join("@anthropic-ai").join("claude-code").join("bin").join("claude.exe");
            if native.is_file() { Ok(Command::new(native)) } else { Err(format!("Claude npm 래퍼의 실제 claude.exe를 찾지 못했습니다: {}. Claude를 다시 설치하거나 네이티브 실행 파일을 지정하세요.", native.display())) }
        }
        "codex" => {
            let entry = parent.join("node_modules").join("@openai").join("codex").join("bin").join("codex.js");
            if !entry.is_file() { return Err(format!("Codex npm 진입점을 찾지 못했습니다: {}. Codex를 다시 설치하거나 네이티브 실행 파일을 지정하세요.", entry.display())); }
            let node = find_executable("node").ok_or("Node 실행 파일을 찾지 못했습니다. Node를 설치하거나 네이티브 Codex 실행 파일을 지정하세요")?;
            let mut command = Command::new(node);
            command.arg(entry);
            Ok(command)
        }
        _ => Err(format!("Windows 배치 래퍼에는 목표 문장을 안전하게 전달할 수 없습니다: {}. 네이티브 실행 파일이나 외부 어댑터를 지정하세요.", path.display())),
    }
}

pub fn repaired_known_windows_cli_path(path: &Path, id: &str) -> Option<PathBuf> {
    if !cfg!(windows) || !["codex", "claude", "gemini", "opencode"].contains(&id) { return None; }
    if path.extension().is_some() || path.file_name()?.to_string_lossy().to_ascii_lowercase() != id { return None; }
    for extension in ["exe", "cmd", "bat"] {
        let candidate = path.with_extension(extension);
        if candidate.is_file() && probe_executable(&candidate) { return Some(candidate); }
    }
    None
}

fn probe_version(path: &Path) -> Result<String, String> {
    let started = Instant::now();
    let mut child = Command::new(path).arg("--version").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| e.to_string())?;
    let deadline = started + VERSION_PROBE_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child.wait_with_output().map_err(|e| e.to_string())?;
                let combined = format!("{} {}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr)).trim().to_string();
                return if status.success() { Ok(combined.chars().take(160).collect()) } else { Err(format!("--version exited {status}: {}", combined.chars().take(160).collect::<String>())) };
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            _ => { let _ = child.kill(); let _ = child.wait(); return Err(format!("--version이 {}초 안에 끝나지 않았습니다", VERSION_PROBE_TIMEOUT.as_secs())); }
        }
    }
}

pub fn discover() -> Vec<DiscoveredAi> {
    let mut found = Vec::new();
    for (id, name) in [("codex", "Codex CLI"), ("claude", "Claude Code"), ("gemini", "Gemini CLI"), ("opencode", "OpenCode")] {
        if let Some(path) = find_executable(id) {
            let connection = AiConnection { id: id.into(), name: name.into(), kind: "cli".into(), executable: path.to_string_lossy().into_owned(), endpoint: String::new(), args: Vec::new(), model_id: String::new(), status: String::new(), capabilities: vec![] };
            found.push(inspect_cli_connection(&connection));
        } else {
            let connection = AiConnection { id: id.into(), name: name.into(), kind: "cli".into(), executable: String::new(), endpoint: String::new(), args: vec![], model_id: String::new(), status: "missing".into(), capabilities: vec![] };
            found.push(DiscoveredAi { connection, source: "preset".into(), version: None, detail: "이 PC의 PATH에서 실행 파일을 찾지 못했습니다".into(), next_action: "도구를 직접 설치하거나 아래에서 실행 파일 경로를 지정하세요".into() });
        }
    }
    for (id, name, url, catalog_url) in [
        ("ollama", "Ollama", "http://127.0.0.1:11434/v1", "http://127.0.0.1:11434/api/tags"),
        ("lmstudio", "LM Studio", "http://127.0.0.1:1234/v1", "http://127.0.0.1:1234/v1/models"),
    ] {
        if let Some(models) = probe_models(catalog_url) {
            for model in models {
                let connection = AiConnection { id: format!("{id}:{model}"), name: format!("{name} · {model}"), kind: "http".into(), executable: String::new(), endpoint: url.into(), args: vec![], model_id: model, status: "ready".into(), capabilities: vec!["text".into()] };
                found.push(DiscoveredAi { connection, source: "local".into(), version: None, detail: format!("로컬 모델 서버 {url}에서 모델 응답 확인"), next_action: "등록한 뒤 Office에서 텍스트 역할에 배정하세요".into() });
            }
        }
    }
    found
}

pub fn inspect_cli_connection(connection: &AiConnection) -> DiscoveredAi {
    let path = Path::new(&connection.executable);
    let probe = probe_version(path);
    let auth = if probe.is_ok() && connection.id == "claude" { Some(probe_claude_auth(path)) } else { None };
    let status = if probe.is_err() { "failed" }
        else if connection.id == "codex" { if probe_codex_auth(path) { "ready" } else { "needs_setup" } }
        else if let Some(result) = &auth { match result { Ok(true) => "ready", Ok(false) => "needs_setup", Err(_) => "needs_auth_check" } }
        else { "needs_auth_check" };
    let detail = match (&probe, &auth) {
        (Err(error), _) => format!("실행 경로 {}의 버전 확인에 실패했습니다: {error}", path.display()),
        (Ok(version), Some(Ok(true))) => format!("{} · 버전 {version} · Claude auth status: loggedIn=true", path.display()),
        (Ok(version), Some(Ok(false))) => format!("{} · 버전 {version} · Claude auth status: loggedIn=false. 이 앱 실행 환경에서는 로그인되지 않은 상태로 보고됩니다.", path.display()),
        (Ok(version), Some(Err(error))) => format!("{} · 버전 {version} · Claude 로그인 상태를 확인하지 못했습니다: {error}", path.display()),
        (Ok(version), None) if status == "ready" => format!("{} · 버전 {version} · 로그인 확인 완료", path.display()),
        (Ok(version), None) if status == "needs_setup" => format!("{} · 버전 {version} · CLI 로그인 확인 실패", path.display()),
        (Ok(version), None) => format!("{} · 버전 {version} · 로그인 상태 미확인", path.display()),
    };
    let next_action = match status {
        "ready" => "Office에서 역할에 배정할 수 있습니다".into(),
        "needs_setup" if connection.id == "claude" => format!("같은 Windows 사용자로 {} auth login을 실행하고, auth status가 loggedIn=true인지 확인한 뒤 앱에서 다시 검사하세요", path.display()),
        "needs_setup" => "CLI에서 로그인한 뒤 다시 검사하세요".into(),
        "failed" => "표시된 실행 경로를 터미널에서 직접 확인하거나 연결 경로를 수정하세요".into(),
        _ => "표시된 실행 경로에서 auth status를 직접 확인한 뒤 다시 검사하세요".into(),
    };
    let mut checked = connection.clone();
    checked.status = status.into();
    checked.capabilities = if probe.is_ok() { vec!["agent".into(), "files".into(), "tools".into()] } else { vec![] };
    DiscoveredAi { connection: checked, source: "local".into(), version: probe.ok(), detail, next_action }
}

fn login_args(id: &str) -> Option<&'static [&'static str]> {
    match id {
        "claude" | "opencode" => Some(&["auth", "login"]),
        "codex" => Some(&["login"]),
        "gemini" => Some(&[]),
        _ => None,
    }
}

pub fn start_login(connection: &AiConnection) -> Result<String, String> {
    if connection.kind != "cli" { return Err("대화형 로그인은 CLI 연결에서만 사용할 수 있습니다".into()); }
    let args = login_args(&connection.id).ok_or("이 AI의 로그인 명령은 등록되지 않았습니다. 제공자 안내에 따라 터미널에서 로그인하세요")?;
    let executable = Path::new(&connection.executable);
    if !executable.is_file() { return Err(format!("저장된 실행 파일을 찾지 못했습니다: {}", executable.display())); }
    if cfg!(windows) && !matches!(executable.extension().and_then(|value| value.to_str()).map(str::to_ascii_lowercase).as_deref(), Some("exe" | "cmd" | "bat")) {
        return Err("Windows에서 실행할 수 있는 .exe 또는 .cmd 경로를 등록한 뒤 로그인하세요".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // `cmd /c start` gives PowerShell its own console with its own input/output. Spawning it
        // directly would inherit the app's standard handles, so the login prompt could land in the
        // launcher terminal (or nowhere) instead of the new window. The script travels as
        // -EncodedCommand, so no shell ever re-parses the saved path or provider arguments.
        Command::new("cmd.exe")
            .raw_arg(login_console_command(&connection.executable, args))
            .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW for cmd.exe itself only
            .spawn()
            .map_err(|error| format!("로그인 터미널을 열지 못했습니다: {error}"))?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("x-terminal-emulator")
            .arg("-e")
            .arg(executable)
            .args(args)
            .spawn()
            .map_err(|error| format!("로그인 터미널을 열지 못했습니다. x-terminal-emulator 설치 상태를 확인하세요: {error}"))?;
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    { return Err("이 운영체제의 로그인 터미널은 아직 지원하지 않습니다".into()); }
    Ok(format!("{} 로그인 창을 열었습니다. 로그인 후 저장된 경로를 다시 검사하세요.", connection.name))
}

/// `cmd.exe` argument line that opens a separate PowerShell console running the provider login.
/// Only a saved path and fixed provider arguments enter the script; single quotes are escaped for
/// PowerShell literal syntax and the whole script is UTF-16LE base64 (-EncodedCommand).
#[cfg_attr(not(windows), allow(dead_code))]
fn login_console_command(executable: &str, args: &[&str]) -> String {
    let literal = executable.replace('\'', "''");
    let suffix = if args.is_empty() { String::new() } else { format!(" {}", args.join(" ")) };
    let script = format!("$Host.UI.RawUI.WindowTitle = 'AI 로그인'; & '{literal}'{suffix}; Write-Host ''; Write-Host '로그인을 마친 뒤 앱에서 이 경로로 다시 검사를 누르세요.'");
    let utf16: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    format!("/d /c start \"\" powershell.exe -NoProfile -NoExit -EncodedCommand {}", base64(&utf16))
}

#[cfg_attr(not(windows), allow(dead_code))]
fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        for (index, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            out.push(if index <= chunk.len() { TABLE[(n >> shift & 63) as usize] as char } else { '=' });
        }
    }
    out
}

fn probe_codex_auth(path: &Path) -> bool {
    let Ok(mut child) = Command::new(path).args(["login", "status"]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() else { return false; };
    let deadline = Instant::now() + AUTH_PROBE_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            _ => { let _ = child.kill(); let _ = child.wait(); return false; }
        }
    }
}

fn probe_claude_auth(path: &Path) -> Result<bool, String> {
    let mut command = if cfg!(windows) && matches!(path.extension().and_then(|s| s.to_str()).map(str::to_ascii_lowercase).as_deref(), Some("cmd" | "bat")) {
        let connection = AiConnection { id: "claude".into(), name: String::new(), kind: "cli".into(), executable: path.to_string_lossy().into_owned(), endpoint: String::new(), args: vec![], model_id: String::new(), status: String::new(), capabilities: vec![] };
        runtime_command(&connection)?
    } else { Command::new(path) };
    let started = Instant::now();
    let child = command.args(["auth", "status"]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| format!("auth status를 실행하지 못했습니다: {e}"))?;
    // Read both pipes on another thread so a chatty CLI can never block on a full pipe.
    let (sender, receiver) = std::sync::mpsc::channel();
    let id = child.id();
    thread::spawn(move || { let _ = sender.send(child.wait_with_output()); });
    let result = match receiver.recv_timeout(AUTH_PROBE_TIMEOUT) {
        Ok(result) => result.map_err(|e| e.to_string())?,
        Err(_) => { kill_process(id); return Err(format!("auth status가 {}초 안에 끝나지 않았습니다", AUTH_PROBE_TIMEOUT.as_secs())); }
    };
    let elapsed = started.elapsed().as_secs_f32();
    // The CLI emits a small JSON document. Never include its identity fields in logs or UI.
    interpret_claude_auth(&result.stdout, result.status.success(), &result.status.to_string())
        .map_err(|error| format!("{error} · {elapsed:.1}초{}", stderr_hint(&result.stderr)))
}

fn interpret_claude_auth(output: &[u8], success: bool, status: &str) -> Result<bool, String> {
    match parse_claude_auth(output) {
        Some(false) => Ok(false),
        Some(true) if success => Ok(true),
        Some(true) => Err(format!("loggedIn=true가 반환됐지만 종료 코드가 {status}입니다")),
        None => Err(format!("auth status 응답에 loggedIn 값이 없습니다 (종료 코드 {status}, 출력 앞부분: {})", redacted_head(output))),
    }
}

fn kill_process(id: u32) {
    #[cfg(windows)]
    { let _ = Command::new("taskkill").args(["/PID", &id.to_string(), "/T", "/F"]).stdout(Stdio::null()).stderr(Stdio::null()).status(); }
    #[cfg(not(windows))]
    { let _ = Command::new("kill").args(["-9", &id.to_string()]).status(); }
}

/// First line of CLI output with anything that looks like an account identifier masked.
fn redacted_head(output: &[u8]) -> String {
    let text = String::from_utf8_lossy(output);
    let line = text.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or("(비어 있음)");
    let masked: Vec<String> = line.split_whitespace().map(|word| if word.contains('@') || word.len() > 40 { "<숨김>".to_string() } else { word.to_string() }).collect();
    masked.join(" ").chars().take(120).collect()
}

fn stderr_hint(stderr: &[u8]) -> String {
    if String::from_utf8_lossy(stderr).trim().is_empty() { String::new() } else { format!(" · 오류 출력: {}", redacted_head(stderr)) }
}

fn parse_claude_auth(output: &[u8]) -> Option<bool> {
    let logged_in = |value: serde_json::Value| value.get("loggedIn").and_then(serde_json::Value::as_bool);
    if let Some(found) = serde_json::from_slice::<serde_json::Value>(output).ok().and_then(logged_in) { return Some(found); }
    // Tolerate update notices or warnings printed around the JSON document.
    let text = String::from_utf8_lossy(output);
    let (start, end) = (text.find('{')?, text.rfind('}')?);
    if start >= end { return None; }
    serde_json::from_str::<serde_json::Value>(&text[start..=end]).ok().and_then(logged_in)
}

fn probe_models(url: &str) -> Option<Vec<String>> {
    let response = ureq::get(url).config().timeout_global(Some(Duration::from_secs(2))).build().call().ok()?;
    let body: serde_json::Value = response.into_body().read_json().ok()?;
    let models = body.get("models").or_else(|| body.get("data"))?.as_array()?;
    Some(models.iter().filter_map(|m| m.get("name").or_else(|| m.get("id")).and_then(|v| v.as_str()).map(str::to_string)).collect())
}

pub fn list_models(connection: &AiConnection) -> Result<Vec<String>, String> {
    if connection.kind != "http" { return Ok(Vec::new()); }
    let base = connection.endpoint.trim_end_matches('/');
    if !(base.starts_with("http://127.0.0.1:") || base.starts_with("http://localhost:") || base.starts_with("https://")) {
        return Err("로컬 HTTP 주소 또는 HTTPS 주소를 입력해 주세요".into());
    }
    let url = format!("{base}/models");
    let mut request = ureq::get(&url).config().timeout_global(Some(Duration::from_secs(5))).build();
    if let Some(key_name) = connection.args.first().filter(|value| !value.is_empty()) {
        let token = env::var(key_name).map_err(|_| format!("인증 환경 변수 {key_name}를 찾지 못했습니다"))?;
        request = request.header("Authorization", &format!("Bearer {token}"));
    }
    let response = request.call().map_err(|e| format!("모델 목록을 가져올 수 없습니다 ({url}): {e}"))?;
    let body: serde_json::Value = response.into_body().read_json().map_err(|e| format!("모델 목록 응답을 읽을 수 없습니다: {e}"))?;
    let rows = body.get("data").or_else(|| body.get("models")).and_then(|value| value.as_array()).ok_or("모델 목록에 data 또는 models 배열이 없습니다")?;
    let mut names: Vec<String> = rows.iter().filter_map(|row| row.get("id").or_else(|| row.get("name")).and_then(|value| value.as_str()).map(str::to_string)).collect();
    names.sort();
    names.dedup();
    if names.is_empty() { return Err("사용 가능한 모델이 없습니다. 모델 서버를 확인해 주세요".into()); }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_commands_are_fixed_to_builtin_providers() {
        assert_eq!(login_args("claude"), Some(["auth", "login"].as_slice()));
        assert_eq!(login_args("codex"), Some(["login"].as_slice()));
        assert_eq!(login_args("gemini"), Some([].as_slice()));
        assert_eq!(login_args("opencode"), Some(["auth", "login"].as_slice()));
        assert_eq!(login_args("custom"), None);
    }

    #[test]
    fn claude_auth_requires_explicit_logged_in_boolean() {
        assert_eq!(parse_claude_auth(br#"{"loggedIn":true}"#), Some(true));
        assert_eq!(parse_claude_auth(br#"{"loggedIn":false}"#), Some(false));
        assert_eq!(parse_claude_auth(br#"{"other":"value"}"#), None);
        assert_eq!(interpret_claude_auth(br#"{"loggedIn":false}"#, false, "1"), Ok(false));
        assert_eq!(interpret_claude_auth(br#"{"loggedIn":true}"#, true, "0"), Ok(true));
        assert!(interpret_claude_auth(br#"{"loggedIn":true}"#, false, "1").is_err());
        assert_eq!(parse_claude_auth(b"Update available: 2.1.296\n{\"loggedIn\":true,\"authMethod\":\"claude.ai\"}\n"), Some(true));
        assert_eq!(parse_claude_auth(b"Logged in as someone@example.com"), None);
        let error = interpret_claude_auth(b"Logged in as someone@example.com via claude.ai", true, "0").unwrap_err();
        assert!(error.contains("Logged in as <\u{c228}\u{ae40}> via claude.ai") && !error.contains("example.com"), "{error}");
        assert!(interpret_claude_auth(br#"{}"#, false, "1").is_err());
    }

    #[test]
    fn lists_model_ids_from_openai_compatible_endpoint() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0u8; 1024];
            let size = stream.read(&mut request).unwrap();
            assert!(String::from_utf8_lossy(&request[..size]).starts_with("GET /v1/models "));
            let body = r#"{"data":[{"id":"model-b"},{"id":"model-a"},{"id":"model-a"}]}"#;
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        });
        let connection = AiConnection { id: "test".into(), name: "Test".into(), kind: "http".into(), executable: String::new(), endpoint: format!("http://127.0.0.1:{}/v1", address.port()), args: vec![], model_id: String::new(), status: "ready".into(), capabilities: vec![] };
        assert_eq!(list_models(&connection).unwrap(), vec!["model-a", "model-b"]);
        server.join().unwrap();
    }

    #[test]
    fn local_cli_results_have_real_executable_paths() {
        for item in discover().into_iter().filter(|item| item.source == "local" && item.connection.kind == "cli") {
            assert!(Path::new(&item.connection.executable).is_file(), "{} was marked local without a file", item.connection.name);
            assert_ne!(item.connection.status, "missing");
        }
    }

    #[test]
    fn login_console_is_a_separate_window_with_an_encoded_script() {
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        let line = login_console_command("C:\\Users\\O'Neil\\npm\\claude.cmd", &["auth", "login"]);
        assert!(line.starts_with("/d /c start \"\" powershell.exe -NoProfile -NoExit -EncodedCommand "), "{line}");
        let encoded = line.rsplit(' ').next().unwrap();
        assert!(encoded.chars().all(|c| c.is_ascii_alphanumeric() || "+/=".contains(c)), "no shell metacharacters reach cmd.exe");
        assert!(!line.contains("O'Neil") && !line.contains('&'));
    }

    #[cfg(unix)]
    #[test]
    fn claude_auth_probe_waits_beyond_three_seconds_and_reads_stderr_safely() {
        use std::os::unix::fs::PermissionsExt;
        let folder = std::env::temp_dir().join(format!("utopia-claude-auth-{}", crate::model::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let script = folder.join("claude");
        std::fs::write(&script, "#!/bin/sh\nsleep 4\necho 'warning for someone@example.com' >&2\necho '{\"loggedIn\":true,\"email\":\"someone@example.com\"}'\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(probe_claude_auth(&script), Ok(true));
        std::fs::write(&script, "#!/bin/sh\necho 'not json' \necho 'oops someone@example.com' >&2\nexit 1\n").unwrap();
        let error = probe_claude_auth(&script).unwrap_err();
        assert!(error.contains("not json") && error.contains("oops") && !error.contains("example.com"), "{error}");
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn windows_prefers_runnable_cmd_over_extensionless_npm_shim() {
        let folder = std::env::temp_dir().join(format!("utopia-cli-discovery-{}", crate::model::id()));
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("claude"), "#!/bin/sh\n").unwrap();
        std::fs::write(folder.join("claude.cmd"), "@echo off\r\necho test\r\n").unwrap();
        assert_eq!(find_executable_in("claude", [folder.clone()]), Some(folder.join("claude.cmd")));
        assert_eq!(probe_version(&folder.join("claude.cmd")).unwrap(), "test");
        assert_eq!(repaired_known_windows_cli_path(&folder.join("claude"), "claude"), Some(folder.join("claude.cmd")));
        assert!(repaired_known_windows_cli_path(&folder.join("claude"), "custom").is_none());
        let resolved = folder.canonicalize().unwrap();
        assert!(resolved.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(resolved.file_name().unwrap().to_string_lossy().starts_with("utopia-cli-discovery-"));
        std::fs::remove_dir_all(resolved).unwrap();
    }
}
