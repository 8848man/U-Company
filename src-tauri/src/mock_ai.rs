//! ADR-0052: the mock AI is an ordinary `plugin` connection that runs `scripts/mock-ai/mock-adapter.mjs`
//! with Node. No engine branch knows it is a mock; it exists so a whole goal can run without tokens.

use crate::model::AiConnection;
use std::path::PathBuf;

pub const CONNECTION_ID: &str = "mock-ai";
pub const MODES: [&str; 3] = ["qa-fail-once", "plan-form-once", "needs-scope-once"];

/// The adapter script shipped in this repository (development and test builds run from the checkout).
pub fn script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("scripts").join("mock-ai").join("mock-adapter.mjs")
}

/// A ready mock connection for the given Node executable and failure modes.
pub fn connection(node: &std::path::Path, script: &std::path::Path, modes: &[String]) -> Result<AiConnection, String> {
    if let Some(unknown) = modes.iter().find(|mode| !MODES.contains(&mode.as_str())) { return Err(format!("모르는 모의 AI 방식입니다: {unknown}")); }
    let mut args = vec![script.to_string_lossy().into_owned()];
    if !modes.is_empty() { args.push("--mode".into()); args.push(modes.join(",")); }
    let label = if modes.is_empty() { String::new() } else { format!(" · {}", modes.join(", ")) };
    Ok(AiConnection {
        id: CONNECTION_ID.into(),
        name: format!("모의 AI (테스트용, 토큰 없음){label}"),
        kind: "plugin".into(),
        executable: node.to_string_lossy().into_owned(),
        endpoint: String::new(),
        args,
        model_id: "mock-scenario".into(),
        status: "ready".into(),
        capabilities: vec!["agent".into(), "files".into(), "tools".into()],
    })
}

/// Finds Node and the adapter, or explains what is missing.
pub fn local_connection(modes: &[String]) -> Result<AiConnection, String> {
    let node = crate::discovery::find_executable("node").ok_or("모의 AI를 쓰려면 Node.js가 필요해요. Node.js를 설치하고 앱을 다시 시작해 주세요")?;
    let script = script_path();
    if !script.is_file() { return Err(format!("모의 AI 스크립트를 찾지 못했어요: {}. 저장소에서 빌드한 앱에서만 쓸 수 있어요", script.display())); }
    let script = script.canonicalize().map_err(|e| e.to_string())?;
    // Windows canonical paths carry a `\\?\` prefix; Node and the connection card read the plain form.
    let plain = script.to_string_lossy().trim_start_matches(r"\\?\").to_string();
    connection(&node, std::path::Path::new(&plain), modes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_connection_is_a_file_capable_plugin_and_rejects_unknown_modes() {
        let made = connection(std::path::Path::new("node"), std::path::Path::new("adapter.mjs"), &["qa-fail-once".into()]).unwrap();
        assert_eq!(made.kind, "plugin");
        assert!(crate::delegation::can_write_files(&made).is_ok());
        assert!(crate::delegation::usable(&made).is_ok());
        assert_eq!(made.args, vec!["adapter.mjs", "--mode", "qa-fail-once"]);
        assert!(connection(std::path::Path::new("node"), std::path::Path::new("a.mjs"), &["boom".into()]).is_err());
        assert!(script_path().is_file(), "the adapter ships in the repository");
    }
}
