//! Goal archive ("목표 보관") and permanent deletion from the backup dialog (ADR-0043).
//!
//! Archiving hides an inactive goal and everything that belongs to it from every screen without
//! deleting data; it can be undone. Permanent deletion removes the goal's records, its events and its
//! isolated workspaces, only for an archived goal, only after an explicit confirmation, and never a
//! goal another visible goal still depends on. Token usage records are kept so project totals stay
//! truthful; they hold counts, not content.
use crate::model::{Agent, Goal, GoalGuidance, GoalLineage, Meeting, Task};
use crate::storage::{tx_event, tx_get, tx_list, tx_put, Store};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

/// The archive marker (kind `goal_archive`, id = goal id).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GoalArchive {
    pub goal_id: String,
    pub archived_at: String,
}

/// What the backup dialog shows: a summary only (UXP-1).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedGoal {
    pub goal_id: String,
    pub kind: String,
    pub title: String,
    pub status: String,
    pub created_at: String,
    pub archived_at: String,
}

/// Object kinds that belong to a goal through `goalId`, `replacementGoalId` or `meetingId`.
const OWNED_KINDS: [&str; 12] = ["task", "task_proof", "approval", "goal_turn", "goal_guidance", "deliverable_review", "meeting", "meeting_packet", "meeting_contribution", "meeting_rejected_output", "transmission_approval", "goal_lineage"];
/// Object kinds keyed by the goal id itself.
const KEYED_KINDS: [&str; 6] = ["goal", "goal_analysis", "goal_summary", "goal_origin", "squad", "goal_archive"];

pub fn archived_ids(tx: &rusqlite::Transaction<'_>) -> Result<BTreeSet<String>, String> {
    Ok(tx_list::<GoalArchive>(tx, "goal_archive")?.into_iter().map(|item| item.goal_id).collect())
}

/// A goal may be archived only when nothing is running or waiting on it.
fn archivable(goal: &Goal, tasks: &[Task], meeting: Option<&Meeting>) -> Result<(), String> {
    if goal.kind == "deliberation" {
        let stage = meeting.map(|item| item.stage.as_str()).unwrap_or("decision_recorded");
        if goal.status == "completed" || ["decision_recorded", "legacy_closed", "blocked", "stopped"].contains(&stage) { return Ok(()); }
        return Err("진행 중인 회의는 보관할 수 없습니다. 회의를 중단하거나 결정을 기록한 뒤 보관해 주세요".into());
    }
    if !["new", "completed", "blocked", "cancelled"].contains(&goal.status.as_str()) { return Err("진행 중이거나 승인을 기다리는 목표는 보관할 수 없습니다. 끝나거나 멈춘 뒤 보관해 주세요".into()); }
    if tasks.iter().any(|task| task.goal_id == goal.id && ["running", "awaiting_integration"].contains(&task.status.as_str())) { return Err("실행 중인 업무가 있어 보관할 수 없습니다".into()); }
    Ok(())
}

/// ADR-0051: the versions of one goal conversation (a goal and the revisions that replaced it), oldest
/// first, limited to those `keep` accepts. The screen shows them as one conversation, so they are
/// archived and restored together.
fn version_chain(tx: &rusqlite::Transaction<'_>, goal_id: &str, keep: impl Fn(&str) -> bool) -> Result<Vec<String>, String> {
    let links: Vec<(String, String)> = tx_list::<GoalGuidance>(tx, "goal_guidance")?.into_iter()
        .filter(|item| item.status == "revised")
        .filter_map(|item| item.replacement_goal_id.map(|next| (item.goal_id, next)))
        .collect();
    let mut chain = vec![goal_id.to_string()];
    let mut current = goal_id.to_string();
    while let Some(previous) = links.iter().find(|(_, next)| *next == current).map(|(from, _)| from.clone()) {
        if chain.contains(&previous) || !keep(&previous) || tx_get::<Goal>(tx, "goal", &previous)?.is_none() { break; }
        chain.insert(0, previous.clone());
        current = previous;
    }
    current = goal_id.to_string();
    while let Some(next) = links.iter().find(|(from, _)| *from == current).map(|(_, next)| next.clone()) {
        if chain.contains(&next) || !keep(&next) || tx_get::<Goal>(tx, "goal", &next)?.is_none() { break; }
        chain.push(next.clone());
        current = next;
    }
    Ok(chain)
}

pub fn archive(store: &Store, goal_id: &str) -> Result<(), String> {
    store.transact(|tx| {
        let archived = archived_ids(tx)?;
        if archived.contains(goal_id) { return Err("이미 보관한 목표입니다".into()); }
        tx_get::<Goal>(tx, "goal", goal_id)?.ok_or("목표를 찾을 수 없습니다")?;
        let chain = version_chain(tx, goal_id, |id| !archived.contains(id))?;
        let tasks: Vec<Task> = tx_list(tx, "task")?;
        let meetings: Vec<Meeting> = tx_list(tx, "meeting")?;
        // Every version must be archivable before any is archived.
        for id in &chain {
            let goal: Goal = tx_get(tx, "goal", id)?.ok_or("목표를 찾을 수 없습니다")?;
            archivable(&goal, &tasks, meetings.iter().find(|item| &item.goal_id == id)).map_err(|error| if id == goal_id { error } else { format!("같은 대화의 다른 버전 때문에 보관할 수 없습니다: {error}") })?;
        }
        for id in &chain {
            // External staff contracted to this goal would otherwise stay busy forever (ADR-0041).
            for mut agent in tx_list::<Agent>(tx, "agent")?.into_iter().filter(|agent| agent.is_external() && agent.external.as_ref().is_some_and(|item| item.status == "contracted" && item.goal_id.as_deref() == Some(id.as_str()))) {
                agent.enabled = false;
                if let Some(contract) = agent.external.as_mut() { contract.status = "released".into(); contract.released_at = Some(crate::model::now()); }
                tx_put(tx, "agent", &agent.id.clone(), &agent)?;
            }
            tx_put(tx, "goal_archive", id, &GoalArchive { goal_id: id.clone(), archived_at: crate::model::now() })?;
            tx_event(tx, "goal_archived", id, if id == goal_id { "CEO archived the goal; hidden from screens, data kept" } else { "Archived with another version of the same goal conversation" })?;
        }
        Ok(())
    })
}

pub fn restore(store: &Store, goal_id: &str) -> Result<(), String> {
    store.transact(|tx| {
        let archived = archived_ids(tx)?;
        if !archived.contains(goal_id) { return Err("보관함에 없는 목표입니다".into()); }
        for id in version_chain(tx, goal_id, |id| archived.contains(id))? {
            tx.execute("DELETE FROM objects WHERE kind='goal_archive' AND id=?1", [&id]).map_err(|e| e.to_string())?;
            tx_event(tx, "goal_restored", &id, if id == goal_id { "CEO restored the goal from the archive" } else { "Restored with another version of the same goal conversation" })?;
        }
        Ok(())
    })
}

/// Visible goals that still depend on `goal_id` (a revision built on it, or an execution goal drafted
/// from its meeting). Permanent deletion is refused while any exists.
fn dependants(tx: &rusqlite::Transaction<'_>, goal_id: &str, archived: &BTreeSet<String>) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for guidance in tx_list::<GoalGuidance>(tx, "goal_guidance")? {
        if guidance.goal_id == goal_id { if let Some(next) = guidance.replacement_goal_id.filter(|id| !archived.contains(id)) { out.push(next); } }
    }
    let meetings: BTreeSet<String> = tx_list::<Meeting>(tx, "meeting")?.into_iter().filter(|item| item.goal_id == goal_id).map(|item| item.id).collect();
    for lineage in tx_list::<GoalLineage>(tx, "goal_lineage")? {
        if meetings.contains(&lineage.meeting_id) && lineage.goal_id != goal_id && !archived.contains(&lineage.goal_id) { out.push(lineage.goal_id); }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// Rows (kind, id, bytes) that belong to an archived goal, and its event count and bytes.
fn owned_rows(tx: &rusqlite::Transaction<'_>, goal_id: &str) -> Result<(Vec<(String, String, usize)>, usize, usize), String> {
    let meeting_ids: BTreeSet<String> = tx_list::<Meeting>(tx, "meeting")?.into_iter().filter(|item| item.goal_id == goal_id).map(|item| item.id).collect();
    let mut rows = Vec::new();
    let mut stmt = tx.prepare("SELECT kind,id,payload FROM objects").map_err(|e| e.to_string())?;
    let all = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))).map_err(|e| e.to_string())?;
    for row in all {
        let (kind, id, payload) = row.map_err(|e| e.to_string())?;
        let owned = if KEYED_KINDS.contains(&kind.as_str()) { id == goal_id } else if OWNED_KINDS.contains(&kind.as_str()) {
            let value: Value = serde_json::from_str(&payload).unwrap_or(Value::Null);
            let field = |name: &str| value.get(name).and_then(Value::as_str).map(str::to_string);
            field("goalId").as_deref() == Some(goal_id) || field("replacementGoalId").as_deref() == Some(goal_id) || field("meetingId").is_some_and(|id| meeting_ids.contains(&id)) || (kind == "meeting" && meeting_ids.contains(&id))
        } else { false };
        if owned { rows.push((kind, id, payload.len())); }
    }
    let (events, event_bytes): (i64, i64) = tx.query_row("SELECT COUNT(*), COALESCE(SUM(LENGTH(message)),0) FROM events WHERE goal_id=?1", [goal_id], |r| Ok((r.get(0)?, r.get(1)?))).map_err(|e| e.to_string())?;
    Ok((rows, events as usize, event_bytes as usize))
}

fn dir_bytes(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else { return 0 };
    if meta.file_type().is_symlink() { return 0; }
    if meta.is_file() { return meta.len(); }
    std::fs::read_dir(path).map(|entries| entries.flatten().map(|entry| dir_bytes(&entry.path())).sum()).unwrap_or(0)
}

/// Approximate space each archived goal uses: its records, events and isolated workspaces.
pub fn usage(store: &Store, worktrees: &Path) -> Result<Vec<(String, u64)>, String> {
    let ids: Vec<String> = store.transact(|tx| Ok(archived_ids(tx)?.into_iter().collect()))?;
    let mut out = Vec::new();
    for id in ids {
        let (rows, _, event_bytes) = store.transact(|tx| owned_rows(tx, &id))?;
        let bytes = rows.iter().map(|row| row.2 as u64).sum::<u64>() + event_bytes as u64 + dir_bytes(&worktrees.join(&id));
        out.push((id, bytes));
    }
    Ok(out)
}

/// Permanent deletion of an archived goal: records, events and isolated workspaces. Requires the
/// explicit confirmation flag and refuses while a visible goal depends on it.
pub fn purge(store: &Store, goal_id: &str, confirmed: bool, worktrees: &Path) -> Result<usize, String> {
    if !confirmed { return Err("영구 삭제는 되돌릴 수 없습니다. 확인을 선택한 뒤 다시 눌러 주세요".into()); }
    if goal_id.is_empty() || goal_id.contains(['/', '\\', '.']) { return Err("목표 ID가 올바르지 않습니다".into()); }
    let removed = store.transact(|tx| {
        let archived = archived_ids(tx)?;
        if !archived.contains(goal_id) { return Err("보관함에 있는 목표만 영구 삭제할 수 있습니다".into()); }
        let blockers = dependants(tx, goal_id, &archived)?;
        if !blockers.is_empty() { return Err(format!("이 목표를 바탕으로 한 다른 목표 {}개가 화면에 남아 있어 영구 삭제할 수 없습니다. 그 목표를 먼저 보관해 주세요", blockers.len())); }
        let (rows, events, _) = owned_rows(tx, goal_id)?;
        for (kind, id, _) in &rows { tx.execute("DELETE FROM objects WHERE kind=?1 AND id=?2", params![kind, id]).map_err(|e| e.to_string())?; }
        tx.execute("DELETE FROM events WHERE goal_id=?1", [goal_id]).map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO events(at,kind,goal_id,task_id,agent_id,message) VALUES(?1,'goal_purged',NULL,NULL,NULL,?2)", params![crate::model::now(), format!("CEO permanently deleted an archived goal ({} records, {} events)", rows.len(), events)]).map_err(|e| e.to_string())?;
        Ok(rows.len())
    })?;
    // Workspaces live only under the engine's own worktree folder; never follow a link out of it.
    let folder = worktrees.join(goal_id);
    if let Ok(meta) = std::fs::symlink_metadata(&folder) {
        if meta.is_dir() && !meta.file_type().is_symlink() { let _ = std::fs::remove_dir_all(&folder); }
    }
    Ok(removed)
}

/// Summary rows for the snapshot (title only: AI summary, meeting topic summary or an excerpt).
pub fn summaries(goals: &[Goal], archives: &[GoalArchive], meetings: &[Meeting], goal_summaries: &[crate::model::GoalSummary]) -> Vec<ArchivedGoal> {
    let mut out: Vec<ArchivedGoal> = archives.iter().filter_map(|archive| {
        let goal = goals.iter().find(|goal| goal.id == archive.goal_id)?;
        let ai = if goal.kind == "deliberation" { meetings.iter().find(|item| item.goal_id == goal.id).map(|item| item.topic_summary.clone()) } else { goal_summaries.iter().find(|item| item.goal_id == goal.id).map(|item| item.text.clone()) };
        let title = ai.filter(|text| !text.trim().is_empty()).unwrap_or_else(|| { let text: String = goal.intent.split_whitespace().collect::<Vec<_>>().join(" "); if text.chars().count() > 80 { format!("{}…", text.chars().take(79).collect::<String>()) } else { text } });
        Some(ArchivedGoal { goal_id: goal.id.clone(), kind: goal.kind.clone(), title, status: goal.status.clone(), created_at: goal.created_at.clone(), archived_at: archive.archived_at.clone() })
    }).collect();
    out.sort_by(|a, b| b.archived_at.cmp(&a.archived_at));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn store() -> (Store, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("utopia-archive-{}", crate::model::id()));
        std::fs::create_dir_all(&root).unwrap();
        (Store::open(&root.join("db.sqlite")).unwrap(), root)
    }
    fn goal(id: &str, status: &str) -> Goal { serde_json::from_value(json!({"id": id, "kind": "execution", "intent": format!("{id} 목표 원문"), "status": status, "report": "", "createdAt": "t"})).unwrap() }
    fn task(id: &str, goal_id: &str, status: &str) -> Task { serde_json::from_value(json!({"id": id, "goalId": goal_id, "title": id, "instructions": "", "files": [], "assigneeId": "developer-a", "status": status, "dependsOn": [], "result": ""})).unwrap() }

    #[test]
    fn archive_hides_restores_and_refuses_active_goals() {
        let (store, _) = store();
        store.put("goal", "done", &goal("done", "completed"), None).unwrap();
        store.put("goal", "busy", &goal("busy", "executing"), None).unwrap();
        store.put("goal", "stuck", &goal("stuck", "blocked"), None).unwrap();
        store.put("task", "t1", &task("t1", "stuck", "running"), None).unwrap();
        store.put("task", "t2", &task("t2", "done", "completed"), None).unwrap();
        assert!(archive(&store, "busy").unwrap_err().contains("진행 중"));
        assert!(archive(&store, "stuck").unwrap_err().contains("실행 중인 업무"));
        archive(&store, "done").unwrap();
        let snapshot = store.snapshot().unwrap();
        assert!(snapshot.goals.iter().all(|goal| goal.id != "done"), "hidden from every screen");
        assert!(snapshot.tasks.iter().all(|task| task.goal_id != "done"));
        assert_eq!(snapshot.archived_goals.len(), 1);
        assert_eq!(snapshot.archived_goals[0].title, "done 목표 원문");
        assert!(store.get::<Goal>("goal", "done").unwrap().is_some(), "data kept");
        restore(&store, "done").unwrap();
        assert!(store.snapshot().unwrap().goals.iter().any(|goal| goal.id == "done"));
    }

    #[test]
    fn the_versions_of_one_conversation_are_archived_and_restored_together() {
        let (store, _) = store();
        store.put("goal", "v1", &goal("v1", "blocked"), None).unwrap();
        store.put("goal", "v2", &goal("v2", "blocked"), None).unwrap();
        store.put("goal", "v3", &goal("v3", "executing"), None).unwrap();
        store.put("goal", "solo", &goal("solo", "completed"), None).unwrap();
        store.put("goal_guidance", "g1", &json!({"id": "g1", "goalId": "v1", "text": "x", "status": "revised", "createdAt": "t", "replacementGoalId": "v2"}), None).unwrap();
        store.put("goal_guidance", "g2", &json!({"id": "g2", "goalId": "v2", "text": "x", "status": "revised", "createdAt": "t", "replacementGoalId": "v3"}), None).unwrap();
        assert!(archive(&store, "v2").unwrap_err().contains("다른 버전"), "a running version keeps the whole conversation");
        assert!(store.snapshot().unwrap().archived_goals.is_empty(), "nothing archived on refusal");
        let mut v3 = goal("v3", "completed");
        v3.id = "v3".into();
        store.put("goal", "v3", &v3, None).unwrap();
        archive(&store, "v3").unwrap();
        let hidden: Vec<String> = store.snapshot().unwrap().archived_goals.into_iter().map(|item| item.goal_id).collect();
        assert_eq!(hidden.len(), 3, "{hidden:?}");
        assert!(store.snapshot().unwrap().goals.iter().any(|item| item.id == "solo"));
        restore(&store, "v1").unwrap();
        assert!(store.snapshot().unwrap().archived_goals.is_empty(), "restored together");
    }

    #[test]
    fn purge_needs_archive_and_confirmation_and_keeps_usage_and_dependants() {
        let (store, root) = store();
        let worktrees = root.join("worktrees");
        std::fs::create_dir_all(worktrees.join("old").join("t1")).unwrap();
        std::fs::write(worktrees.join("old").join("t1").join("a.md"), "x".repeat(100)).unwrap();
        store.put("goal", "old", &goal("old", "completed"), Some(crate::storage::NewEvent { kind: "goal_stage".into(), goal_id: Some("old".into()), task_id: None, agent_id: None, message: "m".into() })).unwrap();
        store.put("task", "t1", &task("t1", "old", "completed"), None).unwrap();
        store.put("goal_analysis", "old", &"분석".to_string(), None).unwrap();
        store.put("usage", "u1", &json!({"id": "u1", "projectId": "p", "goalId": "old", "agentId": "cto", "taskId": null, "inputTokens": 5, "outputTokens": 1, "source": "reported", "status": "completed", "updatedAt": "t"}), None).unwrap();
        store.put("goal", "keep", &goal("keep", "completed"), None).unwrap();
        store.put("task", "t9", &task("t9", "keep", "completed"), None).unwrap();
        assert!(purge(&store, "old", true, &worktrees).unwrap_err().contains("보관함"));
        archive(&store, "old").unwrap();
        assert!(purge(&store, "old", false, &worktrees).unwrap_err().contains("되돌릴 수 없습니다"));
        let sizes = usage(&store, &worktrees).unwrap();
        assert!(sizes[0].1 >= 100, "{sizes:?}");
        // A visible revision built on the archived goal blocks deletion.
        store.put("goal_guidance", "g1", &json!({"id": "g1", "goalId": "old", "text": "x", "status": "applied", "createdAt": "t", "replacementGoalId": "keep"}), None).unwrap();
        assert!(purge(&store, "old", true, &worktrees).unwrap_err().contains("다른 목표"));
        archive(&store, "keep").unwrap();
        let removed = purge(&store, "old", true, &worktrees).unwrap();
        assert!(removed >= 4, "goal, task, analysis, archive, guidance: {removed}");
        assert!(store.get::<Goal>("goal", "old").unwrap().is_none());
        assert!(store.get::<Task>("task", "t1").unwrap().is_none());
        assert!(store.get::<Task>("task", "t9").unwrap().is_some(), "other goals untouched");
        assert!(store.get::<Value>("usage", "u1").unwrap().is_some(), "token counts kept");
        assert!(!worktrees.join("old").exists());
        assert!(store.snapshot().unwrap().archived_goals.iter().all(|item| item.goal_id != "old"));
    }
}
