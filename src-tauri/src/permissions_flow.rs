//! ADR-0058: the engine side of command permissions. Grants live on the goal conversation; a refused
//! call stops the work at a `command_permission` card, and approval resumes the same provider session.

use crate::engine::{check_goal_guidance, connection, execute_tracked, execute_tracked_with, record_progress, transition_goal};
use crate::model::{Agent, AiConnection, Approval, Goal, GoalGuidance, Project, Task};
use crate::permissions::{self, CommandGrants, CommandRequest, Denial, Rule};
use crate::provider::CallOptions;
use crate::storage::{NewEvent, Store};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};

/// Calls a task's provider refused and that no grant covers yet.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TaskDenials { pub task_id: String, pub goal_id: String, pub denials: Vec<Denial>, pub resumes: u32 }

pub fn conversation(store: &Store, goal_id: &str) -> Result<String, String> {
    Ok(permissions::conversation_id(goal_id, &store.list::<Goal>("goal")?, &store.list::<GoalGuidance>("goal_guidance")?))
}

pub fn grants(store: &Store, goal_id: &str) -> Result<Vec<Rule>, String> {
    let id = conversation(store, goal_id)?;
    Ok(store.get::<CommandGrants>("command_grant", &id)?.map(|item| item.rules).unwrap_or_default())
}

fn add_grants(store: &Store, goal_id: &str, rules: &[Rule]) -> Result<(), String> {
    let id = conversation(store, goal_id)?;
    let mut saved = store.get::<CommandGrants>("command_grant", &id)?.unwrap_or(CommandGrants { conversation_id: id.clone(), rules: Vec::new() });
    for rule in rules { if !saved.rules.iter().any(|existing| existing.key() == rule.key()) { saved.rules.push(rule.clone()); } }
    store.put("command_grant", &id, &saved, None)
}

pub fn options_for(store: &Store, goal_id: &str, resume: Option<String>, read_only_commands: bool) -> Result<CallOptions, String> {
    Ok(CallOptions { allowed: permissions::cli_allowed(&grants(store, goal_id)?), resume, read_only_commands })
}

/// A line for the teammate's prompt naming what it may run (also useful to providers without grants).
pub fn approved_note(store: &Store, goal_id: &str) -> Result<String, String> {
    let rules = grants(store, goal_id)?;
    if rules.is_empty() { return Ok(String::new()); }
    Ok(format!("\nCommands the CEO approved in this goal conversation (run these when the task needs them; other commands may be refused): {}", rules.iter().map(|rule| format!("`{}`", rule.prefix)).collect::<Vec<_>>().join(", ")))
}

fn label(rule: &Rule) -> String { if rule.prefix.is_empty() { rule.tool.clone() } else { rule.prefix.clone() } }

/// Records the plan's commands as a request attached to the plan approval; returns a line for its scope text.
pub fn plan_request(store: &Store, goal_id: &str, approval_id: &str, commands: &[String]) -> Result<String, String> {
    let granted = grants(store, goal_id)?;
    let rules: Vec<Rule> = permissions::rules_for_commands(commands).into_iter().filter(|rule| !granted.iter().any(|item| item.key() == rule.key())).collect();
    if rules.is_empty() { return Ok(String::new()); }
    let request = CommandRequest { id: approval_id.into(), goal_id: goal_id.into(), conversation_id: conversation(store, goal_id)?, source: "plan".into(), rules: rules.clone(), granted: Vec::new(), task_ids: Vec::new(), created_at: crate::model::now() };
    store.put("command_request", approval_id, &request, None)?;
    let safe: Vec<String> = rules.iter().filter(|rule| rule.class == "safe").map(label).collect();
    let later: Vec<String> = rules.iter().filter(|rule| rule.class != "safe").map(label).collect();
    let mut note = String::new();
    if !safe.is_empty() { note.push_str(&format!("\n실행할 명령(승인하면 이 대화에서 허용): {}", safe.join(", "))); }
    if !later.is_empty() { note.push_str(&format!("\n따로 확인할 명령(필요할 때 다시 물어요): {}", later.join(", "))); }
    Ok(note)
}

/// Approving the plan grants its safe commands for the conversation.
pub fn grant_plan_commands(store: &Store, goal_id: &str, approval_id: &str) -> Result<(), String> {
    let Some(mut request) = store.get::<CommandRequest>("command_request", approval_id)? else { return Ok(()) };
    if request.source != "plan" || !request.granted.is_empty() { return Ok(()); }
    let safe: Vec<Rule> = request.rules.iter().filter(|rule| rule.class == "safe").cloned().collect();
    if safe.is_empty() { return Ok(()); }
    add_grants(store, goal_id, &safe)?;
    request.granted = safe.iter().map(Rule::key).collect();
    store.put("command_request", approval_id, &request, Some(NewEvent { kind: "commands_granted".into(), goal_id: Some(goal_id.into()), task_id: None, agent_id: None, message: format!("plan: {}", safe.iter().map(label).collect::<Vec<_>>().join(", ")) }))
}

pub fn record_task_denials(store: &Store, task: &Task, denials: &[Denial], resumes: u32) -> Result<(), String> {
    let granted = grants(store, &task.goal_id)?;
    let open: Vec<Denial> = denials.iter().filter(|denial| !permissions::covered(&granted, &denial.tool, &denial.command)).cloned().collect();
    let record = TaskDenials { task_id: task.id.clone(), goal_id: task.goal_id.clone(), denials: open.clone(), resumes };
    let event = (!open.is_empty()).then(|| NewEvent::task("permission_denied", task, &format!("{} refused: {}", task.title, open.iter().map(|item| item.command.clone()).collect::<Vec<_>>().join(" / ").chars().take(300).collect::<String>())));
    store.put("task_denial", &task.id, &record, event)
}

fn clear(store: &Store, record: &TaskDenials) -> Result<(), String> {
    store.put("task_denial", &record.task_id, &TaskDenials { denials: Vec::new(), ..record.clone() }, None)
}

/// Shows a `command_permission` card and waits for the CEO. `true` when anything was approved.
fn ask<R: tauri::Runtime>(app: &tauri::AppHandle<R>, goal_id: &str, rules: Vec<Rule>, task_ids: Vec<String>, who: &str, back_to: &str) -> Result<bool, String> {
    let store = app.state::<Store>();
    let id = crate::model::id();
    let request = CommandRequest { id: id.clone(), goal_id: goal_id.into(), conversation_id: conversation(&store, goal_id)?, source: "blocked".into(), rules: rules.clone(), granted: Vec::new(), task_ids, created_at: crate::model::now() };
    store.put("command_request", &id, &request, None)?;
    let lines = rules.iter().map(|rule| format!("- {}{}", label(rule), match rule.class.as_str() { "safe" => String::new(), "individual" => format!(" (따로 확인: {})", rule.reason), _ => format!(" (허용 불가: {})", rule.reason) })).collect::<Vec<_>>().join("\n");
    let approval = Approval { id: id.clone(), goal_id: Some(goal_id.into()), operation: permissions::OPERATION.into(), reason: format!("{who}이(가) 이 명령을 실행하려다 막혔어요. 승인하면 이 목표 대화에서만 허용하고, 같은 작업을 이어서 실행해요.\n{lines}"), status: "pending".into(), created_at: crate::model::now() };
    store.put("approval", &id, &approval, Some(NewEvent { kind: "approval_requested".into(), goal_id: Some(goal_id.into()), task_id: None, agent_id: None, message: approval.reason.clone() }))?;
    transition_goal(app, goal_id, "awaiting_approval", "CEO approval requested for refused commands")?;
    let deadline = Instant::now() + Duration::from_secs(24 * 60 * 60);
    let approved = loop {
        check_goal_guidance(app, goal_id, "명령 권한 승인 대기 중")?;
        let current: Approval = store.get("approval", &id)?.ok_or("Approval disappeared")?;
        match current.status.as_str() {
            "approved" => break true,
            "denied" => break false,
            "pending" if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(300)),
            "pending" => return Err("Command permission approval expired".into()),
            _ => return Err("Unknown approval status".into()),
        }
    };
    transition_goal(app, goal_id, back_to, if approved { "Commands approved; the refused work continues" } else { "Commands not approved; work continues as reported" })?;
    Ok(approved)
}

/// Before integration: ask about refused commands of the wave's tasks and resume those tasks.
pub fn settle_task_denials<R: tauri::Runtime>(app: &tauri::AppHandle<R>, goal_id: &str, project: &Project, goal: &Goal, wave_ids: &[String]) -> Result<(), String> {
    let store = app.state::<Store>();
    loop {
        let pending: Vec<TaskDenials> = wave_ids.iter().filter_map(|id| store.get::<TaskDenials>("task_denial", id).ok().flatten()).filter(|item| !item.denials.is_empty()).collect();
        if pending.is_empty() { return Ok(()); }
        let granted = grants(&store, goal_id)?;
        let denials: Vec<Denial> = pending.iter().flat_map(|item| item.denials.clone()).collect();
        let rules: Vec<Rule> = permissions::rules_for_denials(&denials).into_iter().filter(|rule| !granted.iter().any(|item| item.key() == rule.key())).collect();
        let approved = if rules.iter().all(|rule| rule.class == "blocked") && !rules.is_empty() { false }
            else if rules.is_empty() { true }
            else {
                let snapshot = store.snapshot()?;
                let mut names: Vec<String> = pending.iter().filter_map(|item| snapshot.tasks.iter().find(|task| task.id == item.task_id)).filter_map(|task| snapshot.agents.iter().find(|agent| agent.id == task.assignee_id).map(|agent| agent.name.clone())).collect();
                names.dedup();
                let who = if names.is_empty() { "팀원".to_string() } else { names.join(", ") };
                ask(app, goal_id, rules, pending.iter().map(|item| item.task_id.clone()).collect(), &who, "executing")?
            };
        if !approved { for record in &pending { clear(&store, record)?; } return Ok(()); }
        for record in pending {
            if record.resumes >= permissions::MAX_RESUMES { clear(&store, &record)?; continue; }
            resume_task(app, project, goal, &record)?;
        }
    }
}

fn resume_task<R: tauri::Runtime>(app: &tauri::AppHandle<R>, project: &Project, goal: &Goal, record: &TaskDenials) -> Result<(), String> {
    let store = app.state::<Store>();
    let mut task: Task = store.get("task", &record.task_id)?.ok_or("업무 기록이 사라졌습니다")?;
    let snapshot = store.snapshot()?;
    let agent: Agent = snapshot.agents.iter().find(|agent| agent.id == task.assignee_id).cloned().ok_or("담당 팀원을 찾을 수 없습니다")?;
    let selected: AiConnection = connection(&snapshot.connections, &agent)?;
    let worktree = task.worktree.clone().ok_or("Missing worktree")?;
    let approved = grants(&store, &goal.id)?.iter().map(label).collect::<Vec<_>>().join(", ");
    let prompt = format!("You are {} ({}). The CEO approved these commands for this goal conversation: {approved}. Continue your task in this isolated folder: run the commands that were refused before and finish the work, changing ONLY these declared files: {}. Do not redo finished edits without reason. Task: {}\nInstructions: {}\nCEO goal: {}\nReturn a concise summary with the command output as evidence.", agent.name, agent.role, task.files.join(", "), task.title, task.instructions, goal.intent);
    let options = options_for(&store, &goal.id, task.session_id.clone(), false)?;
    record_progress(app, &goal.id, Some(&task.id), &agent.id, "승인된 명령으로 작업을 이어서 하고 있습니다");
    match execute_tracked_with(app, &project.id, &goal.id, Some(&task.id), &agent.id, &selected, &prompt, Path::new(&worktree), true, &options) {
        Ok(result) => {
            task.result = format!("{}\n\n[명령 승인 후 이어서 실행]\n{}", task.result, result.text).chars().rev().take(10_000).collect::<Vec<_>>().into_iter().rev().collect();
            if result.session_id.is_some() { task.session_id = result.session_id.clone(); }
            store.put("task", &task.id, &task, Some(NewEvent::task("task_resumed_with_commands", &task, "Teammate continued with approved commands")))?;
            record_task_denials(&store, &task, &result.permission_denials, record.resumes + 1)?;
            let _ = app.emit("state-changed", ());
            Ok(())
        }
        Err(error) => {
            task.status = "failed".into();
            task.result = error.clone();
            store.put("task", &task.id, &task, Some(NewEvent::task("task_failed", &task, &error)))?;
            Err(crate::delegation::mark("provider", error))
        }
    }
}

/// Whether the provider reports refused commands and accepts grants.
fn takes_grants(connection: &AiConnection) -> bool {
    connection.kind == "plugin" || connection.kind == "cli" && Path::new(&connection.executable).file_stem().is_some_and(|stem| stem.to_string_lossy().eq_ignore_ascii_case("claude"))
}

/// QA: on a provider that takes grants, review in a copy of the integrated project with the
/// conversation's commands allowed; refused commands are asked about and the review resumes.
#[allow(clippy::too_many_arguments)]
pub fn review_with_commands<R: tauri::Runtime>(app: &tauri::AppHandle<R>, project: &Project, goal_id: &str, reviewer: &Agent, selected: &AiConnection, prompt: &str, root: &Path, worktree_root: &Path) -> Result<String, String> {
    let store = app.state::<Store>();
    if !takes_grants(selected) { return Ok(execute_tracked(app, &project.id, goal_id, None, &reviewer.id, selected, prompt, root, false)?.text); }
    let copy = worktree_root.join(format!("review-{}", crate::model::id()));
    if let Err(error) = crate::platform::copy_for_review(root, &copy) {
        let _ = store.append_event(NewEvent { kind: "review_copy_skipped".into(), goal_id: Some(goal_id.into()), task_id: None, agent_id: Some(reviewer.id.clone()), message: format!("QA reads the project without running commands: {error}") });
        let _ = std::fs::remove_dir_all(&copy);
        return Ok(execute_tracked(app, &project.id, goal_id, None, &reviewer.id, selected, prompt, root, false)?.text);
    }
    let result = (|| -> Result<String, String> {
        let prompt = format!("{prompt}\nYou review a disposable copy of the integrated project; you may run the approved commands to check criteria, but your file tools cannot write.{}", approved_note(&store, goal_id)?);
        let mut response = execute_tracked_with(app, &project.id, goal_id, None, &reviewer.id, selected, &prompt, &copy, false, &options_for(&store, goal_id, None, true)?)?;
        for _ in 0..permissions::MAX_RESUMES {
            let granted = grants(&store, goal_id)?;
            let open: Vec<Denial> = response.permission_denials.iter().filter(|denial| !permissions::covered(&granted, &denial.tool, &denial.command)).cloned().collect();
            if open.is_empty() { break; }
            let _ = store.append_event(NewEvent { kind: "permission_denied".into(), goal_id: Some(goal_id.into()), task_id: None, agent_id: Some(reviewer.id.clone()), message: format!("QA refused: {}", open.iter().map(|item| item.command.clone()).collect::<Vec<_>>().join(" / ").chars().take(300).collect::<String>()) });
            let rules: Vec<Rule> = permissions::rules_for_denials(&open).into_iter().filter(|rule| !granted.iter().any(|item| item.key() == rule.key())).collect();
            if rules.is_empty() || rules.iter().all(|rule| rule.class == "blocked") || !ask(app, goal_id, rules, Vec::new(), &format!("QA({})", reviewer.name), "validating")? { break; }
            let approved = grants(&store, goal_id)?.iter().map(label).collect::<Vec<_>>().join(", ");
            let follow = format!("The CEO approved these commands for this goal conversation: {approved}. Run the checks that were refused before, then give your complete review again in exactly the format requested earlier.\n\nThe original request, for reference:\n{prompt}");
            record_progress(app, goal_id, None, &reviewer.id, "QA가 승인된 명령으로 검토를 이어서 하고 있습니다");
            response = execute_tracked_with(app, &project.id, goal_id, None, &reviewer.id, selected, &follow, &copy, false, &options_for(&store, goal_id, response.session_id.clone(), true)?)?;
        }
        Ok(response.text)
    })();
    let _ = std::fs::remove_dir_all(&copy);
    result
}

/// `decide_command_permission`: the CEO's choice on a `command_permission` card. An empty choice
/// is `승인하지 않음`. Only listed, grantable rules can be chosen.
pub fn decide(store: &Store, approval_id: &str, keys: &[String]) -> Result<(), String> {
    let approval: Approval = store.get("approval", approval_id)?.ok_or("승인 요청을 찾을 수 없습니다")?;
    if approval.operation != permissions::OPERATION { return Err("명령 권한 요청이 아닙니다".into()); }
    if approval.status != "pending" { return Err("이미 결정한 요청입니다".into()); }
    let mut request: CommandRequest = store.get("command_request", approval_id)?.ok_or("명령 권한 요청 기록이 없습니다")?;
    let chosen = permissions::chosen_rules(&request, keys)?;
    let goal_id = approval.goal_id.clone().ok_or("목표에 묶이지 않은 요청입니다")?;
    if !chosen.is_empty() { add_grants(store, &goal_id, &chosen)?; }
    request.granted = chosen.iter().map(Rule::key).collect();
    store.put("command_request", approval_id, &request, None)?;
    let decided = Approval { status: if chosen.is_empty() { "denied" } else { "approved" }.into(), ..approval };
    store.put("approval", approval_id, &decided, Some(NewEvent { kind: "approval_decided".into(), goal_id: Some(goal_id), task_id: None, agent_id: None, message: format!("{}: {}{}", decided.operation, decided.status, if chosen.is_empty() { String::new() } else { format!(" ({})", chosen.iter().map(label).collect::<Vec<_>>().join(", ")) }) }))
}
