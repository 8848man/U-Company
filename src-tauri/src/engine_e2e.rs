//! ADR-0052: whole-goal tests. The real engine runs on `tauri::test::mock_app()` against a temporary
//! Git project, with every teammate on the mock AI (`scripts/mock-ai/mock-adapter.mjs`). Nothing is
//! stubbed inside the engine: analysis, planning, approval, isolated worktrees, integration, QA, rework
//! and the report all take their production paths. Skipped (with a note) when Node or Git is missing.

use crate::model::{Approval, Goal, Task};
use crate::storage::Store;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use tauri::Manager;

pub(crate) struct Harness {
    pub app: tauri::App<tauri::test::MockRuntime>,
    pub root: PathBuf,
    pub data: PathBuf,
    _dir: TempDir,
}

pub(crate) struct TempDir(pub PathBuf);
impl Drop for TempDir { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }

fn tools_available() -> bool {
    let node = crate::discovery::find_executable("node").is_some();
    let git = Command::new("git").arg("--version").output().is_ok_and(|out| out.status.success());
    if !(node && git) { eprintln!("engine_e2e skipped: node={node} git={git}"); }
    node && git
}

fn git(root: &Path, args: &[&str]) {
    let out = Command::new("git").args(args).current_dir(root).output().expect("git");
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
}

/// A committed Git project, a fresh store whose team all use the mock AI with `modes`.
pub(crate) fn harness(modes: &[&str]) -> Option<Harness> {
    if !tools_available() { return None; }
    let dir = TempDir(std::env::temp_dir().join(format!("u-e2e-{}", crate::model::id())));
    let root = dir.0.join("project");
    let data = dir.0.join("data");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&data).unwrap();
    git(&root, &["init", "-q", "-b", "main"]);
    git(&root, &["config", "user.email", "e2e@example.invalid"]);
    git(&root, &["config", "user.name", "e2e"]);
    git(&root, &["config", "core.autocrlf", "false"]);
    std::fs::write(root.join("README.md"), "# 샘플 프로젝트\n").unwrap();
    git(&root, &["add", "."]);
    git(&root, &["commit", "-q", "-m", "init"]);
    let store = Store::open(&data.join("organization.sqlite")).unwrap();
    let project = crate::platform::adopt_project(root.to_str().unwrap()).unwrap();
    assert_eq!(project.kind, "git");
    store.select_project(&project).unwrap();
    let node = crate::discovery::find_executable("node").unwrap();
    let mut connection = crate::mock_ai::connection(&node, &crate::mock_ai::script_path(), &modes.iter().map(|m| m.to_string()).collect::<Vec<_>>()).unwrap();
    connection.args.extend(["--state".to_string(), dir.0.join("mock-state").to_string_lossy().into_owned()]);
    store.register_connection_and_assign_unassigned(&connection).unwrap();
    let app = tauri::test::mock_app();
    app.manage(store);
    app.manage(crate::engine::WorktreeBase(dir.0.join("worktrees")));
    Some(Harness { app, root, data, _dir: dir })
}

impl Harness {
    pub fn store(&self) -> tauri::State<'_, Store> { self.app.state::<Store>() }

    pub fn submit(&self, intent: &str) -> String {
        let store = self.store();
        let project: crate::model::Project = store.get("project", "selected").unwrap().unwrap();
        let goal = Goal { id: crate::model::id(), kind: "execution".into(), project_id: Some(project.id), intent: intent.into(), status: "new".into(), report: String::new(), created_at: crate::model::now(), resumed_from: None, review_loops_used: 0, review_loop_limit: Some(project.review_loop_limit), work_group: None, block_kind: String::new() };
        store.put("goal", &goal.id, &goal, None).unwrap();
        crate::engine::start(self.app.handle().clone(), goal.id.clone()).unwrap();
        goal.id
    }

    pub fn goal(&self, id: &str) -> Goal { self.store().get("goal", id).unwrap().unwrap() }

    /// Waits until the goal reaches one of `statuses`; panics with its report on timeout or on an
    /// unexpected terminal state.
    pub fn wait(&self, id: &str, statuses: &[&str]) -> Goal {
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            let goal = self.goal(id);
            if statuses.contains(&goal.status.as_str()) { return goal; }
            if ["completed", "blocked", "cancelled"].contains(&goal.status.as_str()) { panic!("goal ended as {} ({}) while waiting for {statuses:?}:\n{}", goal.status, goal.block_kind, goal.report); }
            assert!(Instant::now() < deadline, "timed out in {} waiting for {statuses:?}", goal.status);
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn pending_approval(&self, goal_id: &str) -> Approval {
        self.store().list::<Approval>("approval").unwrap().into_iter().find(|item| item.goal_id.as_deref() == Some(goal_id) && item.status == "pending").expect("pending approval")
    }

    /// The same transaction `decide_approval` runs when the CEO presses 승인 on the card it saw.
    pub fn approve(&self, goal_id: &str) {
        let approval = self.pending_approval(goal_id);
        let digest = self.store().get::<crate::model::Squad>("squad", goal_id).unwrap().map(|squad| crate::squad::digest(&squad));
        self.store().transact(|tx| {
            let mut decided = approval.clone();
            decided.status = "approved".into();
            crate::squad::lock_in_tx(tx, &approval, true, digest.as_deref())?;
            crate::storage::tx_put(tx, "approval", &approval.id, &decided)
        }).unwrap();
    }

    /// `거부` on a scope-addition card: the plain approval decision.
    pub fn deny(&self, goal_id: &str) {
        let approval = self.pending_approval(goal_id);
        self.store().transact(|tx| { let mut decided = approval.clone(); decided.status = "denied".into(); crate::storage::tx_put(tx, "approval", &approval.id, &decided) }).unwrap();
    }

    pub fn tasks(&self, goal_id: &str) -> Vec<Task> { self.store().list::<Task>("task").unwrap().into_iter().filter(|task| task.goal_id == goal_id).collect() }

    pub fn event_kinds(&self, goal_id: &str) -> Vec<String> {
        self.store().snapshot().unwrap().events.into_iter().filter(|event| event.goal_id.as_deref() == Some(goal_id)).map(|event| event.kind).collect()
    }

    /// The generated app's own tests, run in the project after integration.
    pub fn project_tests_pass(&self) -> bool {
        Command::new(crate::discovery::find_executable("node").unwrap()).arg("--test").current_dir(&self.root).output().is_ok_and(|out| out.status.success())
    }
}

const GOAL: &str = "할 일을 적고 완료 표시와 삭제를 할 수 있는 작은 웹앱을 만들어 주세요. 테스트도 함께요.";

#[test]
fn a_sample_app_goal_runs_from_analysis_to_a_qa_passed_report() {
    let Some(h) = harness(&[]) else { return };
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    assert_eq!(h.tasks(&id).len(), 2, "the PM plan becomes two tasks before approval");
    assert!(!h.root.join("todo.js").exists(), "nothing changes in the project before approval");
    h.approve(&id);
    let goal = h.wait(&id, &["completed"]);
    for file in ["todo.js", "todo.test.js", "index.html", "style.css", "main.js"] { assert!(h.root.join(file).is_file(), "{file} integrated into the project"); }
    assert!(h.project_tests_pass(), "the integrated app's tests pass");
    assert!(goal.report.contains("QA result:\nPASS"), "the report carries the QA verdict:\n{}", goal.report);
    assert_eq!(goal.review_loops_used, 0);
    assert!(h.tasks(&id).iter().all(|task| task.status == "completed"));
    // ADR-0054: the result card lists what changed and how to try it.
    let result = crate::goal_result::result(&h.store(), &id).unwrap();
    assert_eq!(result.files.len(), 5);
    assert!(result.files.iter().all(|file| file.exists));
    assert!(result.hints.iter().any(|hint| hint.contains("index.html")) && result.hints.iter().any(|hint| hint.contains("node --test")), "{:?}", result.hints);
}

#[test]
fn a_qa_failure_is_reworked_inside_the_approved_files_and_then_passes() {
    let Some(h) = harness(&["qa-fail-once"]) else { return };
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    h.approve(&id);
    let goal = h.wait(&id, &["completed"]);
    assert_eq!(goal.review_loops_used, 1, "one rework round");
    assert!(h.tasks(&id).iter().any(|task| task.title.starts_with(crate::delegation::REPAIR_PREFIX)), "a repair task ran");
    assert!(h.event_kinds(&id).contains(&"review_rework_started".to_string()));
}

#[test]
fn a_malformed_plan_is_sent_back_once_and_the_goal_continues() {
    let Some(h) = harness(&["plan-form-once"]) else { return };
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    assert!(h.event_kinds(&id).contains(&"plan_form_retry".to_string()), "the app sent the plan back with the error");
    assert!(h.tasks(&id).iter().all(|task| task.acceptance.len() <= 8));
    h.approve(&id);
    h.wait(&id, &["completed"]);
}

#[test]
fn a_restart_while_waiting_for_approval_keeps_the_approval_pending() {
    let Some(h) = harness(&[]) else { return };
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    let approval = h.pending_approval(&id);
    // The engine thread keeps polling the old store; a reopened store is what the restarted app sees.
    let reopened = Store::open(&h.data.join("organization.sqlite")).unwrap();
    let after: Approval = reopened.get("approval", &approval.id).unwrap().unwrap();
    assert_eq!(after.status, "pending", "waiting never silently becomes approval");
    assert!(!h.root.join("todo.js").exists());
}

/// Waits until a second pending approval (the scope addition) appears for the goal.
fn wait_scope_approval(h: &Harness, id: &str) -> Approval {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let found = h.store().list::<Approval>("approval").unwrap().into_iter().find(|item| item.goal_id.as_deref() == Some(id) && item.status == "pending" && item.operation == crate::engine::SCOPE_OPERATION);
        if let Some(approval) = found { h.wait(id, &["awaiting_approval"]); return approval; }
        let goal = h.goal(id);
        assert!(!["completed", "blocked"].contains(&goal.status.as_str()), "goal ended as {} before asking for scope: {}", goal.status, goal.report);
        assert!(Instant::now() < deadline, "no scope approval");
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn a_repair_that_needs_a_new_file_asks_on_the_same_goal_and_completes() {
    let Some(h) = harness(&["qa-fail-once", "needs-scope-once"]) else { return };
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    h.approve(&id);
    let scope = wait_scope_approval(&h, &id);
    assert!(scope.reason.contains("README.md"), "the card names the new file: {}", scope.reason);
    let before = std::fs::read_to_string(h.root.join("README.md")).unwrap();
    assert_eq!(before, "# 샘플 프로젝트\n", "nothing changes before the CEO decides");
    h.approve(&id);
    let goal = h.wait(&id, &["completed"]);
    assert_eq!(goal.review_loops_used, 1);
    assert_ne!(std::fs::read_to_string(h.root.join("README.md")).unwrap(), before, "the added file was changed by the repair");
    let versions = h.store().list::<Goal>("goal").unwrap().len();
    assert_eq!(versions, 1, "no new goal version");
}

#[test]
fn a_denied_scope_addition_blocks_as_needs_scope_without_changing_files() {
    let Some(h) = harness(&["qa-fail-once", "needs-scope-once"]) else { return };
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    h.approve(&id);
    wait_scope_approval(&h, &id);
    h.deny(&id);
    let deadline = Instant::now() + Duration::from_secs(30);
    let goal = loop {
        let goal = h.goal(&id);
        if goal.status == "blocked" { break goal; }
        assert!(Instant::now() < deadline, "still {}", goal.status);
        std::thread::sleep(Duration::from_millis(100));
    };
    assert_eq!(goal.block_kind, "needs_scope");
    let project_id = goal.project_id.clone().unwrap();
    let stats = crate::goal_result::block_stats(&h.store().list("goal_block").unwrap(), &project_id);
    assert_eq!(stats.iter().map(|stat| (stat.kind.as_str(), stat.count)).collect::<Vec<_>>(), vec![("needs_scope", 1)]);
    assert_eq!(std::fs::read_to_string(h.root.join("README.md")).unwrap(), "# 샘플 프로젝트\n");
}

#[test]
fn a_repair_with_nothing_to_change_goes_back_to_qa_instead_of_blocking() {
    let Some(h) = harness(&["qa-fail-once", "repair-no-change"]) else { return };
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    h.approve(&id);
    let goal = h.wait(&id, &["completed"]);
    assert_eq!(goal.review_loops_used, 1);
    assert!(h.event_kinds(&id).contains(&"task_unchanged".to_string()));
}

/// Screen fixtures for headless rendering checks: real snapshots of a project with no goal, a plan
/// waiting for approval, a scope addition waiting, and a finished goal (with its result and block
/// statistics). Run with `U_SNAPSHOT_DIR=<dir> cargo test --lib dump_screen_snapshots -- --ignored`.
#[test]
#[ignore]
fn dump_screen_snapshots() {
    let Ok(out) = std::env::var("U_SNAPSHOT_DIR") else { return };
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap();
    let write = |name: &str, value: serde_json::Value| std::fs::write(out.join(format!("{name}.json")), serde_json::to_string_pretty(&value).unwrap()).unwrap();
    let Some(empty) = harness(&[]) else { return };
    write("empty", serde_json::to_value(empty.store().snapshot().unwrap()).unwrap());
    let plan_id = empty.submit(GOAL);
    empty.wait(&plan_id, &["awaiting_approval"]);
    write("plan", serde_json::json!({ "snapshot": empty.store().snapshot().unwrap(), "goalId": plan_id }));
    empty.approve(&plan_id);
    empty.wait(&plan_id, &["completed"]);
    let project_id = empty.goal(&plan_id).project_id.unwrap();
    write("completed", serde_json::json!({ "snapshot": empty.store().snapshot().unwrap(), "goalId": plan_id, "result": crate::goal_result::result(&empty.store(), &plan_id).unwrap() }));
    let Some(scope) = harness(&["qa-fail-once", "needs-scope-once"]) else { return };
    let scope_id = scope.submit(GOAL);
    scope.wait(&scope_id, &["awaiting_approval"]);
    scope.approve(&scope_id);
    wait_scope_approval(&scope, &scope_id);
    write("scope", serde_json::json!({ "snapshot": scope.store().snapshot().unwrap(), "goalId": scope_id }));
    scope.deny(&scope_id);
    let deadline = Instant::now() + Duration::from_secs(30);
    while scope.goal(&scope_id).status != "blocked" { assert!(Instant::now() < deadline); std::thread::sleep(Duration::from_millis(100)); }
    let scope_project = scope.goal(&scope_id).project_id.unwrap();
    write("blocked", serde_json::json!({ "snapshot": scope.store().snapshot().unwrap(), "goalId": scope_id, "stats": crate::goal_result::block_stats(&scope.store().list("goal_block").unwrap(), &scope_project) }));
    let _ = project_id;
}
