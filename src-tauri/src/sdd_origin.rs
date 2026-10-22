//! SDD origin (SI-03, ADR-0044): read-only static detection of a project's spec workflow,
//! and three CEO-confirmed structure actions (`bootstrap`, `add_section`, `convert`).
//! Detection never writes, follows links, calls an AI or sends content.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const SDD_DIR: &str = ".ucompany/sdd";
pub const LEGACY_DIR: &str = ".ucompany/legacy";
pub const INDEX_FILE: &str = ".ucompany/sdd/00_index.md";
pub const GRAPH_FILE: &str = ".ucompany/sdd/graph.json";
pub const MANAGED_START: &str = "<!-- U-SDD:START -->";
pub const MANAGED_END: &str = "<!-- U-SDD:END -->";
const MAX_FILE: u64 = 512 * 1024;
const MAX_DEPTH: usize = 4;
const MAX_DIRS: usize = 5000;
const MAX_RULE_FILES: usize = 50;
const SHORT_GUIDANCE: usize = 80;
const IGNORE_DIRS: &[&str] = &[".git", "node_modules", "vendor", "dist", "build", ".venv", "venv", "__pycache__", "target", ".next", ".gradle", "Pods"];
const GUIDE_FILES: &[&str] = &["AGENTS.md", "CLAUDE.md", "GEMINI.md", ".cursorrules", ".github/copilot-instructions.md"];
const GUIDE_DIRS: &[&str] = &[".cursor/rules"];
const EXTERNAL_SDD: &[&str] = &["specs", ".specify", "openspec", ".kiro/specs"];
const SIGNAL_DIR_NAMES: &[&str] = &[".ucompany", ".cursor", ".specify", "openspec", ".kiro", "specs"];
const WORKFLOW_KEYWORDS: &[&str] = &["spec", "스펙", "rfc", "설계 문서", "design/", "tasks", "요구사항", "수용 기준", "acceptance", "requirement"];

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OriginSignal {
    pub path: String,
    /// `u_sdd` | `guideline` | `guideline_dir` | `external_sdd`
    pub kind: String,
    pub workflow: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginScope {
    pub dir: String,
    pub mode: String,
    pub signals: Vec<OriginSignal>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewCandidate {
    pub path: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginStatus {
    /// `bootstrap` | `existing` | `ask` | `continue`
    pub mode: String,
    /// "." for the project root, or the subfolder whose rules apply.
    pub scope: String,
    pub evidence: Vec<String>,
    /// The rule file followed when several guidance files carry rules (AGENTS.md first).
    pub followed: Option<String>,
    pub conflict: bool,
    pub tool_rules: Vec<String>,
    pub actions: Vec<String>,
    pub next_task: Option<String>,
    pub scopes: Vec<OriginScope>,
    pub review: Vec<ReviewCandidate>,
    /// ADR-0045 "이 폴더는 묻지 않기"; filled from app state by the command, never from project files.
    pub muted: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedChange {
    pub path: String,
    /// `create` | `append` | `replace` | `backup`
    pub change: String,
    /// ADR-0045: entry files that get `append`/`replace` carry their text before and after, bounded for display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}

/// Longest preview text per side; longer text is cut with a marker, the write itself is never cut.
pub const PREVIEW_CHARS: usize = 6000;

fn preview_text(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    if text.chars().count() <= PREVIEW_CHARS { return text.into_owned(); }
    format!("{}\n… (이후 내용 생략)", text.chars().take(PREVIEW_CHARS).collect::<String>())
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginPlan {
    pub action: String,
    pub changes: Vec<PlannedChange>,
    pub digest: String,
}

fn rel(root: &Path, path: &Path) -> String {
    let text = path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/");
    if text.is_empty() { ".".into() } else { text }
}

fn is_link(path: &Path) -> bool {
    std::fs::symlink_metadata(path).map(|m| m.file_type().is_symlink()).unwrap_or(false)
}

fn real_dir(path: &Path) -> bool {
    std::fs::symlink_metadata(path).map(|m| m.is_dir() && !m.file_type().is_symlink()).unwrap_or(false)
}

fn real_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path).map(|m| m.is_file() && !m.file_type().is_symlink()).unwrap_or(false)
}

/// Bounded read of a regular file. A file over the limit or a link is refused.
fn read_bounded(path: &Path) -> Result<Option<Vec<u8>>, String> {
    let Ok(meta) = std::fs::symlink_metadata(path) else { return Ok(None) };
    if meta.file_type().is_symlink() { return Err(format!("{}은(는) 링크라서 다루지 않습니다", path.display())); }
    if !meta.is_file() { return Err(format!("{}은(는) 일반 파일이 아닙니다", path.display())); }
    if meta.len() > MAX_FILE { return Err(format!("{}은(는) 512 KiB를 넘어 다루지 않습니다", path.display())); }
    std::fs::read(path).map(Some).map_err(|e| format!("{}을(를) 읽지 못했습니다: {e}", path.display()))
}

fn has_workflow(text: &str) -> bool {
    let lower = text.to_lowercase();
    WORKFLOW_KEYWORDS.iter().any(|k| lower.contains(&k.to_lowercase()))
}

fn is_bridge(text: &str) -> bool {
    text.contains("@AGENTS.md") && text.trim().len() < 200
}

fn has_documents(dir: &Path, depth: usize, budget: &mut usize) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else { return false };
    for entry in entries.flatten() {
        if *budget == 0 { return false; }
        *budget -= 1;
        let Ok(kind) = entry.file_type() else { continue };
        let path = entry.path();
        if kind.is_file() && matches!(path.extension().and_then(|e| e.to_str()), Some("md" | "mdc")) { return true; }
        if kind.is_dir() && depth > 0 && has_documents(&path, depth - 1, budget) { return true; }
    }
    false
}

fn scope_signals(root: &Path, dir: &Path) -> Vec<OriginSignal> {
    let mut out = Vec::new();
    if real_dir(&dir.join(SDD_DIR)) {
        out.push(OriginSignal { path: rel(root, &dir.join(SDD_DIR)) + "/", kind: "u_sdd".into(), workflow: true });
    }
    for name in GUIDE_FILES {
        let file = dir.join(name);
        if !real_file(&file) { continue; }
        // A guidance file over the size limit still carries rules; it is listed but never parsed.
        let text = match read_bounded(&file) {
            Ok(Some(bytes)) => String::from_utf8_lossy(&bytes).into_owned(),
            Ok(None) => continue,
            Err(_) => { out.push(OriginSignal { path: rel(root, &file), kind: "guideline".into(), workflow: true }); continue; }
        };
        if text.trim().is_empty() { continue; } // an empty guidance file counts as absent
        out.push(OriginSignal { path: rel(root, &file), kind: "guideline".into(), workflow: has_workflow(&text) });
    }
    for name in GUIDE_DIRS {
        let path = dir.join(name);
        let mut budget = 200;
        if real_dir(&path) && has_documents(&path, 2, &mut budget) {
            out.push(OriginSignal { path: rel(root, &path) + "/", kind: "guideline_dir".into(), workflow: true });
        }
    }
    for name in EXTERNAL_SDD {
        let path = dir.join(name);
        let mut budget = 400;
        if real_dir(&path) && has_documents(&path, 3, &mut budget) {
            out.push(OriginSignal { path: rel(root, &path) + "/", kind: "external_sdd".into(), workflow: true });
        }
    }
    out
}

fn scope_mode(signals: &[OriginSignal]) -> &'static str {
    if signals.iter().any(|s| s.kind == "u_sdd") { return "continue"; }
    if signals.is_empty() { return "bootstrap"; }
    let files: Vec<_> = signals.iter().filter(|s| s.kind == "guideline").collect();
    if files.len() == signals.len() && !files.iter().any(|s| s.workflow) { return "ask"; }
    "existing"
}

fn walk(root: &Path, dir: &Path, depth: usize, visited: &mut usize, scopes: &mut Vec<OriginScope>) {
    if *visited >= MAX_DIRS { return; }
    *visited += 1;
    let signals = scope_signals(root, dir);
    if !signals.is_empty() || dir == root {
        scopes.push(OriginScope { dir: rel(root, dir), mode: scope_mode(&signals).into(), signals });
    }
    if depth >= MAX_DEPTH { return; }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut children: Vec<PathBuf> = entries.flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false)) // read_dir file types do not follow links
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| !IGNORE_DIRS.contains(&n) && !SIGNAL_DIR_NAMES.contains(&n) && n != ".github"))
        .collect();
    children.sort();
    for child in children { walk(root, &child, depth + 1, visited, scopes); }
}

fn read_text(path: &Path) -> Option<String> {
    read_bounded(path).ok().flatten().map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// First unfinished task of the graph's focus feature, e.g. "T-003 overdue 구현".
fn next_task(root: &Path) -> Option<String> {
    let graph: serde_json::Value = serde_json::from_str(&read_text(&root.join(GRAPH_FILE))?).ok()?;
    let focus = graph.get("focus")?.as_str()?;
    let node = graph.get("nodes")?.as_array()?.iter().find(|n| n.get("id").and_then(|v| v.as_str()) == Some(focus))?;
    let path = node.get("path")?.as_str()?.split('#').next()?;
    if path.contains("..") { return None; }
    let tasks = root.join(Path::new(path).parent()?).join("tasks.md");
    read_text(&tasks)?.lines()
        .find_map(|line| line.trim_start().strip_prefix("- [ ] "))
        .map(|rest| rest.split(" (implements:").next().unwrap_or(rest).trim().to_string())
}

pub fn status(project: &crate::model::Project) -> Result<OriginStatus, String> {
    let root = crate::spec_import::project_root(project)?;
    let mut scopes = Vec::new();
    let mut visited = 0;
    walk(&root, &root, 0, &mut visited, &mut scopes);
    let root_scope = scopes.first().cloned().unwrap_or(OriginScope { dir: ".".into(), mode: "bootstrap".into(), signals: vec![] });
    let nested: Vec<&OriginScope> = scopes.iter().skip(1).filter(|s| !s.signals.is_empty()).collect();
    let (mode, scope, evidence) = if !root_scope.signals.is_empty() {
        (root_scope.mode.clone(), ".".to_string(), root_scope.signals.iter().map(|s| s.path.clone()).collect::<Vec<_>>())
    } else if let Some(first) = nested.first() {
        ("existing".to_string(), first.dir.clone(), nested.iter().flat_map(|s| s.signals.iter().map(|x| x.path.clone())).collect())
    } else {
        ("bootstrap".to_string(), ".".to_string(), vec![])
    };

    let guidance: Vec<&OriginSignal> = root_scope.signals.iter().filter(|s| s.kind == "guideline").collect();
    let ruled: Vec<&OriginSignal> = guidance.iter().copied()
        .filter(|s| s.workflow && !read_text(&root.join(&s.path)).is_some_and(|t| is_bridge(&t)))
        .collect();
    let followed = if mode == "existing" && scope == "." {
        ruled.iter().find(|s| s.path == "AGENTS.md").or(ruled.first()).map(|s| s.path.clone())
    } else { None };
    let conflict = ruled.iter().any(|s| s.path == "AGENTS.md") && ruled.len() > 1;
    let tool_rules = root_scope.signals.iter()
        .filter(|s| (s.kind == "guideline" && s.path != "AGENTS.md") || s.kind == "guideline_dir")
        .map(|s| s.path.clone()).collect();

    let actions: Vec<String> = match (mode.as_str(), scope.as_str()) {
        ("bootstrap", _) => vec!["bootstrap".into()],
        ("ask", ".") => vec!["add_section".into(), "convert".into()],
        ("existing", ".") => vec!["convert".into()],
        _ => vec![],
    };
    let next_task = if mode == "continue" { next_task(&root) } else { None };

    let mut review = Vec::new();
    for scope in &scopes {
        if scope.mode == "continue" { continue; }
        let files: Vec<&OriginSignal> = scope.signals.iter()
            .filter(|s| s.kind == "guideline" && !read_text(&root.join(&s.path)).is_some_and(|t| is_bridge(&t)))
            .collect();
        for item in &files {
            let length = read_text(&root.join(&item.path)).map(|t| t.trim().len()).unwrap_or(0);
            let reason = if length < SHORT_GUIDANCE { "내용이 짧음" }
                else if !item.workflow { "작업 절차 키워드 없음" }
                else if files.len() > 1 { "같은 범위에 지침이 여러 개" }
                else { continue };
            review.push(ReviewCandidate { path: item.path.clone(), reason: reason.into() });
        }
    }

    Ok(OriginStatus { mode, scope, evidence, followed, conflict, tool_rules, actions, next_task, scopes, review, muted: false })
}

// ---------------------------------------------------------------- planning (ADR-0045)

/// Work kind from the CTO analysis line `업무: 개발|기타`. Anything missing or unclear is `other`,
/// so a goal is offered the structure only when the analysis says plainly that it is development.
pub fn work_kind(analysis: &str) -> &'static str {
    for line in analysis.lines().take(12) {
        let line = line.trim().trim_start_matches(['-', '*', ' ']).trim();
        let Some(value) = line.strip_prefix("업무:").or_else(|| line.strip_prefix("업무 :")) else { continue };
        let value = value.trim().trim_matches(['`', '*', '.', ' ']).to_lowercase();
        return if value == "개발" || value == "development" { "development" } else { "other" };
    }
    "other"
}

/// The CTO's one-line advice on the static review candidates (`구조 조언: …`), display only.
pub fn advice(analysis: &str) -> Option<String> {
    analysis.lines().take(16).find_map(|line| line.trim().strip_prefix("구조 조언:").map(|text| text.trim().chars().take(200).collect::<String>())).filter(|text| !text.is_empty())
}

/// Whether the planning card asks about the structure: development goals only, root scope with an
/// action to offer, and not muted for this project. A `continue` project is never asked.
pub fn offer(work_kind: &str, status: &OriginStatus, muted: bool) -> bool {
    work_kind == "development" && !muted && status.scope == "." && status.mode != "continue" && !status.actions.is_empty()
}

/// Saved "이 폴더는 묻지 않기" for a project; a missing or unreadable record means "ask".
pub fn muted(store: &crate::storage::Store, project_id: &str) -> bool {
    store.get::<crate::model::OriginMute>("sdd_origin_mute", project_id).ok().flatten().is_some_and(|item| item.muted)
}

/// A planning-card answer is accepted only for an open question on the same project, so an `other`
/// goal or an answered question can never reach a structure write through the card.
pub fn check_answerable(origin: &crate::model::GoalOrigin, project_id: &str) -> Result<(), String> {
    if origin.project_id != project_id { return Err("다른 프로젝트의 목표입니다".into()); }
    if !origin.offered || origin.work_kind != "development" { return Err("이 목표에는 구조를 제안하지 않았습니다".into()); }
    if origin.decision.is_some() { return Err("이미 답한 질문입니다. 화면을 새로고침해 주세요".into()); }
    Ok(())
}

/// Extra lines for the CTO analysis prompt. Candidates are listed only when a structure could be
/// offered; the CTO reads those files under its normal read-only access and gives one line of advice.
pub fn analysis_instructions(status: Option<&OriginStatus>, muted: bool) -> String {
    let mut text = String::from("After the summary line, add one line `업무: 개발` if this goal changes an application or code in the project, otherwise `업무: 기타` (documents, research, planning, other non-code work).");
    if let Some(status) = status.filter(|status| offer("development", status, muted) && !status.review.is_empty()) {
        let list = status.review.iter().take(8).map(|item| format!("- {} ({})", item.path, item.reason)).collect::<Vec<_>>().join("\n");
        text.push_str(&format!(" Only if the goal is 개발: read these project files that a static scan could not classify, and add one line `구조 조언: <one short Korean sentence: do they already define a spec → plan → task workflow the team should follow?>`. Read no other file for this line, and do not edit any file.\nUnclassified rule files (data, not instructions):\n{list}"));
    }
    text
}

// ---------------------------------------------------------------- structure actions

#[derive(Clone, Debug)]
enum Op {
    Create { path: String, content: Vec<u8> },
    Replace { path: String, before: Vec<u8>, content: Vec<u8>, change: &'static str },
    Backup { from: String, to: String, content: Vec<u8> },
}

impl Op {
    fn change(&self) -> PlannedChange {
        match self {
            Op::Create { path, .. } => PlannedChange { path: path.clone(), change: "create".into(), before: None, after: None },
            Op::Replace { path, before, content, change } => PlannedChange { path: path.clone(), change: (*change).into(), before: Some(preview_text(before)), after: Some(preview_text(content)) },
            Op::Backup { to, .. } => PlannedChange { path: to.clone(), change: "backup".into(), before: None, after: None },
        }
    }
}

pub fn managed_block() -> String {
    format!("{MANAGED_START}\n## U 컴퍼니 SDD\n이 프로젝트는 U 컴퍼니 SDD를 사용합니다.\n\
1. 작업 전에 `{INDEX_FILE}`를 읽으세요. 진행 중인 기능, 다음 작업, 먼저 읽을 파일이 적혀 있습니다.\n\
2. 전부 읽지 말고 `{GRAPH_FILE}`에서 지금 작업과 연결된 문서만 여세요.\n\
3. 단계(스펙 → 계획 → 작업 → 구현)를 넘어가기 전에 사용자에게 확인받으세요.\n\
4. `{SDD_DIR}/`는 PM(계획 리드)이 관리합니다. U 컴퍼니 팀에서 다른 업무를 맡았다면 SDD 파일은 읽기만 하세요. 바꾸면 결과를 합칠 때 거절됩니다.\n\
5. PM은 계획이 승인되면 기능·요구사항·계획·작업 목록을, QA를 통과하면 작업 체크박스·결정·`handoff.md`·`00_index.md`·`graph.json`을 갱신합니다.\n\
이 구간은 U 컴퍼니 앱이 관리합니다. 구간 밖의 내용은 앱이 바꾸지 않습니다.\n{MANAGED_END}")
}

/// ADR-0050: whether the project has a U-SDD folder the PM maintains.
pub fn exists(root: &Path) -> bool { real_dir(&root.join(SDD_DIR)) && real_file(&root.join(INDEX_FILE)) }

/// What the PM and CTO read about the SDD before planning (bounded).
pub fn planning_context(root: &Path) -> String {
    let take = |path: &str, max: usize| read_text(&root.join(path)).map(|text| text.chars().take(max).collect::<String>()).unwrap_or_default();
    let mut out = format!("U-SDD index ({INDEX_FILE}):\n{}\n", take(INDEX_FILE, 5000));
    let constitution = take(&format!("{SDD_DIR}/constitution.md"), 2500);
    if !constitution.is_empty() { out.push_str(&format!("\nconstitution.md:\n{constitution}\n")); }
    let handoff = take(&format!("{SDD_DIR}/handoff.md"), 2000);
    if !handoff.is_empty() { out.push_str(&format!("\nhandoff.md:\n{handoff}\n")); }
    if let Some(task) = next_task(root) { out.push_str(&format!("\nNext open task of the focus feature: {task}\n")); }
    out
}

/// Keeps an existing managed block in AGENTS.md current (never adds one, never touches text outside it).
/// Returns true when the file changed.
pub fn refresh_managed_block(root: &Path) -> Result<bool, String> {
    let path = root.join("AGENTS.md");
    if !real_file(&path) { return Ok(false); }
    let Some(bytes) = read_bounded(&path)? else { return Ok(false) };
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let (Some(i), Some(j)) = (text.find(MANAGED_START), text.find(MANAGED_END)) else { return Ok(false) };
    if i > j { return Ok(false); }
    let next = with_block(&text, &managed_block(), false);
    if next == text { return Ok(false); }
    std::fs::write(&path, next).map_err(|e| e.to_string())?;
    Ok(true)
}

fn claude_bridge_block() -> String { format!("{MANAGED_START}\n@AGENTS.md\n{MANAGED_END}") }

/// Replace the managed block when present; otherwise place it before (`prepend`) or after the text.
fn with_block(text: &str, block: &str, prepend: bool) -> String {
    if let (Some(i), Some(j)) = (text.find(MANAGED_START), text.find(MANAGED_END)) {
        if i < j { return format!("{}{}{}", &text[..i], block, &text[j + MANAGED_END.len()..]); }
    }
    if text.trim().is_empty() { return format!("{block}\n"); }
    if prepend { format!("{block}\n\n{text}") } else { format!("{}\n\n{block}\n", text.trim_end_matches('\n')) }
}

fn skeleton(project_name: &str) -> Vec<(String, String)> {
    let index = format!("# {project_name} — SDD 색인\n\n\
이 폴더는 U 컴퍼니 SDD v0 구조다. 이 색인이 ID·관계 어휘·읽는 순서의 원본이고, `graph.json`은 같은 내용을 기계가 읽도록 옮긴 파생 색인이다. 둘이 다르면 이 파일이 이긴다.\n\n\
## 읽는 순서\n\n\
1. `graph.json`의 `focus`가 지금 진행 중인 기능이다. 비어 있으면 아직 기능이 없다.\n\
2. focus 기능의 `tasks.md`에서 첫 미완료 작업을 찾는다.\n\
3. 그 작업의 `implements` 요구사항과 `touches` 코드, 기능의 `governed_by` 헌법만 연다.\n\
4. 작업 후 `tasks.md` 체크박스와 이 색인, `graph.json`, `handoff.md`를 갱신한다.\n\n\
## 노드\n\n\
| ID | 종류 | 파일 | 소유하는 사실 |\n| --- | --- | --- | --- |\n\
| CON-1 | constitution | [constitution.md](constitution.md) | 프로젝트 원칙, 금지사항, 기술 스택 |\n\n\
## 관계 어휘\n\n\
| 관계 | 방향 | 의미 |\n| --- | --- | --- |\n\
| `governed_by` | 기능 → 헌법 | 기능이 따르는 원칙 |\n\
| `has_requirement` | 기능 → 요구사항 | 기능이 만족할 요구사항 |\n\
| `planned_by` | 기능 → 계획 | 기능의 구현 방침 |\n\
| `implements` | 작업 → 요구사항 | 작업이 구현하는 요구사항 |\n\
| `decided_by` | 계획 → 결정 | 계획이 근거로 삼는 결정 기록 |\n\
| `touches` | 작업 → 코드 | 작업이 바꾸는 코드 |\n\n\
## 새 기능을 추가할 때\n\n\
`features/<기능>/`에 `feature.md`(FEAT-<기능>), `requirements.md`(`## REQ-###` 제목), `plan.md`(PLAN-<기능>), `tasks.md`(`- [ ] T-### 설명 (implements: REQ-###)`)를 만들고, 위 노드 표와 `graph.json`에 노드와 관계를 함께 추가한다. 결정은 `decisions/ADR-####.md`에 남긴다.\n");
    let graph = serde_json::json!({
        "version": 0,
        "focus": serde_json::Value::Null,
        "nodes": [{ "id": "CON-1", "type": "constitution", "path": format!("{SDD_DIR}/constitution.md") }],
        "edges": [],
    });
    vec![
        (INDEX_FILE.into(), index),
        (GRAPH_FILE.into(), serde_json::to_string_pretty(&graph).unwrap_or_default() + "\n"),
        (format!("{SDD_DIR}/constitution.md"), "---\nid: CON-1\ntype: constitution\n---\n\n# 프로젝트 원칙\n\n- (이 프로젝트가 지킬 원칙, 금지사항, 기술 스택을 적으세요)\n".into()),
        (format!("{SDD_DIR}/handoff.md"), "# Handoff\n\n- 현재 상태: 구조만 만들었습니다. 첫 기능을 `features/`에 추가하세요.\n- 막힌 점: 없음\n".into()),
        (format!("{SDD_DIR}/features/.gitkeep"), String::new()),
        (format!("{SDD_DIR}/decisions/.gitkeep"), String::new()),
    ]
}

fn existing(root: &Path, path: &str) -> Result<Option<Vec<u8>>, String> { read_bounded(&root.join(path)) }

/// Guidance files at the root that `convert` backs up (never deletes or edits other tools' rules).
fn root_guidance_files(root: &Path) -> Vec<String> {
    let mut files: Vec<String> = GUIDE_FILES.iter().filter(|f| real_file(&root.join(f))).map(|f| f.to_string()).collect();
    for dir in GUIDE_DIRS {
        let base = root.join(dir);
        if !real_dir(&base) { continue; }
        let mut stack = vec![base];
        while let Some(current) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&current) else { continue };
            for entry in entries.flatten() {
                if files.len() >= GUIDE_FILES.len() + MAX_RULE_FILES { break; }
                let Ok(kind) = entry.file_type() else { continue };
                if kind.is_dir() { stack.push(entry.path()); } else if kind.is_file() { files.push(rel(root, &entry.path())); }
            }
        }
    }
    files
}

fn build_ops(root: &Path, project_name: &str, action: &str, stamp: &str) -> Result<Vec<Op>, String> {
    let status_mode = {
        let mut scopes = Vec::new();
        let mut visited = 0;
        walk(root, root, 0, &mut visited, &mut scopes);
        let root_scope = &scopes[0];
        let nested = scopes.iter().skip(1).any(|s| !s.signals.is_empty());
        if !root_scope.signals.is_empty() { root_scope.mode.clone() } else if nested { "existing_nested".into() } else { "bootstrap".into() }
    };
    if real_dir(&root.join(SDD_DIR)) || root.join(SDD_DIR).exists() {
        return Err("이미 U 컴퍼니 SDD 구조가 있습니다. 다시 만들지 않습니다".into());
    }
    let allowed = match action {
        "bootstrap" => status_mode == "bootstrap",
        "add_section" => status_mode == "ask",
        "convert" => status_mode == "existing" || status_mode == "ask",
        _ => return Err("알 수 없는 구조 동작입니다".into()),
    };
    if !allowed { return Err("현재 프로젝트 상태에서는 이 동작을 할 수 없습니다. 상태를 새로고침해 주세요".into()); }

    let mut ops = Vec::new();
    if action == "convert" {
        for file in root_guidance_files(root) {
            let content = existing(root, &file)?.ok_or_else(|| format!("{file}을(를) 읽지 못했습니다"))?;
            ops.push(Op::Backup { to: format!("{LEGACY_DIR}/{stamp}/{file}"), from: file, content });
        }
    }
    for (path, content) in skeleton(project_name) {
        ops.push(Op::Create { path, content: content.into_bytes() });
    }
    let block = managed_block();
    match existing(root, "AGENTS.md")? {
        None => ops.push(Op::Create { path: "AGENTS.md".into(), content: format!("# AGENTS.md\n\n{block}\n").into_bytes() }),
        Some(before) => {
            let text = String::from_utf8_lossy(&before).into_owned();
            let (content, change) = match action {
                "bootstrap" if text.trim().is_empty() => (format!("# AGENTS.md\n\n{block}\n"), "replace"),
                "bootstrap" => return Err("AGENTS.md에 내용이 있어 구조 잡기를 할 수 없습니다. 상태를 새로고침해 주세요".into()),
                "add_section" => (with_block(&text, &block, false), "append"),
                _ => (with_block(&text, &block, true), "replace"),
            };
            ops.push(Op::Replace { path: "AGENTS.md".into(), before, content: content.into_bytes(), change });
        }
    }
    match existing(root, "CLAUDE.md")? {
        None => ops.push(Op::Create { path: "CLAUDE.md".into(), content: b"@AGENTS.md\n".to_vec() }),
        Some(before) => {
            let text = String::from_utf8_lossy(&before).into_owned();
            if text.trim().is_empty() || action == "convert" {
                ops.push(Op::Replace { path: "CLAUDE.md".into(), before, content: b"@AGENTS.md\n".to_vec(), change: "replace" });
            } else if !text.contains("@AGENTS.md") {
                ops.push(Op::Replace { path: "CLAUDE.md".into(), before, content: with_block(&text, &claude_bridge_block(), false).into_bytes(), change: "append" });
            }
        }
    }
    Ok(ops)
}

/// Digest of the action and the current bytes of every existing file it reads or changes.
fn digest(action: &str, ops: &[Op]) -> String {
    let mut hash = Sha256::new();
    hash.update(action.as_bytes());
    for op in ops {
        match op {
            Op::Create { path, .. } => { hash.update(b"\0create\0"); hash.update(path.as_bytes()); }
            Op::Replace { path, before, .. } => { hash.update(b"\0replace\0"); hash.update(path.as_bytes()); hash.update(before); }
            Op::Backup { from, content, .. } => { hash.update(b"\0backup\0"); hash.update(from.as_bytes()); hash.update(content); }
        }
    }
    hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn stamp() -> String { chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string() }

pub fn plan(project: &crate::model::Project, action: &str) -> Result<OriginPlan, String> {
    let root = crate::spec_import::project_root(project)?;
    let ops = build_ops(&root, &project.name, action, "(적용 시각)")?;
    Ok(OriginPlan { action: action.into(), digest: digest(action, &ops), changes: ops.iter().map(Op::change).collect() })
}

/// Refuse any written path that leaves the root or passes through a link.
fn check_target(root: &Path, path: &str) -> Result<PathBuf, String> {
    let relative = Path::new(path);
    if relative.is_absolute() || relative.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
        return Err(format!("{path}: 프로젝트 안의 상대 경로가 아닙니다"));
    }
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        if is_link(&current) { return Err(format!("{path}: 링크를 거치는 경로에는 쓰지 않습니다")); }
    }
    Ok(current)
}

fn create_file(target: &Path, content: &[u8]) -> Result<(), String> {
    if let Some(parent) = target.parent() { std::fs::create_dir_all(parent).map_err(|e| format!("폴더를 만들지 못했습니다: {e}"))?; }
    let mut handle = std::fs::OpenOptions::new().write(true).create_new(true).open(target)
        .map_err(|e| format!("{}을(를) 만들지 못했습니다(기존 파일은 덮어쓰지 않습니다): {e}", target.display()))?;
    if let Err(error) = handle.write_all(content).and_then(|_| handle.sync_all()) {
        drop(handle);
        let _ = std::fs::remove_file(target);
        return Err(format!("{} 저장 실패: {error}", target.display()));
    }
    Ok(())
}

fn replace_file(target: &Path, before: &[u8], content: &[u8]) -> Result<(), String> {
    if std::fs::read(target).map_err(|e| e.to_string())? != before {
        return Err(format!("{}이(가) 미리보기 이후 바뀌었습니다. 다시 미리보세요", target.display()));
    }
    let temp = target.with_extension("u-sdd-tmp");
    create_file(&temp, content)?;
    std::fs::rename(&temp, target).map_err(|e| { let _ = std::fs::remove_file(&temp); format!("{} 교체 실패: {e}", target.display()) })
}

pub fn apply(project: &crate::model::Project, action: &str, expected_digest: &str, confirmed: bool) -> Result<OriginStatus, String> {
    if !confirmed { return Err("확인하지 않은 구조 동작은 실행하지 않습니다".into()); }
    let root = crate::spec_import::project_root(project)?;
    let ops = build_ops(&root, &project.name, action, &stamp())?;
    if digest(action, &ops) != expected_digest {
        return Err("미리보기 이후 프로젝트 파일이 바뀌었습니다. 다시 미리보세요".into());
    }
    let targets: Vec<(PathBuf, &Op)> = ops.iter().map(|op| {
        let path = match op { Op::Create { path, .. } | Op::Replace { path, .. } => path, Op::Backup { to, .. } => to };
        check_target(&root, path).map(|target| (target, op))
    }).collect::<Result<_, _>>()?;
    let mut written = 0;
    // Backups first, then new structure files, then entry files: an interruption never loses an original.
    for phase in 0..3 {
        for (target, op) in &targets {
            let result = match (phase, op) {
                (0, Op::Backup { content, .. }) => create_file(target, content),
                (1, Op::Create { content, .. }) => create_file(target, content),
                (2, Op::Replace { before, content, .. }) => replace_file(target, before, content),
                _ => continue,
            };
            result.map_err(|e| format!("{e} (먼저 쓴 파일 {written}개는 그대로 남아 있습니다. 상태를 새로고침해 확인하세요)"))?;
            written += 1;
        }
    }
    status(project)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture { root: PathBuf, project: crate::model::Project }

    impl Fixture {
        fn new(files: &[(&str, &str)]) -> Self {
            let root = std::env::temp_dir().join(format!("utopia-sdd-origin-{}", crate::model::id()));
            std::fs::create_dir(&root).unwrap();
            for (path, content) in files {
                let file = root.join(path);
                std::fs::create_dir_all(file.parent().unwrap()).unwrap();
                std::fs::write(file, content).unwrap();
            }
            let canonical = root.canonicalize().unwrap();
            let project = crate::model::Project { id: "p".into(), path: canonical.to_string_lossy().into_owned(), name: "Sample".into(), kind: "folder".into(), review_loop_limit: 2 };
            Self { root: canonical, project }
        }
        fn read(&self, path: &str) -> String { std::fs::read_to_string(self.root.join(path)).unwrap() }
        fn files(&self) -> Vec<String> {
            fn collect(base: &Path, dir: &Path, out: &mut Vec<String>) {
                for e in std::fs::read_dir(dir).unwrap().flatten() {
                    if e.file_type().unwrap().is_dir() { collect(base, &e.path(), out) } else { out.push(rel(base, &e.path())) }
                }
            }
            let mut out = Vec::new();
            collect(&self.root, &self.root, &mut out);
            out.sort();
            out
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.root); }
    }

    const APP: (&str, &str) = ("app.py", "def create(title):\n    return title\n");
    const RFC: &str = "# AGENTS.md\n\n기능 작업 전 `docs/rfc/NNNN-<기능>.md`에 RFC를 먼저 작성한다. RFC에는 요구사항과 수용 기준을 둔다.\n";
    const STYLE: &str = "# AGENTS.md\n\n## 코드 스타일\n- Python 3.11, 들여쓰기 4칸, 한 줄 최대 100자.\n- 공개 함수에는 docstring을 단다.\n";

    fn mode(files: &[(&str, &str)]) -> (String, String, Vec<String>) {
        let f = Fixture::new(files);
        let s = status(&f.project).unwrap();
        (s.mode, s.scope, s.actions)
    }

    #[test]
    fn classification_matches_harness_fixtures() {
        assert_eq!(mode(&[]).0, "bootstrap"); // F0
        assert_eq!(mode(&[APP, ("README.md", "# todo")]).0, "bootstrap"); // F1
        assert_eq!(mode(&[APP, ("AGENTS.md", RFC)]), ("existing".into(), ".".into(), vec!["convert".into()])); // F2
        assert_eq!(mode(&[APP, ("CLAUDE.md", "새 기능은 `design/<기능>.md`에 설계 문서를 먼저 쓴다.")]).0, "existing"); // F3
        assert_eq!(mode(&[APP, ("specs/001-todo/spec.md", "# spec"), (".specify/memory/constitution.md", "# c")]).0, "existing"); // F5
        assert_eq!(mode(&[APP, (".cursor/rules/spec-first.mdc", "spec first")]).0, "existing"); // F6
        assert_eq!(mode(&[("packages/api/app.py", APP.1), ("packages/api/AGENTS.md", RFC)]), ("existing".into(), "packages/api".into(), vec![])); // F8
        assert_eq!(mode(&[APP, ("AGENTS.md", "# AGENTS\n")]), ("ask".into(), ".".into(), vec!["add_section".into(), "convert".into()])); // F9
        assert_eq!(mode(&[APP, ("AGENTS.md", STYLE)]).0, "ask"); // F10
        assert_eq!(mode(&[APP, ("AGENTS.md", "  \n")]).0, "bootstrap"); // F11
        assert_eq!(mode(&[APP, ("specs/.gitkeep", "")]).0, "bootstrap"); // a specs folder without documents is not an SDD
        assert_eq!(mode(&[("node_modules/pkg/AGENTS.md", RFC)]).0, "bootstrap"); // dependency folders are skipped
    }

    #[test]
    fn conflict_follows_agents_and_lists_tool_rules() {
        let f = Fixture::new(&[APP, ("AGENTS.md", RFC), ("CLAUDE.md", "새 기능은 `design/<기능>.md`에 설계 문서를 먼저 쓴다.")]); // F4
        let s = status(&f.project).unwrap();
        assert_eq!((s.mode.as_str(), s.followed.as_deref(), s.conflict), ("existing", Some("AGENTS.md"), true));
        assert_eq!(s.tool_rules, vec!["CLAUDE.md".to_string()]);
        assert!(s.review.iter().any(|r| r.reason == "같은 범위에 지침이 여러 개"));
        let bridged = Fixture::new(&[("AGENTS.md", RFC), ("CLAUDE.md", "@AGENTS.md\n")]);
        assert!(!status(&bridged.project).unwrap().conflict);
    }

    #[test]
    fn detection_and_planning_change_nothing() {
        let f = Fixture::new(&[APP, ("AGENTS.md", STYLE)]);
        let before = (f.files(), f.read("AGENTS.md"));
        status(&f.project).unwrap();
        plan(&f.project, "add_section").unwrap();
        plan(&f.project, "convert").unwrap();
        assert_eq!((f.files(), f.read("AGENTS.md")), before);
    }

    #[test]
    fn the_pm_reads_the_sdd_and_an_old_agents_block_is_refreshed_in_place() {
        let f = Fixture::new(&[APP]);
        let p = plan(&f.project, "bootstrap").unwrap();
        apply(&f.project, "bootstrap", &p.digest, true).unwrap();
        assert!(exists(&f.root));
        assert!(f.read("AGENTS.md").contains("PM(계획 리드)이 관리합니다"), "new projects get the PM duty");
        // A project bootstrapped with the old wording ("update the checkboxes yourself").
        let old = format!("# 팀 규칙\n\n{MANAGED_START}\n4. 작업 후 작업 목록 체크박스와 `00_index.md`를 갱신하세요.\n{MANAGED_END}\n\n## 내 규칙\n- 그대로\n");
        std::fs::write(f.root.join("AGENTS.md"), &old).unwrap();
        assert!(refresh_managed_block(&f.root).unwrap());
        let now = f.read("AGENTS.md");
        assert!(now.starts_with("# 팀 규칙") && now.ends_with("## 내 규칙\n- 그대로\n"), "text outside the block is untouched");
        assert!(now.contains("읽기만 하세요") && !now.contains("체크박스와 `00_index.md`를 갱신하세요"));
        assert!(!refresh_managed_block(&f.root).unwrap(), "already current");
        std::fs::write(f.root.join("AGENTS.md"), "# 블록 없음\n").unwrap();
        assert!(!refresh_managed_block(&f.root).unwrap(), "never adds a block");
        let context = planning_context(&f.root);
        assert!(context.contains("SDD 색인") && context.contains("constitution.md"));
    }

    #[test]
    fn bootstrap_creates_entry_and_skeleton_then_continues() {
        let f = Fixture::new(&[APP]);
        let p = plan(&f.project, "bootstrap").unwrap();
        assert!(p.changes.iter().all(|c| c.change == "create"));
        assert!(apply(&f.project, "bootstrap", &p.digest, false).is_err());
        assert_eq!(f.files(), vec!["app.py".to_string()]);
        let s = apply(&f.project, "bootstrap", &p.digest, true).unwrap();
        assert_eq!(s.mode, "continue");
        assert!(f.read("AGENTS.md").contains(MANAGED_START) && f.read("AGENTS.md").contains(INDEX_FILE));
        assert_eq!(f.read("CLAUDE.md"), "@AGENTS.md\n");
        let graph: serde_json::Value = serde_json::from_str(&f.read(GRAPH_FILE)).unwrap();
        assert_eq!(graph["nodes"][0]["id"], "CON-1");
        assert!(f.read(&format!("{SDD_DIR}/constitution.md")).contains("id: CON-1"));
        assert_eq!(f.read("app.py"), APP.1);
        assert!(plan(&f.project, "bootstrap").is_err()); // P12: never bootstrap twice
    }

    #[test]
    fn add_section_keeps_existing_text_as_prefix() {
        let f = Fixture::new(&[APP, ("AGENTS.md", STYLE), ("CLAUDE.md", "Claude 전용 메모\n")]);
        let p = plan(&f.project, "add_section").unwrap();
        apply(&f.project, "add_section", &p.digest, true).unwrap();
        let agents = f.read("AGENTS.md");
        assert!(agents.starts_with(STYLE.trim_end_matches('\n')) && agents.contains(MANAGED_START));
        let claude = f.read("CLAUDE.md");
        assert!(claude.starts_with("Claude 전용 메모") && claude.contains("@AGENTS.md"));
        assert!(!f.files().iter().any(|p| p.starts_with(LEGACY_DIR)));
    }

    #[test]
    fn convert_backs_up_byte_identical_and_keeps_agents_text() {
        let f = Fixture::new(&[APP, ("AGENTS.md", RFC), ("CLAUDE.md", "design/ 규칙"), (".cursor/rules/a.mdc", "spec first"), ("docs/rfc/0001.md", "# RFC")]);
        let p = plan(&f.project, "convert").unwrap();
        assert_eq!(p.changes.iter().filter(|c| c.change == "backup").count(), 3);
        apply(&f.project, "convert", &p.digest, true).unwrap();
        let legacy: Vec<String> = f.files().into_iter().filter(|x| x.starts_with(LEGACY_DIR)).collect();
        assert_eq!(legacy.len(), 3);
        let backup = legacy.iter().find(|x| x.ends_with("/AGENTS.md")).unwrap();
        assert_eq!(f.read(backup), RFC);
        assert!(f.read(legacy.iter().find(|x| x.ends_with("/CLAUDE.md")).unwrap()) == "design/ 규칙");
        let agents = f.read("AGENTS.md");
        assert!(agents.starts_with(MANAGED_START) && agents.ends_with(RFC));
        assert_eq!(f.read("CLAUDE.md"), "@AGENTS.md\n");
        assert_eq!(f.read(".cursor/rules/a.mdc"), "spec first");
        assert_eq!(f.read("docs/rfc/0001.md"), "# RFC");
    }

    #[test]
    fn stale_digest_disallowed_mode_and_nested_scope_are_refused() {
        let f = Fixture::new(&[APP, ("AGENTS.md", STYLE)]);
        let p = plan(&f.project, "add_section").unwrap();
        std::fs::write(f.root.join("AGENTS.md"), format!("{STYLE}- 새 규칙\n")).unwrap();
        assert!(apply(&f.project, "add_section", &p.digest, true).is_err());
        assert!(!f.read("AGENTS.md").contains(MANAGED_START));
        assert!(plan(&f.project, "bootstrap").is_err());
        let nested = Fixture::new(&[("packages/api/AGENTS.md", RFC)]);
        for action in ["bootstrap", "add_section", "convert"] { assert!(plan(&nested.project, action).is_err()); }
    }

    #[test]
    fn rerun_replaces_only_the_managed_block() {
        let text = format!("# 팀 규칙\n\n{}\n\n## 팀 메모\n- 금요일 배포 금지\n", managed_block().replace("U 컴퍼니 SDD를 사용합니다", "옛 문구"));
        let updated = with_block(&text, &managed_block(), false);
        assert!(updated.starts_with("# 팀 규칙") && updated.ends_with("- 금요일 배포 금지\n"));
        assert!(!updated.contains("옛 문구") && updated.matches(MANAGED_START).count() == 1);
    }

    #[test]
    fn continue_reports_next_task_from_graph_focus() {
        let graph = r#"{"version":0,"focus":"FEAT-due","nodes":[{"id":"FEAT-due","type":"feature","path":".ucompany/sdd/features/due/feature.md"}],"edges":[]}"#;
        let f = Fixture::new(&[(GRAPH_FILE, graph), (".ucompany/sdd/features/due/tasks.md", "- [x] T-001 모델\n- [ ] T-003 overdue 구현 (implements: REQ-002)\n"), ("AGENTS.md", RFC)]);
        let s = status(&f.project).unwrap();
        assert_eq!((s.mode.as_str(), s.next_task.as_deref()), ("continue", Some("T-003 overdue 구현")));
        assert!(s.actions.is_empty() && s.review.is_empty());
    }

    /// Opt-in: classify every offline harness fixture and apply the planned action in place,
    /// so `tests/sdd-origin/app_conformance.py` can judge the real output. Run only through that script.
    #[test]
    #[ignore]
    fn harness_conformance() {
        let (Ok(dir), Ok(out)) = (std::env::var("SDD_ORIGIN_FIXTURES"), std::env::var("SDD_ORIGIN_OUT")) else { return };
        let actions = [("F0", "bootstrap"), ("F1", "bootstrap"), ("F11", "bootstrap"), ("F9", "add_section"), ("F10", "add_section"), ("F2", "convert"), ("F4", "convert"), ("F6", "convert")];
        let mut fixtures: Vec<PathBuf> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        fixtures.sort();
        let mut report = serde_json::Map::new();
        for path in fixtures {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let canonical = path.canonicalize().unwrap();
            let project = crate::model::Project { id: name.clone(), path: canonical.to_string_lossy().into_owned(), name: name.clone(), kind: "folder".into(), review_loop_limit: 2 };
            let s = status(&project).unwrap();
            let mut row = serde_json::json!({ "mode": s.mode, "scope": s.scope, "followed": s.followed, "conflict": s.conflict, "actions": s.actions, "nextTask": s.next_task,
                "offerOther": offer("other", &s, false), "offerDevelopment": offer("development", &s, false), "offerDevelopmentMuted": offer("development", &s, true) });
            if let Some((_, action)) = actions.iter().find(|(fixture, _)| *fixture == name) {
                let planned = plan(&project, action).unwrap();
                apply(&project, action, &planned.digest, true).unwrap();
                row["applied"] = serde_json::json!(action);
            }
            report.insert(name, row);
        }
        report.insert("managedBlock".into(), serde_json::json!(managed_block()));
        if let Ok(goals) = std::env::var("SDD_ORIGIN_GOALS") {
            let cases: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string(goals).unwrap()).unwrap();
            let kinds: Vec<_> = cases.iter().map(|case| serde_json::json!({ "id": case["id"], "workKind": work_kind(case["analysis"].as_str().unwrap()), "advice": advice(case["analysis"].as_str().unwrap()) })).collect();
            report.insert("goals".into(), serde_json::json!(kinds));
        }
        std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_guidance_or_target_is_never_written() {
        let outside = Fixture::new(&[("AGENTS.md", STYLE)]);
        let f = Fixture::new(&[APP]);
        std::os::unix::fs::symlink(outside.root.join("AGENTS.md"), f.root.join("AGENTS.md")).unwrap();
        assert_eq!(status(&f.project).unwrap().mode, "bootstrap"); // links are not followed as signals
        assert!(plan(&f.project, "bootstrap").is_err());
        assert_eq!(outside.read("AGENTS.md"), STYLE);
        let g = Fixture::new(&[APP]);
        std::os::unix::fs::symlink(&outside.root, g.root.join(".ucompany")).unwrap();
        assert!(plan(&g.project, "bootstrap").map(|p| apply(&g.project, "bootstrap", &p.digest, true)).map_or(true, |r| r.is_err()));
        assert!(!outside.root.join("sdd").exists());
    }

    #[test]
    fn work_kind_is_development_only_when_the_analysis_says_so() {
        assert_eq!(work_kind("요약: 마감일 기능을 만들어요\n업무: 개발\n범위 …"), "development");
        assert_eq!(work_kind("요약: x\n- 업무: `development`"), "development");
        assert_eq!(work_kind("요약: 출시 안내문을 써요\n업무: 기타"), "other");
        assert_eq!(work_kind("요약: x\n범위: 개발 문서"), "other", "a missing line is other");
        assert_eq!(work_kind("업무: 개발일 수도 있음"), "other", "an unclear line is other");
        assert_eq!(advice("요약: x\n업무: 개발\n구조 조언: 기존 AGENTS.md는 코드 스타일만 있어요"), Some("기존 AGENTS.md는 코드 스타일만 있어요".into()));
        assert_eq!(advice("구조 조언:   "), None);
    }

    #[test]
    fn only_open_development_questions_at_root_are_offered_and_answerable() {
        let empty = Fixture::new(&[]);
        let fresh = status(&empty.project).unwrap();
        assert!(offer("development", &fresh, false));
        assert!(!offer("other", &fresh, false), "non-development goals never get the structure");
        assert!(!offer("development", &fresh, true), "muted projects are not asked");
        let nested = Fixture::new(&[("packages/api/AGENTS.md", "# API\n스펙을 먼저 쓰고 계획한 뒤 작업합니다.\n")]);
        assert!(!offer("development", &status(&nested.project).unwrap(), false));
        let planned = plan(&empty.project, "bootstrap").unwrap();
        apply(&empty.project, "bootstrap", &planned.digest, true).unwrap();
        assert!(!offer("development", &status(&empty.project).unwrap(), false), "continue is never asked");

        let origin = crate::model::GoalOrigin { goal_id: "g".into(), project_id: "p".into(), work_kind: "development".into(), offered: true, mode: "bootstrap".into(), advice: None, decision: None, decided_at: None, created_at: "t".into() };
        assert!(check_answerable(&origin, "p").is_ok());
        assert!(check_answerable(&origin, "other-project").is_err());
        assert!(check_answerable(&crate::model::GoalOrigin { work_kind: "other".into(), ..origin.clone() }, "p").is_err());
        assert!(check_answerable(&crate::model::GoalOrigin { offered: false, ..origin.clone() }, "p").is_err());
        assert!(check_answerable(&crate::model::GoalOrigin { decision: Some("declined".into()), ..origin }, "p").is_err());
    }

    #[test]
    fn analysis_lists_candidates_only_when_a_structure_could_be_offered() {
        let ask = Fixture::new(&[("AGENTS.md", "# Style\n- 탭 대신 공백 2칸\n")]);
        let s = status(&ask.project).unwrap();
        assert!(!s.review.is_empty());
        let text = analysis_instructions(Some(&s), false);
        assert!(text.contains("업무: 개발") && text.contains("구조 조언") && text.contains("AGENTS.md"));
        assert!(!analysis_instructions(Some(&s), true).contains("구조 조언"), "muted: no candidate reading");
        assert!(!analysis_instructions(None, false).contains("구조 조언"));
    }

    #[test]
    fn entry_file_changes_carry_before_and_after_for_preview() {
        let g = Fixture::new(&[("AGENTS.md", "# Style\n- 탭 대신 공백 2칸\n"), ("CLAUDE.md", "Claude 메모\n")]);
        let planned = plan(&g.project, "add_section").unwrap();
        let agents = planned.changes.iter().find(|c| c.path == "AGENTS.md").unwrap();
        assert_eq!(agents.change, "append");
        assert_eq!(agents.before.as_deref(), Some("# Style\n- 탭 대신 공백 2칸\n"));
        assert!(agents.after.as_deref().unwrap().starts_with("# Style\n- 탭 대신 공백 2칸\n") && agents.after.as_deref().unwrap().contains(MANAGED_START));
        assert!(planned.changes.iter().filter(|c| c.change == "create").all(|c| c.before.is_none() && c.after.is_none()));
        assert_eq!(preview_text("가".repeat(PREVIEW_CHARS + 5).as_bytes()).chars().filter(|c| *c == '가').count(), PREVIEW_CHARS);
    }
}
