//! ADR-0058: the engine side of command permissions. Grants live on the goal conversation; a refused
//! call stops the work at a `command_permission` card, and approval resumes the same provider session.

use crate::engine::{check_goal_guidance, connection, execute_tracked, execute_tracked_with, record_progress, transition_goal};
use crate::model::{Agent, AiConnection, Approval, Goal, GoalGuidance, Project, Squad, Task};
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
pub struct TaskDenials { pub task_id: String, pub goal_id: String, pub denials: Vec<Denial>, pub resumes: u32, #[serde(default)] pub pm_reviewed: bool, #[serde(default)] pub pm_reason: String, #[serde(default)] pub selected_rule_keys: Vec<String>, #[serde(default)] pub auto_attempted: Vec<String> }

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct TaskAutoGrants { task_id: String, rules: Vec<Rule> }

#[derive(Deserialize)]
struct PmDecision { action: String, reason: String, #[serde(default)] rule_keys: Vec<String> }

fn parse_pm_decision(text: &str) -> Result<PmDecision, String> {
    let trimmed = text.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
    let decision: PmDecision = serde_json::from_str(trimmed).map_err(|_| "PM 권한 검토 응답이 JSON 형식이 아닙니다".to_string())?;
    if !["request_approval", "retry_with_existing_grants", "continue_without"].contains(&decision.action.as_str()) || decision.reason.trim().is_empty() || decision.reason.chars().count() > 300 || (decision.action == "request_approval" && decision.rule_keys.is_empty()) {
        return Err("PM 권한 검토 응답의 결정이나 이유가 유효하지 않습니다".into());
    }
    Ok(decision)
}

pub fn conversation(store: &Store, goal_id: &str) -> Result<String, String> {
    Ok(permissions::conversation_id(goal_id, &store.list::<Goal>("goal")?, &store.list::<GoalGuidance>("goal_guidance")?))
}

/// ADR-0063: safe grants are kept per project (`command_grant_project`, keyed by project ID).
const PROJECT_GRANTS: &str = "command_grant_project";

fn project_of(store: &Store, goal_id: &str) -> Result<Option<String>, String> {
    Ok(store.get::<Goal>("goal", goal_id)?.and_then(|goal| goal.project_id))
}

/// The conversation's grants plus the safe grants of the goal's project (ADR-0063).
pub fn grants(store: &Store, goal_id: &str) -> Result<Vec<Rule>, String> {
    let id = conversation(store, goal_id)?;
    let mut rules = store.get::<CommandGrants>("command_grant", &id)?.map(|item| item.rules).unwrap_or_default();
    if let Some(project_id) = project_of(store, goal_id)? {
        for rule in store.get::<CommandGrants>(PROJECT_GRANTS, &project_id)?.map(|item| item.rules).unwrap_or_default() {
            if rule.class == "safe" && !rules.iter().any(|existing| existing.key() == rule.key()) { rules.push(rule); }
        }
    }
    Ok(rules)
}

fn task_auto_grants(store: &Store, task: &Task) -> Result<Vec<Rule>, String> {
    let mode = crate::permission_mode::effective(store, &task.goal_id)?;
    let saved = store.get::<TaskAutoGrants>("task_auto_grant", &task.id)?.unwrap_or_default();
    Ok(saved.rules.into_iter().filter(|rule| crate::permission_mode::eligible(&mode, &Denial { tool: rule.tool.clone(), command: rule.prefix.clone() }, Some(task)).is_some()).collect())
}

fn add_task_auto_grants(store: &Store, task: &Task, rules: &[Rule]) -> Result<(), String> {
    let mut saved = store.get::<TaskAutoGrants>("task_auto_grant", &task.id)?.unwrap_or(TaskAutoGrants { task_id: task.id.clone(), rules: Vec::new() });
    for rule in rules { if !saved.rules.iter().any(|item| item.key() == rule.key()) { saved.rules.push(rule.clone()); } }
    store.put("task_auto_grant", &task.id, &saved, Some(NewEvent::task("command_auto_granted", task, &format!("Exact task commands: {}", rules.iter().map(Rule::key).collect::<Vec<_>>().join(", ")))))
}

fn add_grants(store: &Store, goal_id: &str, rules: &[Rule]) -> Result<(), String> {
    let id = conversation(store, goal_id)?;
    let mut saved = store.get::<CommandGrants>("command_grant", &id)?.unwrap_or(CommandGrants { conversation_id: id.clone(), rules: Vec::new() });
    for rule in rules { if !saved.rules.iter().any(|existing| existing.key() == rule.key()) { saved.rules.push(rule.clone()); } }
    store.put("command_grant", &id, &saved, None)?;
    // ADR-0063: safe rules the CEO approved also apply to later goals of the same project.
    let safe: Vec<&Rule> = rules.iter().filter(|rule| rule.class == "safe").collect();
    if let (Some(project_id), false) = (project_of(store, goal_id)?, safe.is_empty()) {
        let mut project = store.get::<CommandGrants>(PROJECT_GRANTS, &project_id)?.unwrap_or(CommandGrants { conversation_id: project_id.clone(), rules: Vec::new() });
        for rule in safe { if !project.rules.iter().any(|existing| existing.key() == rule.key()) { project.rules.push(rule.clone()); } }
        store.put(PROJECT_GRANTS, &project_id, &project, None)?;
    }
    Ok(())
}

pub fn options_for(store: &Store, goal_id: &str, resume: Option<String>, read_only_commands: bool) -> Result<CallOptions, String> {
    Ok(CallOptions { allowed: permissions::cli_allowed(&grants(store, goal_id)?), resume, read_only_commands })
}

pub fn options_for_task(store: &Store, task: &Task, resume: Option<String>) -> Result<CallOptions, String> {
    let mut allowed = grants(store, &task.goal_id)?;
    allowed.extend(task_auto_grants(store, task)?);
    Ok(CallOptions { allowed: permissions::cli_allowed(&allowed), resume, read_only_commands: false })
}

/// A line for the teammate's prompt naming what it may run (also useful to providers without grants).
pub fn approved_note(store: &Store, goal_id: &str) -> Result<String, String> {
    let rules = grants(store, goal_id)?;
    let mut note = String::from("\nRun commands from the root of this folder; do not change directory outside it. Run project tools through package scripts (`npm test`, `npm run <script>`) rather than `npx`, which may download packages.");
    if !rules.is_empty() { note.push_str(&format!("\nCommands the CEO approved (run these when the task needs them; other commands may be refused): {}", rules.iter().map(|rule| format!("`{}`", rule.prefix)).collect::<Vec<_>>().join(", "))); }
    Ok(note)
}

/// ADR-0063: the commands a teammate's call actually ran, as a third-depth task event.
pub fn record_commands_run(store: &Store, task: &Task, commands: &[String]) -> Result<(), String> {
    if commands.is_empty() { return Ok(()); }
    let text = commands.join(" / ").chars().take(2000).collect::<String>();
    store.append_event(NewEvent::task("commands_run", task, &format!("{} ran: {text}", task.title)))
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
    let prior = store.get::<TaskDenials>("task_denial", &task.id)?;
    let record = TaskDenials { task_id: task.id.clone(), goal_id: task.goal_id.clone(), denials: open.clone(), resumes, pm_reviewed: false, pm_reason: prior.as_ref().map(|item| item.pm_reason.clone()).unwrap_or_default(), selected_rule_keys: Vec::new(), auto_attempted: prior.map(|item| item.auto_attempted).unwrap_or_default() };
    let event = (!open.is_empty()).then(|| NewEvent::task("permission_denied", task, &format!("{} refused: {}", task.title, open.iter().map(|item| item.command.clone()).collect::<Vec<_>>().join(" / ").chars().take(300).collect::<String>())));
    store.put("task_denial", &task.id, &record, event)
}

fn clear(store: &Store, record: &TaskDenials) -> Result<(), String> {
    store.put("task_denial", &record.task_id, &TaskDenials { denials: Vec::new(), ..record.clone() }, None)
}

/// PM recommends the next branch. Its text never becomes a command rule or file scope.
fn pm_review<R: tauri::Runtime>(app: &tauri::AppHandle<R>, project: &Project, goal: &Goal, record: &TaskDenials) -> Result<PmDecision, String> {
    let store = app.state::<Store>();
    let snapshot = store.snapshot()?;
    let task = snapshot.tasks.iter().find(|task| task.id == record.task_id).ok_or("검토할 업무가 없습니다")?;
    let squad: Option<Squad> = store.get("squad", &goal.id)?;
    let pm_id = squad.as_ref().and_then(|squad| squad.members.iter().find(|member| member.slot == "lead" && member.duty == "planning").map(|member| member.agent_id.as_str())).unwrap_or("pm");
    let pm = snapshot.agents.iter().find(|agent| agent.id == pm_id).ok_or("계획 PM을 찾을 수 없습니다")?;
    let selected = connection(&snapshot.connections, pm)?;
    let approved = grants(&store, &goal.id)?.iter().map(Rule::key).collect::<Vec<_>>().join(", ");
    let denied = record.denials.iter().enumerate().map(|(index, item)| format!("[{index}] {}: {}", item.tool, item.command)).collect::<Vec<_>>().join("\n");
    let candidates = permissions::rules_for_denials(&record.denials).iter().map(Rule::key).collect::<Vec<_>>().join("\n");
    let prompt = format!("You are the planning PM reviewing a Dev permission request. READ ONLY: do not edit files or run commands. Return ONLY JSON {{\"action\":\"request_approval|retry_with_existing_grants|continue_without\",\"reason\":\"short Korean reason\",\"rule_keys\":[\"Bash(node --test)\"]}}. Select ONLY candidate rule keys strictly necessary to complete the approved task; omit routine checks and unnecessary commands even when they were chained in the same refused call. For request_approval, rule_keys must be a nonempty subset of Candidate rules. A multiline script or heredoc cannot be approved as fragments: choose retry_with_existing_grants and ask Dev to issue minimal separate commands instead. Choose continue_without when the current result is sufficient for QA. You cannot grant any new command, network access, or file path.\nCEO goal: {}\nTask: {}\nTask instructions: {}\nApproved files: {}\nExisting command grants: {}\nRefused calls:\n{}\nCandidate rules:\n{}\nDev result: {}", goal.intent, task.title, task.instructions, task.files.join(", "), approved, denied, candidates, task.result.chars().take(2000).collect::<String>());
    if prompt.chars().count() > 12_000 { return Err("PM 검토 입력이 너무 길어 자동 검토를 건너뜁니다".into()); }
    record_progress(app, &goal.id, Some(&task.id), &pm.id, "PM이 Dev의 권한 요청을 검토하고 있습니다");
    let result = execute_tracked(app, &project.id, &goal.id, Some(&task.id), &pm.id, &selected, &prompt, Path::new(&project.path), false)?;
    parse_pm_decision(&result.text)
}

/// Shows a `command_permission` card and waits for the CEO. `true` when anything was approved.
fn ask<R: tauri::Runtime>(app: &tauri::AppHandle<R>, goal_id: &str, rules: Vec<Rule>, task_ids: Vec<String>, who: &str, back_to: &str, pm_reason: &str) -> Result<bool, String> {
    let store = app.state::<Store>();
    let id = crate::model::id();
    let request = CommandRequest { id: id.clone(), goal_id: goal_id.into(), conversation_id: conversation(&store, goal_id)?, source: "blocked".into(), rules: rules.clone(), granted: Vec::new(), task_ids, created_at: crate::model::now() };
    store.put("command_request", &id, &request, None)?;
    let lines = rules.iter().map(|rule| format!("- {}{}", label(rule), match rule.class.as_str() { "safe" => String::new(), "individual" => format!(" (따로 확인: {})", rule.reason), _ => format!(" (허용 불가: {})", rule.reason) })).collect::<Vec<_>>().join("\n");
    let approval = Approval { id: id.clone(), goal_id: Some(goal_id.into()), operation: permissions::OPERATION.into(), reason: format!("{who}이(가) 이 명령을 실행하려다 막혔어요. 승인하면 이 목표 대화에서만 허용하고, 같은 작업을 이어서 실행해요.{}\n{lines}", if pm_reason.is_empty() { String::new() } else { format!("\nPM 검토: {pm_reason}") }), status: "pending".into(), created_at: crate::model::now() };
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
        let mode = crate::permission_mode::effective(&store, goal_id)?;
        let mut auto_retried = false;
        for mut record in pending {
            if record.resumes >= permissions::MAX_RESUMES { continue; }
            let task: Task = store.get("task", &record.task_id)?.ok_or("권한을 요청한 업무가 없습니다")?;
            let auto: Vec<Rule> = record.denials.iter().filter_map(|denial| crate::permission_mode::eligible(&mode, denial, Some(&task))).filter(|rule| !record.auto_attempted.contains(&rule.key())).take(12).collect();
            if auto.is_empty() { continue; }
            add_task_auto_grants(&store, &task, &auto)?;
            let active = task_auto_grants(&store, &task)?;
            record.denials.retain(|denial| !permissions::covered(&active, &denial.tool, &denial.command));
            record.auto_attempted.extend(auto.iter().map(Rule::key));
            store.put("task_denial", &record.task_id, &record, None)?;
            if record.denials.is_empty() {
                resume_task(app, project, goal, &record, None, true)?;
                auto_retried = true;
            }
        }
        if auto_retried { continue; }
        let pending: Vec<TaskDenials> = wave_ids.iter().filter_map(|id| store.get::<TaskDenials>("task_denial", id).ok().flatten()).filter(|item| !item.denials.is_empty()).collect();
        if pending.is_empty() { return Ok(()); }
        let mut retried = false;
        for mut record in pending.iter().filter(|item| !item.pm_reviewed).cloned() {
            let decision = pm_review(app, project, goal, &record);
            let (action, reason) = match decision {
                Ok(decision) if decision.action != "request_approval" || decision.rule_keys.iter().all(|key| permissions::rules_for_denials(&record.denials).iter().any(|rule| rule.key() == *key && rule.class != "blocked")) => {
                    if decision.action == "request_approval" {
                        record.selected_rule_keys = decision.rule_keys;
                    }
                    (decision.action, decision.reason)
                }
                Ok(_) => ("retry_with_existing_grants".into(), "PM이 유효하지 않은 명령 규칙을 골라 승인 없이 다시 시도합니다".into()),
                Err(error) => ("retry_with_existing_grants".into(), format!("PM 검토 실패: {error}; 승인 없이 명령을 나눠 다시 시도합니다")),
            };
            record.pm_reviewed = true;
            record.pm_reason = reason.chars().take(300).collect();
            store.put("task_denial", &record.task_id, &record, Some(NewEvent { kind: "permission_pm_reviewed".into(), goal_id: Some(goal_id.into()), task_id: Some(record.task_id.clone()), agent_id: None, message: format!("{action}: {}", record.pm_reason) }))?;
            match action.as_str() {
                "continue_without" => clear(&store, &record)?,
                "retry_with_existing_grants" if record.resumes < permissions::MAX_RESUMES => {
                    resume_task(app, project, goal, &record, Some(&reason), false)?;
                    retried = true;
                }
                "retry_with_existing_grants" => clear(&store, &record)?,
                _ => {},
            }
        }
        if retried { continue; }
        let pending: Vec<TaskDenials> = wave_ids.iter().filter_map(|id| store.get::<TaskDenials>("task_denial", id).ok().flatten()).filter(|item| !item.denials.is_empty()).collect();
        if pending.is_empty() { return Ok(()); }
        let granted = grants(&store, goal_id)?;
        let denials: Vec<Denial> = pending.iter().flat_map(|item| item.denials.clone()).collect();
        let selected: Vec<String> = pending.iter().flat_map(|item| item.selected_rule_keys.clone()).collect();
        let rules: Vec<Rule> = permissions::rules_for_denials(&denials).into_iter().filter(|rule| selected.contains(&rule.key()) && !granted.iter().any(|item| item.key() == rule.key())).collect();
        let approved = if rules.iter().all(|rule| rule.class == "blocked") && !rules.is_empty() { false }
            // ADR-0063: nothing to ask; resume only when grants now cover every refused line
            // (a line of only read-only commands is never covered and is not retried).
            else if rules.is_empty() { pending.iter().all(|item| item.denials.iter().all(|denial| permissions::covered(&granted, &denial.tool, &denial.command))) }
            else {
                let snapshot = store.snapshot()?;
                let mut names: Vec<String> = pending.iter().filter_map(|item| snapshot.tasks.iter().find(|task| task.id == item.task_id)).filter_map(|task| snapshot.agents.iter().find(|agent| agent.id == task.assignee_id).map(|agent| agent.name.clone())).collect();
                names.dedup();
                let who = if names.is_empty() { "팀원".to_string() } else { names.join(", ") };
                let pm_reasons = pending.iter().map(|item| item.pm_reason.as_str()).filter(|reason| !reason.is_empty()).collect::<Vec<_>>().join(" / ");
                ask(app, goal_id, rules, pending.iter().map(|item| item.task_id.clone()).collect(), &who, "executing", &pm_reasons)?
            };
        if !approved { for record in &pending { clear(&store, record)?; } return Ok(()); }
        for record in pending {
            if record.resumes >= permissions::MAX_RESUMES { clear(&store, &record)?; continue; }
            resume_task(app, project, goal, &record, None, false)?;
        }
    }
}

fn resume_task<R: tauri::Runtime>(app: &tauri::AppHandle<R>, project: &Project, goal: &Goal, record: &TaskDenials, pm_guidance: Option<&str>, automatic: bool) -> Result<(), String> {
    let store = app.state::<Store>();
    let mut task: Task = store.get("task", &record.task_id)?.ok_or("업무 기록이 사라졌습니다")?;
    let snapshot = store.snapshot()?;
    let agent: Agent = snapshot.agents.iter().find(|agent| agent.id == task.assignee_id).cloned().ok_or("담당 팀원을 찾을 수 없습니다")?;
    let selected: AiConnection = connection(&snapshot.connections, &agent)?;
    let worktree = task.worktree.clone().ok_or("Missing worktree")?;
    let mut approved_rules = grants(&store, &goal.id)?;
    approved_rules.extend(task_auto_grants(&store, &task)?);
    let approved = approved_rules.iter().map(label).collect::<Vec<_>>().join(", ");
    let direction = match pm_guidance { Some(reason) => format!("The PM suggests a way to continue within EXISTING grants: {reason}. Do not retry refused commands without a CEO grant. Use only already approved commands and declared files."), None => "Run only the newly approved commands needed for this task as separate invocations; do not retry other refused commands. Finish the work.".into() };
    let prompt = format!("You are {} ({}). These commands are allowed for this task: {approved}. Continue your task in this isolated folder: {direction} Change ONLY these declared files: {}. Do not redo finished edits without reason. Task: {}\nInstructions: {}\nCEO goal: {}\nReturn a concise summary with command output as evidence.", agent.name, agent.role, task.files.join(", "), task.title, task.instructions, goal.intent);
    let options = options_for_task(&store, &task, task.session_id.clone())?;
    record_progress(app, &goal.id, Some(&task.id), &agent.id, if automatic { "자동 실행 모드에서 허용한 명령으로 이어서 하고 있습니다" } else if pm_guidance.is_some() { "PM 지시에 따라 기존 권한으로 작업을 이어서 하고 있습니다" } else { "승인된 명령으로 작업을 이어서 하고 있습니다" });
    match execute_tracked_with(app, &project.id, &goal.id, Some(&task.id), &agent.id, &selected, &prompt, Path::new(&worktree), true, &options) {
        Ok(result) => {
            task.result = format!("{}\n\n[{}]\n{}", task.result, if automatic { "자동 실행 모드로 이어서 실행" } else if pm_guidance.is_some() { "PM 검토 후 기존 권한으로 이어서 실행" } else { "명령 승인 후 이어서 실행" }, result.text).chars().rev().take(10_000).collect::<Vec<_>>().into_iter().rev().collect();
            if result.session_id.is_some() { task.session_id = result.session_id.clone(); }
            store.put("task", &task.id, &task, Some(NewEvent::task(if automatic { "task_resumed_auto" } else if pm_guidance.is_some() { "task_resumed_after_pm_review" } else { "task_resumed_with_commands" }, &task, if automatic { "Teammate continued with exact task-local command" } else if pm_guidance.is_some() { "Teammate continued within existing grants after PM review" } else { "Teammate continued with approved commands" })))?;
            record_task_denials(&store, &task, &result.permission_denials, record.resumes + 1)?;
            record_commands_run(&store, &task, &result.commands_run)?;
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
        let mut qa_auto = Vec::<Rule>::new();
        let mut qa_auto_attempted = false;
        for _ in 0..permissions::MAX_RESUMES {
            let granted = grants(&store, goal_id)?;
            let open: Vec<Denial> = response.permission_denials.iter().filter(|denial| !permissions::covered(&granted, &denial.tool, &denial.command)).cloned().collect();
            if open.is_empty() { break; }
            let _ = store.append_event(NewEvent { kind: "permission_denied".into(), goal_id: Some(goal_id.into()), task_id: None, agent_id: Some(reviewer.id.clone()), message: format!("QA refused: {}", open.iter().map(|item| item.command.clone()).collect::<Vec<_>>().join(" / ").chars().take(300).collect::<String>()) });
            let mode = crate::permission_mode::effective(&store, goal_id)?;
            let eligible: Vec<Rule> = open.iter().filter_map(|denial| crate::permission_mode::eligible(&mode, denial, None)).collect();
            if !qa_auto_attempted && eligible.len() == open.len() && !eligible.is_empty() {
                qa_auto_attempted = true;
                qa_auto.extend(eligible);
                let _ = store.append_event(NewEvent { kind: "command_auto_granted".into(), goal_id: Some(goal_id.into()), task_id: None, agent_id: Some(reviewer.id.clone()), message: "QA exact read commands in disposable copy".into() });
                let mut options = options_for(&store, goal_id, response.session_id.clone(), true)?;
                options.allowed.extend(permissions::cli_allowed(&qa_auto));
                let follow = format!("The app allowed these exact checks in this disposable review copy: {}. Run them and provide the complete review again.\nOriginal request:\n{prompt}", qa_auto.iter().map(label).collect::<Vec<_>>().join(", "));
                response = execute_tracked_with(app, &project.id, goal_id, None, &reviewer.id, selected, &follow, &copy, false, &options)?;
                continue;
            }
            let rules: Vec<Rule> = permissions::rules_for_denials(&open).into_iter().filter(|rule| !granted.iter().any(|item| item.key() == rule.key())).collect();
            if rules.is_empty() || rules.iter().all(|rule| rule.class == "blocked") || !ask(app, goal_id, rules, Vec::new(), &format!("QA({})", reviewer.name), "validating", "")? { break; }
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

#[cfg(test)]
mod pm_tests {
    use super::parse_pm_decision;

    #[test]
    fn pm_permission_review_accepts_only_bounded_decisions() {
        assert_eq!(parse_pm_decision(r#"{"action":"retry_with_existing_grants","reason":"승인된 파일만 다시 확인"}"#).unwrap().action, "retry_with_existing_grants");
        assert!(parse_pm_decision(r#"{"action":"grant_network","reason":"필요함"}"#).is_err());
        assert!(parse_pm_decision(r#"{"action":"request_approval","reason":""}"#).is_err());
        assert!(parse_pm_decision(r#"{"action":"request_approval","reason":"필요함"}"#).is_err());
        assert_eq!(parse_pm_decision(r#"{"action":"request_approval","reason":"필요함","rule_keys":["Bash(node --test)"]}"#).unwrap().rule_keys, vec!["Bash(node --test)"]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (Store, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("ucompany-modes-{}", crate::model::id()));
        std::fs::create_dir_all(&root).unwrap();
        (Store::open(&root.join("db.sqlite")).unwrap(), root)
    }

    fn goal(store: &Store, id: &str, project: &str, kind: &str) {
        let goal: Goal = serde_json::from_value(serde_json::json!({ "id": id, "kind": kind, "projectId": project, "intent": "x", "status": "awaiting_approval", "report": "", "createdAt": "t" })).unwrap();
        store.put("goal", id, &goal, None).unwrap();
    }

    #[test]
    fn safe_grants_reach_later_goals_of_the_same_project_only() {
        let (store, root) = store();
        goal(&store, "a", "p1", "execution");
        goal(&store, "b", "p1", "execution");
        goal(&store, "c", "p2", "execution");
        let rules = permissions::rules_for_denials(&[Denial { tool: "Bash".into(), command: "npm test && rm -rf dist".into() }]);
        add_grants(&store, "a", &rules).unwrap();
        let keys = |goal_id: &str| grants(&store, goal_id).unwrap().iter().map(Rule::key).collect::<Vec<_>>();
        assert_eq!(keys("a"), vec!["Bash(npm test)", "Bash(rm -rf)"], "the conversation keeps every rule it granted");
        assert_eq!(keys("b"), vec!["Bash(npm test)"], "only the safe rule reaches a later goal of the project");
        assert!(keys("c").is_empty(), "nothing reaches another project");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn approved_note_keeps_teammates_in_the_task_folder() {
        let (store, root) = store();
        goal(&store, "g", "p1", "execution");
        let note = approved_note(&store, "g").unwrap();
        assert!(note.contains("do not change directory outside it") && note.contains("rather than `npx`"));
        let _ = std::fs::remove_dir_all(root);
    }
}
