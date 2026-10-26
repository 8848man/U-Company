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
fn wait_scope_approval(h: &Harness, id: &str) -> Approval { wait_operation(h, id, crate::engine::SCOPE_OPERATION) }

fn wait_operation(h: &Harness, id: &str, operation: &str) -> Approval {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let found = h.store().list::<Approval>("approval").unwrap().into_iter().find(|item| item.goal_id.as_deref() == Some(id) && item.status == "pending" && item.operation == operation);
        if let Some(approval) = found { h.wait(id, &["awaiting_approval"]); return approval; }
        let goal = h.goal(id);
        assert!(!["completed", "blocked"].contains(&goal.status.as_str()), "goal ended as {} before asking for scope: {}", goal.status, goal.report);
        assert!(Instant::now() < deadline, "no {operation} approval");
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
    let Some(plan_cmd) = harness(&["plan-commands"]) else { return };
    let cmd_id = plan_cmd.submit(GOAL);
    plan_cmd.wait(&cmd_id, &["awaiting_approval"]);
    write("plancmd", serde_json::json!({ "snapshot": plan_cmd.store().snapshot().unwrap(), "goalId": cmd_id }));
    let Some(perm) = harness(&["needs-permission-once"]) else { return };
    let perm_id = perm.submit(GOAL);
    perm.wait(&perm_id, &["awaiting_approval"]);
    perm.approve(&perm_id);
    let _ = wait_operation(&perm, &perm_id, crate::permissions::OPERATION);
    write("permission", serde_json::json!({ "snapshot": perm.store().snapshot().unwrap(), "goalId": perm_id }));
    let Some(open) = harness(&[]) else { return };
    put_goal(&open, A, "blocked", "2026-01-01T00:00:01Z", "할 일을 적고 완료 표시와 삭제를 할 수 있는 작은 웹앱을 만들어 주세요");
    put_goal(&open, B, "completed", "2026-01-01T00:00:02Z", "멈춘 이유를 찾아 고칠 일을 정해요");
    put_link(&open, A, B);
    write("unchecked", serde_json::json!({ "snapshot": open.store().snapshot().unwrap(), "goalId": B }));
    crate::objective::check(open.app.handle(), A, "completion").unwrap();
    write("objopen", serde_json::json!({ "snapshot": open.store().snapshot().unwrap(), "goalId": B }));
    let Some(closed) = harness(&["objective-achieved"]) else { return };
    put_goal(&closed, A, "blocked", "2026-01-01T00:00:01Z", "할 일을 적고 완료 표시와 삭제를 할 수 있는 작은 웹앱을 만들어 주세요");
    put_goal(&closed, B, "blocked", "2026-01-01T00:00:02Z", "통합 충돌을 풀어요");
    put_link(&closed, A, B);
    put_goal(&closed, X, "completed", "2026-01-01T00:00:03Z", "할 일 앱 전체를 새로 만들어요");
    crate::objective::check(closed.app.handle(), A, "resume").unwrap();
    write("objclosed", serde_json::json!({ "snapshot": closed.store().snapshot().unwrap(), "goalId": B }));
}

fn granted(h: &Harness, goal_id: &str) -> Vec<String> {
    crate::permissions_flow::grants(&h.store(), goal_id).unwrap().iter().map(crate::permissions::Rule::key).collect()
}

#[test]
fn commands_named_in_the_plan_are_granted_with_it_and_risky_ones_are_held_back() {
    let Some(h) = harness(&["plan-commands"]) else { return };
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    let plan = h.pending_approval(&id);
    let request: crate::permissions::CommandRequest = h.store().get("command_request", &plan.id).unwrap().expect("plan command request");
    assert_eq!(request.rules.iter().map(|rule| (rule.key(), rule.class.clone())).collect::<Vec<_>>(), vec![("Bash(node --test)".to_string(), "safe".to_string()), ("Bash(npm install)".to_string(), "individual".to_string())]);
    assert!(plan.reason.contains("실행할 명령(승인하면 이 대화에서 허용): node --test") && plan.reason.contains("따로 확인할 명령"), "{}", plan.reason);
    assert!(granted(&h, &id).is_empty(), "nothing is granted before the plan is approved");
    h.approve(&id);
    h.wait(&id, &["completed"]);
    assert_eq!(granted(&h, &id), vec!["Bash(node --test)"]);
    assert!(!h.store().list::<Approval>("approval").unwrap().iter().any(|item| item.operation == crate::permissions::OPERATION), "a granted command never stops the work");
}

#[test]
fn a_refused_test_command_stops_before_integration_and_resumes_after_approve_all() {
    let Some(h) = harness(&["needs-permission-once"]) else { return };
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    h.approve(&id);
    let card = wait_operation(&h, &id, crate::permissions::OPERATION);
    assert!(!h.root.join("todo.js").exists(), "the refused task is not integrated while the card waits");
    let request: crate::permissions::CommandRequest = h.store().get("command_request", &card.id).unwrap().unwrap();
    assert_eq!(request.rules.iter().map(|rule| (rule.key(), rule.class.as_str())).collect::<Vec<_>>(), vec![("Bash(npm install)".to_string(), "individual"), ("Bash(node --test)".to_string(), "safe"), ("Bash(echo \"=== exit: $? ===\")".to_string(), "safe")], "one rule per chained command");
    let keys: Vec<String> = request.rules.iter().filter(|rule| rule.class == "safe").map(crate::permissions::Rule::key).collect();
    crate::permissions_flow::decide(&h.store(), &card.id, &keys).unwrap();
    let goal = h.wait(&id, &["completed"]);
    assert!(h.event_kinds(&id).contains(&"task_resumed_with_commands".to_string()));
    let tests_task = h.tasks(&id).into_iter().find(|task| task.files.iter().any(|file| file == "todo.test.js")).unwrap();
    assert!(tests_task.result.contains("pass 3"), "the resumed session ran the approved command: {}", tests_task.result);
    assert!(granted(&h, &id).contains(&"Bash(node --test)".to_string()));
    assert!(!granted(&h, &id).contains(&"Bash(npm install)".to_string()), "approve-all leaves the risky command out");
    assert!(goal.report.contains("QA result:\nPASS"));
}

#[test]
fn declining_commands_lets_the_task_continue_as_reported() {
    let Some(h) = harness(&["needs-permission-once"]) else { return };
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    h.approve(&id);
    let card = wait_operation(&h, &id, crate::permissions::OPERATION);
    crate::permissions_flow::decide(&h.store(), &card.id, &[]).unwrap();
    h.wait(&id, &["completed"]);
    assert!(!h.event_kinds(&id).contains(&"task_resumed_with_commands".to_string()));
    assert!(granted(&h, &id).is_empty());
    assert!(crate::permissions_flow::decide(&h.store(), &card.id, &[]).is_err(), "a decided card cannot be decided again");
}

#[test]
fn qa_asks_for_its_test_command_reviews_in_a_copy_and_continues() {
    let Some(h) = harness(&["qa-needs-permission-once"]) else { return };
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    h.approve(&id);
    let card = wait_operation(&h, &id, crate::permissions::OPERATION);
    assert!(card.reason.starts_with("QA("), "{}", card.reason);
    assert!(std::fs::read_dir(h.data.parent().unwrap().join("worktrees").join(&id)).unwrap().filter_map(Result::ok).any(|entry| entry.file_name().to_string_lossy().starts_with("review-")), "QA works in a review copy");
    crate::permissions_flow::decide(&h.store(), &card.id, &["Bash(node --test)".into()]).unwrap();
    h.wait(&id, &["completed"]);
    assert!(!std::fs::read_dir(h.data.parent().unwrap().join("worktrees").join(&id)).unwrap().filter_map(Result::ok).any(|entry| entry.file_name().to_string_lossy().starts_with("review-")), "the review copy is removed");
    let generic = crate::permissions_flow::decide(&h.store(), &card.id, &["Bash(node --test)".into()]);
    assert!(generic.is_err());
}

// ---------------------------------------------------------------- ADR-0059 conversation objective

fn put_goal(h: &Harness, id: &str, status: &str, created: &str, intent: &str) -> Goal {
    let project: crate::model::Project = h.store().get("project", "selected").unwrap().unwrap();
    let goal: Goal = serde_json::from_value(serde_json::json!({ "id": id, "kind": "execution", "projectId": project.id, "intent": intent, "status": status, "report": if status == "blocked" { "Blocked: 통합 실패" } else { "done" }, "createdAt": created, "reviewLoopsUsed": 0, "reviewLoopLimit": 2, "blockKind": if status == "blocked" { "integration_conflict" } else { "" } })).unwrap();
    h.store().put("goal", id, &goal, None).unwrap();
    goal
}

fn put_link(h: &Harness, from: &str, to: &str) {
    let link: crate::model::GoalGuidance = serde_json::from_value(serde_json::json!({ "id": format!("{from}-{to}"), "goalId": from, "text": "멈춘 원인을 해결해 주세요", "status": "revised", "createdAt": "t", "replacementGoalId": to })).unwrap();
    h.store().put("goal_guidance", &link.id, &link, None).unwrap();
}

fn objective(h: &Harness, conversation: &str) -> Option<crate::objective::ConversationObjective> { h.store().get("conversation_objective", conversation).unwrap() }

fn wait_objective(h: &Harness, conversation: &str, states: &[&str]) -> crate::objective::ConversationObjective {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if let Some(found) = objective(h, conversation) { if states.contains(&found.state.as_str()) { return found; } }
        assert!(Instant::now() < deadline, "objective never reached {states:?}: {:?}", objective(h, conversation));
        std::thread::sleep(Duration::from_millis(100));
    }
}

const A: &str = "aaaaaaaa-0000-4000-8000-000000000001";
const B: &str = "bbbbbbbb-0000-4000-8000-000000000002";
const X: &str = "cccccccc-0000-4000-8000-000000000003";

#[test]
fn a_single_version_that_passes_qa_achieves_its_objective_without_a_model_call() {
    let Some(h) = harness(&[]) else { return };
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    h.approve(&id);
    h.wait(&id, &["completed"]);
    let found = wait_objective(&h, &id, &["achieved"]);
    assert_eq!(found.achieved_by, vec![id.clone()]);
    let calls = h.store().list::<crate::model::TokenUsageRecord>("usage").unwrap().into_iter().filter(|item| item.goal_id == id && item.agent_id == "cto").count();
    assert_eq!(calls, 1, "only the CTO analysis; the objective needed no extra call");
}

#[test]
fn a_finished_small_pass_leaves_the_conversation_open_and_resume_plans_the_remaining_work() {
    let Some(h) = harness(&[]) else { return };
    put_goal(&h, A, "blocked", "2026-01-01T00:00:01Z", "할 일 앱을 만들어 주세요");
    put_goal(&h, B, "completed", "2026-01-01T00:00:02Z", "멈춘 이유를 찾아요");
    put_link(&h, A, B);
    crate::objective::check(h.app.handle(), A, "completion").unwrap();
    let open = objective(&h, A).unwrap();
    assert_eq!(open.state, "open", "the newest pass completed but the original objective did not");
    assert_eq!(open.remaining.len(), 2);
    crate::objective::resume(h.app.handle().clone(), A.into()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    let next = loop {
        let guidance = h.store().list::<crate::model::GoalGuidance>("goal_guidance").unwrap();
        if let Some(next) = guidance.iter().find(|item| item.goal_id == B).and_then(|item| item.replacement_goal_id.clone()) { break next; }
        assert!(Instant::now() < deadline, "no follow-up pass was created");
        std::thread::sleep(Duration::from_millis(100));
    };
    let goal = h.wait(&next, &["awaiting_approval"]);
    assert!(goal.intent.contains("원래 목표를 이어서 완료해 주세요") && goal.intent.contains("화면 파일"), "{}", goal.intent);
    assert!(h.store().list::<crate::model::GoalTurn>("goal_turn").unwrap().iter().any(|turn| turn.goal_id == B && turn.route.as_deref() == Some("followup")));
}

#[test]
fn resume_closes_a_conversation_a_later_goal_achieved_without_touching_files_and_it_can_be_reopened() {
    let Some(h) = harness(&["objective-achieved"]) else { return };
    put_goal(&h, A, "blocked", "2026-01-01T00:00:01Z", "할 일 앱을 만들어 주세요");
    put_goal(&h, B, "blocked", "2026-01-01T00:00:02Z", "통합 충돌을 풀어요");
    put_link(&h, A, B);
    put_goal(&h, X, "completed", "2026-01-01T00:00:03Z", "할 일 앱 전체를 새로 만들어요");
    let readme = std::fs::read_to_string(h.root.join("README.md")).unwrap();
    crate::objective::resume(h.app.handle().clone(), A.into()).unwrap();
    let closed = wait_objective(&h, A, &["closed_by_other"]);
    assert_eq!(closed.achieved_by, vec![X.to_string()]);
    assert!(!closed.evidence.is_empty());
    assert_eq!(h.goal(B).status, "blocked", "closing changes no goal record");
    assert_eq!(std::fs::read_to_string(h.root.join("README.md")).unwrap(), readme);
    assert!(h.store().list::<crate::model::GoalTurn>("goal_turn").unwrap().iter().any(|turn| turn.route.as_deref() == Some("objective_closed")));
    assert!(crate::objective::resume(h.app.handle().clone(), A.into()).is_err(), "a closed conversation is reopened first");
    assert_eq!(crate::objective::reopen(&h.store(), A).unwrap().state, "reopened");
    assert!(crate::objective::reopen(&h.store(), A).is_err());
}

#[test]
fn a_malformed_judgement_never_closes_and_resume_starts_nothing() {
    let Some(h) = harness(&["objective-bad"]) else { return };
    put_goal(&h, A, "blocked", "2026-01-01T00:00:01Z", "할 일 앱을 만들어 주세요");
    put_goal(&h, B, "completed", "2026-01-01T00:00:02Z", "멈춘 이유를 찾아요");
    put_link(&h, A, B);
    crate::objective::resume(h.app.handle().clone(), A.into()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    while !h.event_kinds(B).contains(&"objective_resume_failed".to_string()) { assert!(Instant::now() < deadline, "no failure recorded"); std::thread::sleep(Duration::from_millis(100)); }
    let found = objective(&h, A).unwrap();
    assert_eq!(found.state, "open");
    assert!(found.summary.contains("형식"), "{}", found.summary);
    assert!(!h.store().list::<crate::model::GoalGuidance>("goal_guidance").unwrap().iter().any(|item| item.goal_id == B), "no next pass");
}

#[test]
fn project_changes_name_files_edited_after_the_app_integrated_them() {
    let Some(h) = harness(&[]) else { return };
    put_goal(&h, A, "completed", "2026-01-01T00:00:01Z", "x");
    std::fs::write(h.root.join("kept.md"), "same").unwrap();
    std::fs::write(h.root.join("edited.md"), "before").unwrap();
    let task: Task = serde_json::from_value(serde_json::json!({ "id": "t1", "goalId": A, "title": "t", "instructions": "i", "assigneeId": "developer-a", "status": "completed", "files": ["kept.md", "edited.md", "gone.md"], "dependsOn": [], "result": "" })).unwrap();
    std::fs::write(h.root.join("gone.md"), "x").unwrap();
    let proof = crate::model::TaskIntegrationProof { task_id: "t1".into(), goal_id: A.into(), files: ["kept.md", "edited.md", "gone.md"].iter().map(|file| crate::engine::digest_project_file(&h.root, file).unwrap()).collect(), integrated_at: "t".into() };
    h.store().put("task_proof", "t1", &proof, None).unwrap();
    std::fs::write(h.root.join("edited.md"), "after").unwrap();
    std::fs::remove_file(h.root.join("gone.md")).unwrap();
    let project: crate::model::Project = h.store().get("project", "selected").unwrap().unwrap();
    let text = crate::objective::project_changes(&h.store(), &project, &[A.to_string()], &[task]).unwrap();
    assert!(text.contains("edited.md (앱이 합친 뒤 내용이 바뀜)") && text.contains("gone.md (지금은 없거나") && !text.contains("- kept.md"), "{text}");
    assert!(text.contains("Git 작업 트리"), "{text}");
}

// ---------------------------------------------------------------- ADR-0060 criteria the team cannot meet

const STUCK: &str = "README.md에 변경 전 커밋 ID와 두 파일의 SHA-256을 적는다";
const REPLACEMENT: &str = "README.md는 변경 전 비교를 미확인으로 표시한다";

fn criteria_card(h: &Harness, id: &str) -> (Approval, crate::criteria::CriteriaRequest) {
    let card = wait_operation(h, id, crate::criteria::OPERATION);
    let request = h.store().get("criteria_request", &card.id).unwrap().expect("criteria request");
    (card, request)
}

fn decision(item: &crate::criteria::StuckCriterion, action: &str, text: &str) -> crate::criteria::CriteriaDecision {
    crate::criteria::CriteriaDecision { task_id: item.task_id.clone(), index: item.index, action: action.into(), text: text.into(), old: String::new() }
}

fn wait_status(h: &Harness, id: &str, status: &str) -> Goal {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let goal = h.goal(id);
        if goal.status == status { return goal; }
        assert!(Instant::now() < deadline, "still {} ({}): {}", goal.status, goal.block_kind, goal.report);
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn a_missing_precondition_stops_before_any_rework_and_dropping_it_completes() {
    let Some(h) = harness(&["qa-precondition"]) else { return };
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    h.approve(&id);
    let (card, request) = criteria_card(&h, &id);
    assert_eq!(h.goal(&id).review_loops_used, 0, "no rework round was spent");
    assert!(!h.tasks(&id).iter().any(|task| task.title.starts_with(crate::delegation::REPAIR_PREFIX)), "no repair task");
    assert_eq!(request.items.len(), 1);
    let item = &request.items[0];
    assert_eq!((item.kind.as_str(), item.criterion.as_str()), ("precondition", STUCK));
    assert!(item.reason.contains("Git 이력"), "{}", item.reason);
    assert_eq!((item.proposal_action.as_str(), item.proposal_text.as_str()), ("replace", REPLACEMENT), "the planner's suggestion is on the card");
    assert!(card.reason.contains("팀이 채울 수 없음"));
    assert!(crate::criteria::decide(&h.store(), &card.id, &[decision(item, "replace", "두 줄\n기준")], false).is_err(), "a replacement is one line");
    crate::criteria::decide(&h.store(), &card.id, &[decision(item, "drop", "")], false).unwrap();
    let goal = wait_status(&h, &id, "completed");
    assert_eq!(goal.review_loops_used, 0);
    assert!(h.tasks(&id).iter().all(|task| !task.acceptance.iter().any(|text| text == STUCK)), "the dropped criterion is gone");
    let saved: crate::criteria::CriteriaRequest = h.store().get("criteria_request", &card.id).unwrap().unwrap();
    assert_eq!((saved.decisions[0].action.as_str(), saved.decisions[0].old.as_str()), ("drop", STUCK), "the change is recorded with the old text");
    assert!(h.event_kinds(&id).contains(&"criteria_changed".to_string()));
    assert!(crate::criteria::decide(&h.store(), &card.id, &[], false).is_err(), "a decided card cannot be decided again");
}

#[test]
fn the_same_failure_after_a_rework_stops_at_the_card_and_stop_blocks_for_a_new_plan() {
    let Some(h) = harness(&["qa-fail-always"]) else { return };
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    h.approve(&id);
    let (card, request) = criteria_card(&h, &id);
    assert_eq!(h.goal(&id).review_loops_used, 1, "one rework ran; the second is not spent");
    assert!(request.items.iter().all(|item| item.kind == "no_progress") && request.items.iter().any(|item| item.criterion == STUCK), "{:?}", request.items);
    crate::criteria::decide(&h.store(), &card.id, &[], true).unwrap();
    let goal = wait_status(&h, &id, "blocked");
    assert_eq!(goal.block_kind, "criteria_blocked");
    assert_eq!(goal.review_loops_used, 1);
    assert!(goal.report.contains(STUCK), "{}", goal.report);
    let snapshot = h.store().snapshot().unwrap();
    assert_eq!(crate::team_fix::route(&goal, &snapshot.tasks, &snapshot.approvals, &snapshot.goal_guidance), crate::team_fix::FixRoute::Revision, "the fix request re-plans instead of reworking again");
}

#[test]
fn a_repair_plan_that_proposes_a_criteria_change_asks_first_and_qa_judges_the_new_criterion() {
    let Some(h) = harness(&["qa-fail-always", "repair-criteria"]) else { return };
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    h.approve(&id);
    let (card, request) = criteria_card(&h, &id);
    assert_eq!(h.goal(&id).review_loops_used, 0, "the proposal is decided before any rework");
    let item = &request.items[0];
    assert_eq!((item.kind.as_str(), item.proposal_text.as_str()), ("proposal", REPLACEMENT));
    crate::criteria::decide(&h.store(), &card.id, &[decision(item, "replace", REPLACEMENT)], false).unwrap();
    let goal = wait_status(&h, &id, "completed");
    assert_eq!(goal.review_loops_used, 0);
    assert!(h.tasks(&id).iter().any(|task| task.acceptance.iter().any(|text| text == REPLACEMENT)));
    let reviews = h.store().snapshot().unwrap().deliverable_reviews;
    assert!(reviews.iter().filter(|review| review.goal_id == id).flat_map(|review| review.criteria.iter()).any(|criterion| criterion.criterion == REPLACEMENT && criterion.met), "QA judged the replaced criterion");
}

#[test]
fn when_only_ceo_actions_remain_resume_plans_no_pass() {
    let Some(h) = harness(&["objective-ceo"]) else { return };
    put_goal(&h, A, "blocked", "2026-01-01T00:00:01Z", "할 일 앱을 만들고 GitHub에 푸시해 주세요");
    put_goal(&h, B, "completed", "2026-01-01T00:00:02Z", "멈춘 이유를 찾아요");
    put_link(&h, A, B);
    crate::objective::resume(h.app.handle().clone(), A.into()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    let turn = loop {
        if let Some(turn) = h.store().list::<crate::model::GoalTurn>("goal_turn").unwrap().into_iter().find(|turn| turn.route.as_deref() == Some("objective_ceo")) { break turn; }
        assert!(Instant::now() < deadline, "no CEO-action turn");
        std::thread::sleep(Duration::from_millis(100));
    };
    assert!(turn.answer.contains("GitHub 원격 저장소에 푸시해요"), "{}", turn.answer);
    let found = objective(&h, A).unwrap();
    assert_eq!((found.state.as_str(), found.remaining.len(), found.ceo_actions.len()), ("open", 0, 1));
    assert!(!h.store().list::<crate::model::GoalGuidance>("goal_guidance").unwrap().iter().any(|item| item.goal_id == B), "no next pass for CEO-only work");
}
