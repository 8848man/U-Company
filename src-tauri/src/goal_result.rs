//! ADR-0054: what a finished goal left in the project, and where goals of a project stop. Both are
//! read from saved records and the project folder only; nothing here calls an AI or changes a file.

use crate::model::{Goal, Project, Task};
use crate::storage::Store;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResultFile { pub path: String, pub exists: bool }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GoalResult { pub folder: String, pub files: Vec<ResultFile>, pub hints: Vec<String> }

/// One saved block of a goal: the kind the engine recorded at the blocking site.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockRecord { pub goal_id: String, pub project_id: String, pub kind: String, pub at: String }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BlockStat { pub kind: String, pub label: String, pub count: usize, pub last_at: String }

/// Files the goal's completed tasks were approved to change, in plan order, and whether each exists now.
pub fn changed_files(tasks: &[Task], goal_id: &str, root: &Path) -> Vec<ResultFile> {
    let mut seen = BTreeSet::new();
    tasks.iter().filter(|task| task.goal_id == goal_id && task.status == "completed")
        .flat_map(|task| task.files.iter())
        .filter(|path| seen.insert(path.to_ascii_lowercase()))
        .map(|path| ResultFile { path: path.clone(), exists: root.join(path).is_file() })
        .collect()
}

/// Plain-language ways to try the result, from files that exist in the project.
pub fn run_hints(root: &Path, files: &[ResultFile]) -> Vec<String> {
    let mut hints = Vec::new();
    let html = files.iter().find(|file| file.exists && file.path.to_ascii_lowercase().ends_with("index.html")).map(|file| file.path.clone())
        .or_else(|| root.join("index.html").is_file().then(|| "index.html".to_string()));
    if let Some(page) = html { hints.push(format!("{page}을(를) 더블클릭하면 브라우저에서 바로 열려요")); }
    let scripts: Vec<String> = std::fs::read_to_string(root.join("package.json")).ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| value.get("scripts").and_then(|scripts| scripts.as_object()).map(|map| map.keys().cloned().collect()))
        .unwrap_or_default();
    let has = |name: &str| scripts.iter().any(|item| item == name);
    if has("dev") { hints.push("터미널에서 `npm install` 후 `npm run dev`로 실행해요".into()); }
    else if has("start") { hints.push("터미널에서 `npm install` 후 `npm start`로 실행해요".into()); }
    if has("test") { hints.push("`npm test`로 테스트를 돌려 볼 수 있어요".into()); }
    else if files.iter().any(|file| file.exists && [".test.js", ".test.mjs", ".test.cjs"].iter().any(|suffix| file.path.to_ascii_lowercase().ends_with(suffix))) {
        hints.push("프로젝트 폴더에서 `node --test`로 테스트를 돌려 볼 수 있어요".into());
    }
    if root.join("Cargo.toml").is_file() { hints.push("`cargo run`으로 실행하고 `cargo test`로 테스트해요".into()); }
    if hints.is_empty() { hints.push("폴더를 열어 바뀐 파일을 확인해 보세요".into()); }
    hints
}

pub fn result(store: &Store, goal_id: &str) -> Result<GoalResult, String> {
    let goal: Goal = store.get("goal", goal_id)?.ok_or("목표를 찾을 수 없습니다")?;
    let project_id = goal.project_id.clone().ok_or("프로젝트에 묶이지 않은 목표입니다")?;
    let project: Project = store.get("project_entry", &project_id)?.ok_or("프로젝트를 찾을 수 없습니다")?;
    let root = Path::new(&project.path);
    let files = changed_files(&store.list::<Task>("task")?, goal_id, root);
    let hints = run_hints(root, &files);
    Ok(GoalResult { folder: project.path.clone(), files, hints })
}

pub fn block_label(kind: &str) -> &'static str {
    match kind {
        "precall" => "AI 연결이나 팀원 확인에서 멈춤",
        "approval_denied" => "CEO가 승인을 거절함",
        "interrupted" => "앱이 꺼져 중단됨",
        "integration_conflict" => "결과를 프로젝트에 합치지 못함",
        "qa_failed" => "다시 고치기 횟수를 다 씀",
        "needs_scope" => "새 파일 승인이 필요함",
        "qa_form" => "QA 판정 형식이 틀림",
        "plan_form" => "계획 형식이 두 번 틀림",
        "provider" => "AI 호출이 실패함",
        _ => "기타 이유로 멈춤",
    }
}

/// How often goals of this project stopped, by kind, most frequent first.
pub fn block_stats(records: &[BlockRecord], project_id: &str) -> Vec<BlockStat> {
    let mut by_kind: BTreeMap<String, (usize, String)> = BTreeMap::new();
    for record in records.iter().filter(|record| record.project_id == project_id) {
        let entry = by_kind.entry(record.kind.clone()).or_insert((0, String::new()));
        entry.0 += 1;
        if record.at > entry.1 { entry.1 = record.at.clone(); }
    }
    let mut stats: Vec<BlockStat> = by_kind.into_iter().map(|(kind, (count, last_at))| BlockStat { label: block_label(&kind).into(), kind, count, last_at }).collect();
    stats.sort_by(|a, b| b.count.cmp(&a.count).then(b.last_at.cmp(&a.last_at)));
    stats
}

/// Opens the project folder in the system file manager. The path comes from the saved project, never
/// from the screen.
pub fn open_folder(path: &str) -> Result<(), String> {
    if !Path::new(path).is_dir() { return Err(format!("폴더를 찾을 수 없어요: {path}")); }
    let mut command = if cfg!(windows) { let mut c = std::process::Command::new("explorer.exe"); c.arg(path); c }
        else if cfg!(target_os = "macos") { let mut c = std::process::Command::new("open"); c.arg(path); c }
        else { let mut c = std::process::Command::new("xdg-open"); c.arg(path); c };
    command.spawn().map(|_| ()).map_err(|e| format!("폴더를 열지 못했어요: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(goal: &str, status: &str, files: &[&str]) -> Task {
        serde_json::from_value(serde_json::json!({ "id": crate::model::id(), "goalId": goal, "title": "t", "instructions": "i", "assigneeId": "dev", "status": status, "files": files, "dependsOn": [], "result": "" })).unwrap()
    }

    #[test]
    fn result_lists_completed_task_files_once_and_suggests_how_to_try_them() {
        let root = std::env::temp_dir().join(format!("u-result-{}", crate::model::id()));
        std::fs::create_dir_all(&root).unwrap();
        for file in ["index.html", "todo.js", "todo.test.js"] { std::fs::write(root.join(file), "x").unwrap(); }
        let tasks = vec![task("g", "completed", &["todo.js", "todo.test.js"]), task("g", "completed", &["index.html", "TODO.js"]), task("g", "failed", &["broken.js"]), task("other", "completed", &["x.js"])];
        let files = changed_files(&tasks, "g", &root);
        assert_eq!(files.iter().map(|file| file.path.as_str()).collect::<Vec<_>>(), vec!["todo.js", "todo.test.js", "index.html"]);
        let hints = run_hints(&root, &files);
        assert!(hints[0].contains("index.html") && hints[0].contains("브라우저"));
        assert!(hints.iter().any(|hint| hint.contains("node --test")));
        std::fs::write(root.join("package.json"), r#"{"scripts":{"dev":"vite","test":"vitest"}}"#).unwrap();
        let hints = run_hints(&root, &files);
        assert!(hints.iter().any(|hint| hint.contains("npm run dev")) && hints.iter().any(|hint| hint.contains("npm test")));
        assert!(!hints.iter().any(|hint| hint.contains("node --test")), "the project's own test script wins");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn block_stats_count_this_projects_blocks_by_kind_most_frequent_first() {
        let record = |project: &str, kind: &str, at: &str| BlockRecord { goal_id: "g".into(), project_id: project.into(), kind: kind.into(), at: at.into() };
        let stats = block_stats(&[record("p", "plan_form", "1"), record("p", "provider", "2"), record("p", "plan_form", "3"), record("q", "provider", "4")], "p");
        assert_eq!(stats.iter().map(|stat| (stat.kind.as_str(), stat.count, stat.last_at.as_str())).collect::<Vec<_>>(), vec![("plan_form", 2, "3"), ("provider", 1, "2")]);
        assert_eq!(stats[0].label, "계획 형식이 두 번 틀림");
        for kind in crate::delegation::BLOCK_KINDS.iter().filter(|kind| **kind != "other") { assert_ne!(block_label(kind), "기타 이유로 멈춤", "{kind} has a plain label"); }
    }
}
