//! Deliverable task contract, checked single assignment, transactional claim and per-deliverable
//! review (ADR-0038). The engine (`engine.rs`) orchestrates; this module holds the rules so they can
//! be tested without Tauri.

use crate::model::{Agent, AiConnection, Approval, CriterionResult, Goal, Meeting, MeetingPacket, Task};
use crate::storage::{tx_get, tx_list, tx_put, Store};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

pub const CAPABILITIES: [&str; 2] = ["document", "code"];
pub const MAX_CRITERIA: usize = 8;
pub const MAX_EVIDENCE: usize = 8;
pub const CRITERION_CHARS: usize = 300;

/// Contract fields of one proposed task as they come from the PM plan.
pub struct ContractInput<'a> {
    pub title: &'a str,
    pub deliverable: &'a str,
    pub acceptance: &'a [String],
    pub evidence: &'a [String],
    pub capability: &'a str,
}

// ----------------------------- block kinds -----------------------------

const MARK_OPEN: &str = "⟦";
const MARK_CLOSE: &str = "⟧ ";
/// `plan_form` (ADR-0048): the planner's plan failed the app's format checks twice in a row.
pub const BLOCK_KINDS: [&str; 10] = ["precall", "approval_denied", "interrupted", "integration_conflict", "qa_failed", "needs_scope", "qa_form", "plan_form", "provider", "other"];

/// Tags an engine-generated error with its block kind. Only engine code calls this; model text is
/// never parsed for a kind because provider errors are passed through `provider` sites.
pub fn mark(kind: &str, message: impl AsRef<str>) -> String {
    let message = message.as_ref();
    if message.starts_with(MARK_OPEN) { return message.to_string(); }
    format!("{MARK_OPEN}{kind}{MARK_CLOSE}{message}")
}

/// Splits a tagged error into (kind, message). Untagged errors are `other`.
pub fn split_mark(error: &str) -> (String, String) {
    if let Some(rest) = error.strip_prefix(MARK_OPEN) {
        if let Some((kind, message)) = rest.split_once(MARK_CLOSE) {
            if BLOCK_KINDS.contains(&kind) { return (kind.to_string(), message.to_string()); }
        }
    }
    ("other".to_string(), error.to_string())
}

// ----------------------------- contract -----------------------------

fn safe_relative(path: &str) -> bool {
    !path.is_empty() && !path.contains('\\') && !path.contains(':') && path.split('/').all(|part| !part.is_empty() && part != "." && part != ".." && part != ".git")
}

/// Rejects a plan before any approval when a task lacks criteria, names an unknown capability, or
/// cites evidence that is not an existing regular file inside the project.
pub fn validate_contract(tasks: &[ContractInput<'_>], root: &Path) -> Result<(), String> {
    let canonical_root = root.canonicalize().map_err(|e| format!("프로젝트 폴더를 확인할 수 없습니다: {e}"))?;
    for task in tasks {
        let name = if task.deliverable.trim().is_empty() { task.title } else { task.deliverable };
        if task.deliverable.trim().is_empty() { return Err(format!("업무 '{}'에 산출물 설명이 없습니다", task.title)); }
        let criteria: Vec<&String> = task.acceptance.iter().filter(|item| !item.trim().is_empty()).collect();
        if criteria.is_empty() { return Err(format!("산출물 '{name}'에 수용 기준이 없습니다")); }
        if criteria.len() > MAX_CRITERIA { return Err(format!("산출물 '{name}'의 수용 기준은 {MAX_CRITERIA}개 이하여야 합니다")); }
        if criteria.iter().any(|item| item.chars().count() > CRITERION_CHARS) { return Err(format!("산출물 '{name}'의 수용 기준 하나가 {CRITERION_CHARS}자를 넘습니다")); }
        if !CAPABILITIES.contains(&task.capability.trim()) { return Err(format!("산출물 '{name}'의 필요한 능력 '{}'은 document 또는 code여야 합니다", task.capability)); }
        if task.evidence.len() > MAX_EVIDENCE { return Err(format!("산출물 '{name}'의 근거 자료는 {MAX_EVIDENCE}개 이하여야 합니다")); }
        for path in task.evidence {
            if !safe_relative(path) { return Err(format!("근거 경로 '{path}'는 프로젝트 안 상대 경로여야 합니다")); }
            let resolved = root.join(path);
            let metadata = std::fs::symlink_metadata(&resolved).map_err(|_| format!("근거 자료 '{path}'를 프로젝트에서 찾을 수 없습니다"))?;
            if !metadata.is_file() { return Err(format!("근거 자료 '{path}'는 일반 파일이어야 합니다")); }
            let canonical = resolved.canonicalize().map_err(|e| e.to_string())?;
            if !canonical.starts_with(&canonical_root) { return Err(format!("근거 자료 '{path}'가 프로젝트 밖을 가리킵니다")); }
        }
    }
    Ok(())
}

// ----------------------------- assignment -----------------------------

/// A saved connection the engine may use at all (AI-03).
pub fn usable(connection: &AiConnection) -> Result<(), String> {
    if ["missing", "failed", "needs_setup"].contains(&connection.status.as_str()) {
        return Err(format!("AI 연결 '{}' 상태가 {}입니다", connection.name, connection.status));
    }
    Ok(())
}

/// Whether a connection can take file-writing work in an isolated workspace (AI-03).
pub fn can_write_files(connection: &AiConnection) -> Result<(), String> {
    match connection.kind.as_str() {
        "cli" => Ok(()),
        "plugin" if connection.capabilities.iter().any(|item| item == "files") => Ok(()),
        "plugin" => Err(format!("외부 어댑터 '{}'가 파일 작업 능력을 선언하지 않았습니다", connection.name)),
        "http" => Err(format!("HTTP 모델 '{}'은 텍스트만 다룰 수 있어 파일 작업을 맡을 수 없습니다", connection.name)),
        other => Err(format!("연결 종류 '{other}'는 파일 작업을 맡을 수 없습니다")),
    }
}

fn is_implementer(agent: &Agent) -> bool {
    agent.enabled && !["cto", "pm", "qa"].contains(&agent.id.as_str()) && agent.role.to_ascii_lowercase().contains("implement")
}

/// Why `agent` cannot take `capability` work right now, or `None` when eligible.
fn ineligible(agent: &Agent, connections: &[AiConnection], busy: &BTreeMap<String, String>) -> Option<String> {
    if !agent.enabled { return Some("숨긴 팀원입니다".into()); }
    let Some(connection) = connections.iter().find(|item| item.id == agent.connection_id) else { return Some("AI 연결이 배정되지 않았습니다".into()) };
    if let Err(error) = usable(connection) { return Some(error); }
    if let Err(error) = can_write_files(connection) { return Some(error); }
    busy.get(&agent.id).cloned()
}

/// Picks one implementer per task: eligible (enabled, usable file-capable connection, not busy),
/// fewest tasks already assigned in this plan, then roster order. Returns `(agent_id, reason)` per
/// task or, when any task has no candidate, the per-task reasons.
pub fn assign(capabilities: &[&str], agents: &[Agent], connections: &[AiConnection], busy: &BTreeMap<String, String>) -> Result<Vec<(String, String)>, String> {
    // External staff join only through a CEO swap on the approval card (ADR-0041).
    let regular: Vec<Agent> = agents.iter().filter(|agent| !agent.is_external()).cloned().collect();
    assign_within(capabilities, &regular, connections, busy)
}

/// `assign` over an explicit pool, such as a locked squad's writers, which may include contracted
/// external staff; those take only the capabilities their character file declared.
pub fn assign_within(capabilities: &[&str], agents: &[Agent], connections: &[AiConnection], busy: &BTreeMap<String, String>) -> Result<Vec<(String, String)>, String> {
    let implementers: Vec<&Agent> = agents.iter().filter(|agent| agent.role.to_ascii_lowercase().contains("implement") && !["cto", "pm", "qa"].contains(&agent.id.as_str())).collect();
    if implementers.is_empty() { return Err("구현 역할(implementation)의 팀원이 없습니다. 설정 > 팀에서 개발 팀원을 추가하세요".into()); }
    let mut eligible = Vec::new();
    let mut skipped = Vec::new();
    for agent in &implementers {
        match ineligible(agent, connections, busy) { None => eligible.push(*agent), Some(reason) => skipped.push(format!("{}: {reason}", agent.name)) }
    }
    if eligible.is_empty() {
        return Err(format!("업무를 맡을 수 있는 개발 팀원이 없습니다 — {}. AI 연결을 고치거나 진행 중인 일이 끝난 뒤 다시 시도하세요", skipped.join(" / ")));
    }
    let mut load: HashMap<String, usize> = HashMap::new();
    let mut result = Vec::new();
    for requested in capabilities {
        // ADR-0049: `document+images` / `code+images` marks a task whose evidence includes an image;
        // only a teammate whose connection can look at images may take it.
        let (capability, needs_images) = match requested.strip_suffix("+images") { Some(base) => (base, true), None => (*requested, false) };
        let declared = |agent: &&&Agent| agent.external.as_ref().filter(|_| agent.is_external()).map_or(true, |contract| contract.capabilities.iter().any(|item| item == capability));
        let sees_images = |agent: &&&Agent| !needs_images || connections.iter().find(|item| item.id == agent.connection_id).is_some_and(crate::curation::can_view_images);
        if needs_images && !eligible.iter().filter(declared).any(|agent| sees_images(&agent)) {
            return Err("이미지 자료를 볼 수 있는 개발 팀원이 없습니다. Claude 연결을 배정하거나, 이미지를 볼 수 있는 연결이면 연결 설정에서 '이미지 가능'을 켜 주세요".into());
        }
        let chosen = eligible.iter().filter(declared).filter(sees_images).min_by_key(|agent| (load.get(&agent.id).copied().unwrap_or(0), implementers.iter().position(|item| item.id == agent.id).unwrap_or(usize::MAX))).ok_or_else(|| format!("{capability} 업무를 맡을 수 있는 작성자가 스쿼드에 없습니다"))?;
        let count = load.entry(chosen.id.clone()).or_insert(0);
        *count += 1;
        let connection = connections.iter().find(|item| item.id == chosen.connection_id).map(|item| item.name.clone()).unwrap_or_default();
        let mut reason = format!("{capability} 작업 가능{} · {connection} 연결 사용 가능 · 다른 목표·회의에 묶여 있지 않음 · 이 계획에서 {}번째 배정", if needs_images { " · 이미지 확인 가능" } else { "" }, *count);
        if !skipped.is_empty() { reason.push_str(&format!(" · 제외: {}", skipped.join(", "))); }
        result.push((chosen.id.clone(), reason));
    }
    Ok(result)
}

/// The shared DM-01 busy rule, ignoring the goal being planned or executed itself.
pub fn busy_excluding_goal(tx: &rusqlite::Transaction<'_>, goal_id: &str) -> Result<BTreeMap<String, String>, String> {
    let agents: Vec<Agent> = tx_list(tx, "agent")?;
    let goals: Vec<Goal> = tx_list::<Goal>(tx, "goal")?.into_iter().filter(|goal| goal.id != goal_id).collect();
    let tasks: Vec<Task> = tx_list::<Task>(tx, "task")?.into_iter().filter(|task| task.goal_id != goal_id).collect();
    let meetings: Vec<Meeting> = tx_list(tx, "meeting")?;
    let packets: Vec<MeetingPacket> = tx_list(tx, "meeting_packet")?;
    let squads: Vec<crate::model::Squad> = tx_list(tx, "squad")?;
    Ok(crate::meeting::busy_reasons(&agents, &goals, &tasks, &squads, &meetings, &packets, None))
}

pub fn busy_for_goal(store: &Store, goal_id: &str) -> Result<BTreeMap<String, String>, String> {
    store.transact(|tx| busy_excluding_goal(tx, goal_id))
}

// ----------------------------- claim -----------------------------

/// Claims every task of one wave in a single transaction (all or none). Each claim re-checks the
/// task, its dependencies, the goal's project-write approval, and the assignee's state, connection
/// and busy status. Nothing is called before this commits.
pub fn claim_wave(store: &Store, goal_id: &str, task_ids: &[String]) -> Result<Vec<(Task, Agent, AiConnection)>, String> {
    store.transact(|tx| {
        let approvals: Vec<Approval> = tx_list(tx, "approval")?;
        if !approvals.iter().any(|item| item.goal_id.as_deref() == Some(goal_id) && item.operation == "project_write" && item.status == "approved") {
            return Err(mark("precall", "프로젝트 변경 승인이 유효하지 않아 업무를 시작하지 않았습니다"));
        }
        let tasks: Vec<Task> = tx_list::<Task>(tx, "task")?.into_iter().filter(|task| task.goal_id == goal_id).collect();
        let agents: Vec<Agent> = tx_list(tx, "agent")?;
        let connections: Vec<AiConnection> = tx_list(tx, "connection")?;
        let busy = busy_excluding_goal(tx, goal_id)?;
        let squad: Option<crate::model::Squad> = tx_get(tx, "squad", goal_id)?;
        let mut claimed_agents: Vec<String> = tasks.iter().filter(|task| task.status == "running").map(|task| task.assignee_id.clone()).collect();
        let mut out = Vec::new();
        for task_id in task_ids {
            let mut task: Task = tx_get(tx, "task", task_id)?.ok_or_else(|| mark("precall", format!("업무 {task_id}를 찾을 수 없습니다")))?;
            if task.goal_id != goal_id { return Err(mark("precall", "다른 목표의 업무는 시작할 수 없습니다")); }
            if task.status != "pending" { return Err(mark("precall", format!("업무 '{}'는 이미 {} 상태라 다시 시작하지 않았습니다", task.title, task.status))); }
            if let Some(dep) = task.depends_on.iter().find(|dep| !tasks.iter().any(|item| &item.id == *dep && item.status == "completed")) {
                return Err(mark("precall", format!("업무 '{}'의 선행 업무 {dep}가 완료되지 않았습니다", task.title)));
            }
            let agent = agents.iter().find(|agent| agent.id == task.assignee_id).cloned().ok_or_else(|| mark("precall", format!("업무 '{}'의 담당 팀원이 없습니다", task.title)))?;
            if !is_implementer(&agent) { return Err(mark("precall", format!("{}은(는) 지금 구현 업무를 맡을 수 없습니다(숨김 또는 역할 변경)", agent.name))); }
            if let Some(squad) = squad.as_ref().filter(|squad| squad.status == "locked") {
                if !crate::squad::ids(squad, "writer").contains(&agent.id) { return Err(mark("precall", format!("{}은(는) CEO가 승인한 스쿼드의 작성자가 아니라 업무를 시작하지 않았습니다", agent.name))); }
            }
            let mut connection = connections.iter().find(|item| item.id == agent.connection_id).cloned().ok_or_else(|| mark("precall", format!("{}에게 AI 연결이 없습니다", agent.name)))?;
            usable(&connection).map_err(|error| mark("precall", format!("{}: {error}", agent.name)))?;
            can_write_files(&connection).map_err(|error| mark("precall", format!("{}: {error}", agent.name)))?;
            if let Some(reason) = busy.get(&agent.id) { return Err(mark("precall", format!("{}이(가) 다른 일을 하고 있습니다: {reason}", agent.name))); }
            if claimed_agents.contains(&agent.id) { return Err(mark("precall", format!("{}이(가) 이미 다른 업무를 실행 중입니다", agent.name))); }
            claimed_agents.push(agent.id.clone());
            if !agent.model_id.is_empty() { connection.model_id = agent.model_id.clone(); }
            task.status = "running".into();
            tx_put(tx, "task", &task.id.clone(), &task)?;
            crate::storage::tx_event(tx, "task_claimed", goal_id, &format!("{} claimed '{}' before any provider call", agent.name, task.title))?;
            out.push((task, agent, connection));
        }
        Ok(out)
    })
}

/// Returns tasks claimed in this wave to `pending` when the wave could not start (for example the
/// isolated workspace could not be prepared). No provider was called for them.
pub fn release_claims(store: &Store, goal_id: &str, task_ids: &[String], reason: &str) -> Result<(), String> {
    store.transact(|tx| {
        for task_id in task_ids {
            if let Some(mut task) = tx_get::<Task>(tx, "task", task_id)? {
                if task.status == "running" {
                    task.status = "pending".into();
                    task.worktree = None;
                    tx_put(tx, "task", &task.id.clone(), &task)?;
                }
            }
        }
        crate::storage::tx_event(tx, "task_claim_released", goal_id, reason)
    })
}

/// A blocked execution goal may resume its approved plan when every unfinished task never started
/// (ADR-0038 §4). Integrated tasks keep their proof; nothing is re-sent automatically.
pub fn resumable(goal: &Goal, tasks: &[Task], approvals: &[Approval]) -> Result<(), String> {
    if goal.kind != "execution" || goal.status != "blocked" { return Err("중단된 실행 목표만 이어서 실행할 수 있습니다".into()); }
    if !approvals.iter().any(|item| item.goal_id.as_deref() == Some(goal.id.as_str()) && item.operation == "project_write" && item.status == "approved") {
        return Err("승인된 프로젝트 변경 계획이 없습니다".into());
    }
    let owned: Vec<&Task> = tasks.iter().filter(|task| task.goal_id == goal.id).collect();
    if !owned.iter().any(|task| task.status == "pending") { return Err("아직 시작하지 않은 업무가 없습니다".into()); }
    if let Some(task) = owned.iter().find(|task| !["pending", "completed"].contains(&task.status.as_str())) {
        return Err(format!("업무 '{}'가 {} 상태입니다. 실행이 시작된 업무는 자동으로 이어서 실행하지 않습니다", task.title, task.status));
    }
    Ok(())
}

// ----------------------------- approval summary -----------------------------

pub fn approval_lines(tasks: &[Task], agents: &[Agent]) -> String {
    tasks.iter().enumerate().map(|(index, task)| {
        let name = agents.iter().find(|agent| agent.id == task.assignee_id).map(|agent| agent.name.clone()).unwrap_or_else(|| task.assignee_id.clone());
        let what = if task.deliverable.is_empty() { task.title.clone() } else { task.deliverable.clone() };
        let evidence = if task.evidence.is_empty() { "없음".to_string() } else { task.evidence.join(", ") };
        format!("{}. {what} — 담당 {name} · 파일: {} · 수용 기준 {}개 · 근거: {evidence}", index + 1, task.files.join(", "), task.acceptance.len())
    }).collect::<Vec<_>>().join("\n")
}

// ----------------------------- review -----------------------------

/// Title prefix the engine gives QA repair tasks; they refine the original deliverables.
pub const REPAIR_PREFIX: &str = "재작업 ";

/// The original deliverable tasks QA judges per criterion (repair tasks change the same files).
pub fn review_targets(tasks: &[Task]) -> Vec<Task> {
    tasks.iter().filter(|task| !task.acceptance.is_empty() && !task.title.starts_with(REPAIR_PREFIX)).cloned().collect()
}

pub fn save_reviews(store: &Store, goal_id: &str, round: u32, reviewer_id: &str, parsed: &ParsedReview) -> Result<(), String> {
    for (task_id, verdict, criteria, unverified) in &parsed.per_task {
        let review = crate::model::DeliverableReview { id: format!("{goal_id}:{task_id}:r{round}"), goal_id: goal_id.into(), task_id: task_id.clone(), round, verdict: verdict.clone(), criteria: criteria.clone(), unverified: unverified.clone(), reviewer_id: reviewer_id.into(), created_at: crate::model::now() };
        store.put("deliverable_review", &review.id.clone(), &review, Some(crate::storage::NewEvent { kind: "deliverable_reviewed".into(), goal_id: Some(goal_id.into()), task_id: Some(task_id.clone()), agent_id: Some(reviewer_id.into()), message: format!("QA {verdict}: {}/{} criteria met with evidence", criteria.iter().filter(|item| item.met && !item.evidence.trim().is_empty()).count(), criteria.len()) }))?;
    }
    Ok(())
}

#[derive(Debug)]
pub struct ParsedReview {
    pub overall_pass: bool,
    pub per_task: Vec<(String, String, Vec<CriterionResult>, Vec<String>)>,
    pub risks: Vec<String>,
}

impl ParsedReview {
    /// Pass only when the overall verdict passes and every task passes with every criterion met
    /// and evidenced.
    pub fn passes(&self) -> bool {
        self.overall_pass && self.per_task.iter().all(|(_, verdict, criteria, _)| verdict == "pass" && criteria.iter().all(|item| item.met && !item.evidence.trim().is_empty()))
    }
    pub fn summary(&self, tasks: &[Task]) -> String {
        self.per_task.iter().map(|(task_id, verdict, criteria, unverified)| {
            let title = tasks.iter().find(|task| &task.id == task_id).map(|task| if task.deliverable.is_empty() { task.title.clone() } else { task.deliverable.clone() }).unwrap_or_default();
            let met = criteria.iter().filter(|item| item.met && !item.evidence.trim().is_empty()).count();
            format!("- {title}: {} · 기준 {met}/{} 충족{}", if verdict == "pass" { "통과" } else { "실패" }, criteria.len(), if unverified.is_empty() { String::new() } else { format!(" · 미확인 {}개", unverified.len()) })
        }).collect::<Vec<_>>().join("\n")
    }
}

fn verdict(value: Option<&Value>) -> Option<String> {
    match value?.as_str()?.trim().to_ascii_uppercase().as_str() { "PASS" => Some("pass".into()), "FAIL" => Some("fail".into()), _ => None }
}

/// Parses QA's per-deliverable JSON. Every contract task must appear once with one result per
/// acceptance criterion, in order. Anything else is a form failure (never a pass).
pub fn parse_review(text: &str, tasks: &[Task]) -> Result<ParsedReview, String> {
    let json = crate::meeting::extract_json(text).map_err(|_| "QA 응답에서 산출물별 판정 JSON을 찾지 못했습니다".to_string())?;
    let value: Value = serde_json::from_str(json).map_err(|e| format!("QA 판정 JSON을 읽을 수 없습니다: {e}"))?;
    let overall = verdict(value.get("verdict")).ok_or("QA 판정에 전체 verdict(PASS/FAIL)가 없습니다")?;
    let items = value.get("deliverables").and_then(Value::as_array).ok_or("QA 판정에 deliverables 목록이 없습니다")?;
    let mut per_task = Vec::new();
    for task in tasks {
        let found: Vec<&Value> = items.iter().filter(|item| item.get("taskId").and_then(Value::as_str).map(str::trim) == Some(task.id.as_str())).collect();
        if found.len() != 1 { return Err(format!("QA 판정에 업무 '{}'({})의 결과가 정확히 한 번 있어야 합니다", task.title, task.id)); }
        let item = found[0];
        let task_verdict = verdict(item.get("verdict")).ok_or_else(|| format!("업무 '{}'의 verdict가 PASS/FAIL이 아닙니다", task.title))?;
        let criteria = item.get("criteria").and_then(Value::as_array).ok_or_else(|| format!("업무 '{}'의 criteria가 없습니다", task.title))?;
        if criteria.len() != task.acceptance.len() { return Err(format!("업무 '{}'의 기준 판정 수({})가 수용 기준 수({})와 다릅니다", task.title, criteria.len(), task.acceptance.len())); }
        let mut results = Vec::new();
        for (index, criterion) in criteria.iter().enumerate() {
            let met = criterion.get("met").and_then(Value::as_bool).ok_or_else(|| format!("업무 '{}'의 기준 {}에 met(true/false)가 없습니다", task.title, index + 1))?;
            let evidence = criterion.get("evidence").and_then(Value::as_str).unwrap_or("").trim().chars().take(600).collect::<String>();
            results.push(CriterionResult { criterion: task.acceptance[index].clone(), met, evidence });
        }
        let unverified = item.get("unverified").and_then(Value::as_array).map(|list| list.iter().filter_map(Value::as_str).map(|text| text.chars().take(300).collect()).collect()).unwrap_or_default();
        per_task.push((task.id.clone(), task_verdict, results, unverified));
    }
    let risks = value.get("risks").and_then(Value::as_array).map(|list| list.iter().filter_map(Value::as_str).map(|text| text.chars().take(300).collect()).collect()).unwrap_or_default();
    Ok(ParsedReview { overall_pass: overall == "pass", per_task, risks })
}

pub fn review_prompt_items(tasks: &[Task]) -> String {
    tasks.iter().map(|task| {
        let criteria = task.acceptance.iter().enumerate().map(|(index, item)| format!("    {}. {item}", index + 1)).collect::<Vec<_>>().join("\n");
        format!("- taskId: {}\n  deliverable: {}\n  files: {}\n  evidence (read-only inputs): {}\n  acceptance criteria (answer in this order):\n{criteria}", task.id, if task.deliverable.is_empty() { &task.title } else { &task.deliverable }, task.files.join(", "), if task.evidence.is_empty() { "none".to_string() } else { task.evidence.join(", ") })
    }).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{id, Project};
    use std::sync::{Arc, Barrier};

    fn agent(id: &str, role: &str, connection: &str) -> Agent {
        Agent { id: id.into(), name: id.to_uppercase(), role: role.into(), instructions: String::new(), reports_to: None, connection_id: connection.into(), model_id: String::new(), x: 0, y: 0, enabled: true, external: None, appearance: None }
    }
    fn conn(id: &str, kind: &str, status: &str) -> AiConnection {
        AiConnection { id: id.into(), name: format!("{id} conn"), kind: kind.into(), executable: "C:/x/claude.exe".into(), endpoint: String::new(), args: vec![], model_id: String::new(), status: status.into(), capabilities: vec![] }
    }
    #[test]
    fn an_image_task_goes_only_to_a_teammate_whose_connection_sees_images() {
        let agents = vec![agent("developer-a", "implementation", "codex"), agent("developer-b", "implementation", "claude")];
        let mut codex = conn("codex", "cli", "ready");
        codex.executable = "C:/x/codex.exe".into();
        let connections = vec![codex.clone(), conn("claude", "cli", "ready")];
        let picked = assign(&["document+images", "document", "document"], &agents, &connections, &BTreeMap::new()).unwrap();
        assert_eq!(picked[0].0, "developer-b");
        assert!(picked[0].1.contains("이미지 확인 가능"));
        let only_codex = vec![codex];
        assert!(assign(&["code+images"], &agents[..1], &only_codex, &BTreeMap::new()).unwrap_err().contains("이미지"));
        assert!(assign(&["code"], &agents[..1], &only_codex, &BTreeMap::new()).is_ok());
    }
    fn team() -> (Vec<Agent>, Vec<AiConnection>) {
        (vec![agent("cto", "architecture", "cli"), agent("pm", "planning", "cli"), agent("developer-a", "implementation", "cli"), agent("developer-b", "implementation", "cli2"), agent("qa", "validation and independent review", "cli")],
         vec![conn("cli", "cli", "ready"), conn("cli2", "cli", "needs_auth_check"), conn("http", "http", "ready"), conn("bad", "cli", "failed")])
    }

    #[test]
    fn block_marks_round_trip_and_ignore_untagged_text() {
        let tagged = mark("precall", "연결 실패");
        assert_eq!(split_mark(&tagged), ("precall".to_string(), "연결 실패".to_string()));
        assert_eq!(mark("provider", &tagged), tagged, "the first site wins");
        assert_eq!(split_mark("Provider=claude failed").0, "other");
        assert_eq!(split_mark("⟦admin⟧ fake").0, "other", "unknown kinds are not trusted");
    }

    #[test]
    fn contract_rejects_missing_criteria_capability_and_bad_evidence() {
        let root = std::env::temp_dir().join(format!("utopia-contract-{}", id()));
        std::fs::create_dir_all(root.join("notes")).unwrap();
        std::fs::write(root.join("notes/idea.md"), "idea").unwrap();
        let ok = vec!["Must name the target customer".to_string()];
        let none: Vec<String> = vec![];
        let ev = vec!["notes/idea.md".to_string()];
        let good = ContractInput { title: "t", deliverable: "고객·문제 가설", acceptance: &ok, evidence: &ev, capability: "document" };
        assert!(validate_contract(&[good], &root).is_ok());
        assert!(validate_contract(&[ContractInput { title: "t", deliverable: "d", acceptance: &none, evidence: &ev, capability: "document" }], &root).unwrap_err().contains("수용 기준"));
        assert!(validate_contract(&[ContractInput { title: "t", deliverable: "", acceptance: &ok, evidence: &ev, capability: "document" }], &root).unwrap_err().contains("산출물 설명"));
        assert!(validate_contract(&[ContractInput { title: "t", deliverable: "d", acceptance: &ok, evidence: &ev, capability: "research" }], &root).unwrap_err().contains("document 또는 code"));
        for bad in ["../outside.md", "notes/missing.md", "notes", "C:/x.md", ".git/config"] {
            let list = vec![bad.to_string()];
            assert!(validate_contract(&[ContractInput { title: "t", deliverable: "d", acceptance: &ok, evidence: &list, capability: "document" }], &root).is_err(), "{bad}");
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn assignment_checks_capability_connection_and_busy_and_explains() {
        let (mut agents, connections) = team();
        let busy = BTreeMap::new();
        let picked = assign(&["document", "document", "code"], &agents, &connections, &busy).unwrap();
        assert_eq!(picked.iter().map(|item| item.0.as_str()).collect::<Vec<_>>(), vec!["developer-a", "developer-b", "developer-a"], "balanced, then roster order");
        assert!(picked[0].1.contains("document 작업 가능"));
        assert!(picked.iter().all(|item| !["cto", "pm", "qa"].contains(&item.0.as_str())));
        let mut busy = BTreeMap::new();
        busy.insert("developer-a".to_string(), "다른 목표 업무".to_string());
        let picked = assign(&["document", "document"], &agents, &connections, &busy).unwrap();
        assert!(picked.iter().all(|item| item.0 == "developer-b"));
        assert!(picked[0].1.contains("제외: DEVELOPER-A: 다른 목표 업무"));
        agents[3].connection_id = "http".into();
        let error = assign(&["document"], &agents, &connections, &busy).unwrap_err();
        assert!(error.contains("HTTP") && error.contains("다른 목표 업무"), "{error}");
        agents[3].connection_id = "bad".into();
        assert!(assign(&["code"], &agents, &connections, &BTreeMap::new()).unwrap().iter().all(|item| item.0 == "developer-a"));
        agents[2].enabled = false;
        assert!(assign(&["code"], &agents, &connections, &BTreeMap::new()).unwrap_err().contains("failed"));
        let only_qa = vec![agent("qa", "implementation and review", "cli")];
        assert!(assign(&["code"], &only_qa, &connections, &BTreeMap::new()).is_err(), "QA never implements");
    }

    fn store_with_plan(two_tasks_same_agent: bool) -> (std::path::PathBuf, Arc<Store>, Vec<String>) {
        let dir = std::env::temp_dir().join(format!("utopia-claim-{}", id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("state.sqlite")).unwrap();
        let project = Project { id: "p".into(), path: dir.to_string_lossy().into_owned(), name: "P".into(), kind: "folder".into(), review_loop_limit: 2 };
        store.select_project(&project).unwrap();
        for item in team().1 { store.put("connection", &item.id.clone(), &item, None).unwrap(); }
        for item in team().0 { store.put("agent", &item.id.clone(), &item, None).unwrap(); }
        let goal = Goal { id: "g".into(), kind: "execution".into(), project_id: Some("p".into()), intent: "Launch docs".into(), status: "executing".into(), report: String::new(), created_at: crate::model::now(), resumed_from: None, review_loops_used: 0, review_loop_limit: None, work_group: None, block_kind: String::new() };
        store.put("goal", "g", &goal, None).unwrap();
        store.put("approval", "a", &Approval { id: "a".into(), goal_id: Some("g".into()), operation: "project_write".into(), reason: String::new(), status: "approved".into(), created_at: crate::model::now() }, None).unwrap();
        let mut ids = Vec::new();
        for (index, name) in ["hypothesis.md", "message.md"].iter().enumerate() {
            let assignee = if two_tasks_same_agent || index == 0 { "developer-a" } else { "developer-b" };
            let task = Task { id: format!("t{index}"), goal_id: "g".into(), title: name.to_string(), instructions: String::new(), files: vec![name.to_string()], assignee_id: assignee.into(), status: "pending".into(), depends_on: vec![], worktree: None, result: String::new(), session_id: None, deliverable: name.to_string(), acceptance: vec!["c".into()], evidence: vec![], capability: "document".into(), assignment_reason: String::new() };
            store.put("task", &task.id.clone(), &task, None).unwrap();
            ids.push(task.id);
        }
        (dir, Arc::new(store), ids)
    }

    #[test]
    fn concurrent_claims_let_exactly_one_win() {
        let (dir, store, ids) = store_with_plan(false);
        let barrier = Arc::new(Barrier::new(4));
        let handles: Vec<_> = (0..4).map(|_| { let store = store.clone(); let barrier = barrier.clone(); let id = ids[0].clone(); std::thread::spawn(move || { barrier.wait(); claim_wave(&store, "g", &[id]).is_ok() }) }).collect();
        let wins = handles.into_iter().map(|handle| handle.join().unwrap()).filter(|ok| *ok).count();
        assert_eq!(wins, 1);
        assert_eq!(store.get::<Task>("task", &ids[0]).unwrap().unwrap().status, "running");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn claim_is_all_or_nothing_and_rechecks_connection_busy_and_approval() {
        let (dir, store, ids) = store_with_plan(true);
        let error = claim_wave(&store, "g", &ids).unwrap_err();
        assert_eq!(split_mark(&error).0, "precall");
        assert!(store.list::<Task>("task").unwrap().iter().all(|task| task.status == "pending"), "a failed wave claims nothing");
        let (dir2, store2, ids2) = store_with_plan(false);
        let mut connection: AiConnection = store2.get("connection", "cli2").unwrap().unwrap();
        connection.status = "failed".into();
        store2.put("connection", "cli2", &connection, None).unwrap();
        assert!(split_mark(&claim_wave(&store2, "g", &ids2).unwrap_err()).1.contains("failed"));
        connection.status = "ready".into();
        store2.put("connection", "cli2", &connection, None).unwrap();
        // developer-b reserved by another goal's pending task
        let other = Goal { id: "other".into(), kind: "execution".into(), project_id: Some("p".into()), intent: "Other".into(), status: "executing".into(), report: String::new(), created_at: crate::model::now(), resumed_from: None, review_loops_used: 0, review_loop_limit: None, work_group: None, block_kind: String::new() };
        store2.put("goal", "other", &other, None).unwrap();
        let mut foreign: Task = store2.get("task", &ids2[1]).unwrap().unwrap();
        foreign.id = "foreign".into(); foreign.goal_id = "other".into();
        store2.put("task", "foreign", &foreign, None).unwrap();
        assert!(split_mark(&claim_wave(&store2, "g", &ids2).unwrap_err()).1.contains("다른 일"));
        store2.put("task", "foreign", &Task { status: "completed".into(), ..foreign.clone() }, None).unwrap();
        let mut approval: Approval = store2.get("approval", "a").unwrap().unwrap();
        approval.status = "denied".into();
        store2.put("approval", "a", &approval, None).unwrap();
        assert!(split_mark(&claim_wave(&store2, "g", &ids2).unwrap_err()).1.contains("승인"));
        approval.status = "approved".into();
        store2.put("approval", "a", &approval, None).unwrap();
        let claimed = claim_wave(&store2, "g", &ids2).unwrap();
        assert_eq!(claimed.len(), 2, "independent files and assignees run in one parallel wave");
        release_claims(&store2, "g", &ids2, "workspace failed").unwrap();
        assert!(store2.list::<Task>("task").unwrap().iter().filter(|task| task.goal_id == "g").all(|task| task.status == "pending"));
        std::fs::remove_dir_all(dir).unwrap();
        std::fs::remove_dir_all(dir2).unwrap();
    }

    #[test]
    fn resume_only_when_every_unfinished_task_never_started() {
        let (dir, store, _) = store_with_plan(false);
        let mut goal: Goal = store.get("goal", "g").unwrap().unwrap();
        goal.status = "blocked".into();
        let snapshot = store.snapshot().unwrap();
        assert!(resumable(&goal, &snapshot.tasks, &snapshot.approvals).is_ok());
        let mut tasks = snapshot.tasks.clone();
        tasks[0].status = "completed".into();
        assert!(resumable(&goal, &tasks, &snapshot.approvals).is_ok(), "integrated work stays, remaining pending work resumes");
        for status in ["running", "blocked", "failed", "awaiting_integration"] {
            tasks[1].status = status.into();
            assert!(resumable(&goal, &tasks, &snapshot.approvals).is_err(), "{status}");
        }
        let denied: Vec<Approval> = snapshot.approvals.iter().cloned().map(|mut item| { item.status = "denied".into(); item }).collect();
        assert!(resumable(&goal, &snapshot.tasks, &denied).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    fn task(id: &str, criteria: usize) -> Task {
        Task { id: id.into(), goal_id: "g".into(), title: id.into(), instructions: String::new(), files: vec![format!("{id}.md")], assignee_id: "developer-a".into(), status: "completed".into(), depends_on: vec![], worktree: None, result: String::new(), session_id: None, deliverable: format!("{id} doc"), acceptance: (0..criteria).map(|n| format!("criterion {n}")).collect(), evidence: vec![], capability: "document".into(), assignment_reason: String::new() }
    }

    #[test]
    fn review_passes_only_with_every_criterion_met_and_evidenced() {
        let tasks = vec![task("a", 2), task("b", 1)];
        let good = r#"Here is my review:
{"verdict":"PASS","deliverables":[
 {"taskId":"a","verdict":"PASS","criteria":[{"met":true,"evidence":"a.md §1"},{"met":true,"evidence":"a.md table"}],"unverified":["market size"]},
 {"taskId":"b","verdict":"pass","criteria":[{"met":true,"evidence":"b.md list"}]}],"risks":["no real interviews"]}"#;
        let parsed = parse_review(good, &tasks).unwrap();
        assert!(parsed.passes());
        assert_eq!(parsed.per_task[0].3, vec!["market size".to_string()]);
        assert!(parsed.summary(&tasks).contains("기준 2/2 충족 · 미확인 1개"));
        let no_evidence = good.replace(r#""a.md table""#, r#""""#);
        assert!(!parse_review(&no_evidence, &tasks).unwrap().passes(), "a met criterion without evidence does not pass");
        let inconsistent = good.replace(r#"{"met":true,"evidence":"b.md list"}"#, r#"{"met":false,"evidence":"missing"}"#);
        assert!(!parse_review(&inconsistent, &tasks).unwrap().passes(), "overall PASS cannot hide an unmet criterion");
        assert!(parse_review("PASS — looks good", &tasks).is_err());
        assert!(parse_review(&good.replace(r#""taskId":"b""#, r#""taskId":"z""#), &tasks).unwrap_err().contains("정확히 한 번"));
        assert!(parse_review(&good.replace(r#",{"met":true,"evidence":"a.md table"}"#, ""), &tasks).unwrap_err().contains("기준 판정 수"));
    }
}
