//! ADR-0066: user-selected command convenience modes. A mode never grants a new file scope.

use crate::model::{Goal, Project, Task};
use crate::permissions::{Denial, Rule};
use crate::storage::{NewEvent, Store};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionModeSetting { pub scope: String, pub id: String, pub mode: String, pub updated_at: String }

pub const MANUAL: &str = "manual";
pub const AUTO: &str = "auto";
pub const FULL_AUTO: &str = "full_auto";

fn key(scope: &str, id: &str) -> String { format!("{scope}:{id}") }

pub fn set(store: &Store, scope: &str, id: &str, mode: &str) -> Result<PermissionModeSetting, String> {
    if ![MANUAL, AUTO, FULL_AUTO].contains(&mode) && !(scope == "conversation" && mode == "inherit") { return Err("알 수 없는 실행 권한 모드입니다".into()); }
    let (actual_id, goal_id) = match scope {
        "project" if store.list::<Project>("project_entry")?.iter().any(|item| item.id == id) => (id.to_string(), None),
        "conversation" => {
            let goal: Goal = store.get("goal", id)?.ok_or("목표를 찾을 수 없습니다")?;
            if goal.kind != "execution" { return Err("실행 목표 대화에서만 명령 모드를 바꿀 수 있습니다".into()); }
            (crate::permissions_flow::conversation(store, id)?, Some(goal.id))
        }
        _ => return Err("권한 모드 대상을 찾을 수 없습니다".into()),
    };
    let setting = PermissionModeSetting { scope: scope.into(), id: actual_id.clone(), mode: mode.into(), updated_at: crate::model::now() };
    store.put("permission_mode", &key(scope, &actual_id), &setting, Some(NewEvent { kind: "permission_mode_changed".into(), goal_id, task_id: None, agent_id: None, message: format!("{scope}: {mode}") }))?;
    Ok(setting)
}

pub fn effective(store: &Store, goal_id: &str) -> Result<String, String> {
    let goal: Goal = store.get("goal", goal_id)?.ok_or("목표를 찾을 수 없습니다")?;
    let conversation = crate::permissions_flow::conversation(store, goal_id)?;
    let override_mode = store.get::<PermissionModeSetting>("permission_mode", &key("conversation", &conversation))?;
    if let Some(setting) = override_mode { if setting.mode != "inherit" { return Ok(setting.mode); } }
    let project_id = goal.project_id.ok_or("목표의 프로젝트가 없습니다")?;
    Ok(store.get::<PermissionModeSetting>("permission_mode", &key("project", &project_id))?.map(|item| item.mode).unwrap_or_else(|| MANUAL.into()))
}

fn simple_shell(command: &str) -> bool {
    !command.is_empty() && command.len() <= 400 && !command.chars().any(|c| "\r\n;|&$`<>(){}%!".contains(c))
}

fn local_args(command: &str) -> bool {
    crate::permissions::words(command).iter().skip(1).all(|item| {
        !item.starts_with('/') && !item.starts_with('~') && !item.starts_with("..") && !item.contains("../") && !item.contains("..\\") && !item.contains(':') && !item.contains('\\')
    })
}

fn no_symlink_args(command: &str, task: &Task) -> bool {
    let Some(root) = task.worktree.as_ref() else { return false; };
    crate::permissions::words(command).iter().skip(1).filter(|item| !item.starts_with('-')).all(|item| {
        let mut current = Path::new(root).to_path_buf();
        for part in item.split('/') {
            current.push(part);
            if std::fs::symlink_metadata(&current).is_ok_and(|meta| meta.file_type().is_symlink()) { return false; }
        }
        true
    })
}

fn routine(command: &str) -> bool {
    if !simple_shell(command) || !local_args(command) { return false; }
    if crate::permissions::words(command).iter().any(|word| ["-L", "-R", "--follow", "--dereference", "--dereference-recursive"].contains(&word.as_str())) { return false; }
    let mut words = command.split_whitespace();
    match words.next().unwrap_or("").to_ascii_lowercase().as_str() {
        "rg" => !command.contains("--pre") && !command.contains("--files-with-matches --null-data"),
        "grep" | "cat" | "ls" | "pwd" | "echo" | "wc" | "head" | "tail" | "uniq" => true,
        "git" => words.next().is_some_and(|item| item == "status"),
        _ => false,
    }
}

fn declared_path(path: &str, task: &Task) -> bool {
    let normalized = path.trim_start_matches("./").replace('\\', "/");
    !normalized.is_empty() && !normalized.starts_with('/') && !normalized.contains(':') && !normalized.split('/').any(|part| part.is_empty() || part == "." || part == "..")
        && task.files.iter().any(|file| file.eq_ignore_ascii_case(&normalized))
        && task.worktree.as_ref().is_some_and(|root| {
            let mut current = Path::new(root).to_path_buf();
            for part in normalized.split('/') {
                current.push(part);
                if std::fs::symlink_metadata(&current).is_ok_and(|meta| meta.file_type().is_symlink()) { return false; }
            }
            true
        })
}

fn destructive(command: &str, task: &Task) -> bool {
    if !simple_shell(command) || !local_args(command) || command.contains(['*', '?', '[', ']']) || command.contains(['\'', '"']) { return false; }
    let words: Vec<&str> = command.split_whitespace().collect();
    match words.as_slice() {
        ["rm" | "del" | "erase", path] => declared_path(path, task),
        ["rm", "-f", path] => declared_path(path, task),
        ["mv" | "move", from, to] => declared_path(from, task) && declared_path(to, task),
        _ => false,
    }
}

/// Exact, task-local provider rule. Unknown shell shapes and all network/install/publish tools stay manual.
pub fn eligible(mode: &str, denial: &Denial, task: Option<&Task>) -> Option<Rule> {
    if ![AUTO, FULL_AUTO].contains(&mode) || !["Bash", "PowerShell"].contains(&denial.tool.as_str()) { return None; }
    let command = denial.command.trim();
    if !(routine(command) && task.is_none_or(|task| no_symlink_args(command, task))) && !(mode == FULL_AUTO && task.is_some_and(|task| destructive(command, task))) { return None; }
    Some(Rule { tool: denial.tool.clone(), prefix: command.into(), class: "safe".into(), reason: "자동 실행 모드의 정확한 명령".into(), exact: true })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_only_accepts_simple_local_read_calls() {
        let denial = |command: &str| Denial { tool: "Bash".into(), command: command.into() };
        assert!(eligible(AUTO, &denial("grep -n TODO report.md"), None).is_some());
        assert!(eligible(AUTO, &denial("echo \"x $(curl https://example.com)\""), None).is_none());
        assert!(eligible(AUTO, &denial("cat ../secret"), None).is_none());
        assert!(eligible(AUTO, &denial("curl https://example.com"), None).is_none());
        assert!(eligible(FULL_AUTO, &denial("rm -rf report.md"), None).is_none());
    }

    #[test]
    fn full_auto_is_exact_and_confined_to_declared_files() {
        let task = Task { id: "t".into(), goal_id: "g".into(), title: "edit".into(), instructions: String::new(), files: vec!["report/a.md".into(), "report/b.md".into()], assignee_id: "dev".into(), status: "running".into(), depends_on: vec![], worktree: Some("C:/isolated-task".into()), result: String::new(), session_id: None, deliverable: String::new(), acceptance: vec![], evidence: vec![], capability: String::new(), assignment_reason: String::new() };
        let denial = |command: &str| Denial { tool: "Bash".into(), command: command.into() };
        let rule = eligible(FULL_AUTO, &denial("rm report/a.md"), Some(&task)).expect("declared file");
        assert!(rule.exact);
        assert_eq!(rule.cli_rules(), vec!["Bash(rm report/a.md)"]);
        assert!(crate::permissions::covered(&[rule.clone()], "Bash", "rm report/a.md"));
        assert!(!crate::permissions::covered(&[rule], "Bash", "rm report/a.md report/b.md"));
        assert!(eligible(FULL_AUTO, &denial("mv report/a.md report/b.md"), Some(&task)).is_some());
        for command in ["rm ../other.md", "rm report/c.md", "rm -rf report", "rm report/a.md; curl https://example.com", "mv report/a.md ../outside.md"] {
            assert!(eligible(FULL_AUTO, &denial(command), Some(&task)).is_none(), "{command}");
        }
        assert!(eligible(AUTO, &denial("rm report/a.md"), Some(&task)).is_none());
    }
}
