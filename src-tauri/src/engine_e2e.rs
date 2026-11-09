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
    git(&root, &["config", "core.autocrlf", "false"]); // Git for Windows defaults to CRLF checkout
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
            if Instant::now() >= deadline {
                let pending: Vec<String> = self.store().list::<Approval>("approval").unwrap().into_iter().filter(|item| item.goal_id.as_deref() == Some(id) && item.status == "pending").map(|item| format!("{} ({})", item.operation, item.reason.chars().take(120).collect::<String>())).collect();
                let events = self.event_kinds(id);
                panic!("timed out in {} waiting for {statuses:?}; pending approvals: {pending:?}; last events: {:?}", goal.status, &events[events.len().saturating_sub(12)..]);
            }
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
            crate::technology_presets::verify_binding_in_tx(tx, &approval, None, "approve")?;
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

/// ADR-0062: the real engine binds the plan to the project's preset selection; a later setting change
/// leaves the pending card and its snapshot alone, the approved revision runs, and restart keeps the link.
#[test]
fn a_preset_selection_is_frozen_into_the_plan_revision_through_approval_and_restart() {
    use crate::technology_presets::{select, snapshot_for, Pick};
    let Some(h) = harness(&[]) else { return };
    let project: crate::model::Project = h.store().get("project", "selected").unwrap().unwrap();
    let pick = |stage: &str, id: &str| Pick { stage: stage.into(), preset_id: Some(id.into()), version: Some("1.0.0".into()), ..Default::default() };
    select(&h.store(), &project.id, &pick("beginner", "B-WEB"), "ceo", None, None).unwrap();
    select(&h.store(), &project.id, &pick("intermediate", "I-SINGLE"), "ceo", None, None).unwrap();
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    let approval = h.pending_approval(&id);
    let snapshot = snapshot_for(&h.store(), &approval.id).unwrap().expect("the plan card has a snapshot");
    assert!(snapshot.applicable, "a development goal applies the selection");
    assert_eq!((snapshot.project_id.as_str(), snapshot.goal_id.as_str(), snapshot.plan_revision), (project.id.as_str(), id.as_str(), 1));
    assert_eq!(snapshot.choices.iter().map(|c| c.preset.id.as_str()).collect::<Vec<_>>(), ["B-WEB", "I-SINGLE"]);
    assert!(approval.reason.contains("B-WEB/1.0.0 + I-SINGLE/1.0.0"), "{}", approval.reason);
    assert_eq!(snapshot.tasks.len(), h.tasks(&id).len(), "the declared files of every planned task are frozen too");
    // A setting change while the card waits changes neither the card nor its snapshot.
    select(&h.store(), &project.id, &pick("beginner", "B-API"), "ceo", None, None).unwrap();
    assert_eq!(h.pending_approval(&id), approval);
    assert_eq!(snapshot_for(&h.store(), &approval.id).unwrap().unwrap(), snapshot);
    h.approve(&id);
    h.wait(&id, &["completed"]);
    let reopened = Store::open(&h.data.join("organization.sqlite")).unwrap();
    assert_eq!(reopened.get::<Approval>("approval", &approval.id).unwrap().unwrap().status, "approved");
    assert_eq!(snapshot_for(&reopened, &approval.id).unwrap().unwrap(), snapshot, "after restart the approved revision still names B-WEB, not the newer B-API setting");
    assert_eq!(crate::technology_presets::selections(&reopened, &project.id).unwrap()[0].preset_id, "B-API");
}

/// ADR-0062 / DM-03: an SDD section that owns the folder structure and disagrees with the selected
/// preset stops the real engine before any plan card; neither the SDD nor the selection is rewritten.
#[test]
fn an_existing_sdd_structure_owner_blocks_planning_without_being_overwritten() {
    use crate::technology_presets::{select, selections, Pick};
    let Some(h) = harness(&[]) else { return };
    let project: crate::model::Project = h.store().get("project", "selected").unwrap().unwrap();
    // Chosen before the SDD existed; the plan re-checks against the project as it is now.
    select(&h.store(), &project.id, &Pick { stage: "intermediate".into(), preset_id: Some("I-MODULAR".into()), version: Some("1.0.0".into()), ..Default::default() }, "ceo", None, None).unwrap();
    let sdd = h.root.join(".ucompany/sdd/architecture.md");
    std::fs::create_dir_all(sdd.parent().unwrap()).unwrap();
    std::fs::write(&sdd, "# 아키텍처\n\n## 폴더 구조\n- `src/` 앱 소스\n- `tests/` 시험\n").unwrap();
    git(&h.root, &["add", "."]);
    git(&h.root, &["commit", "-q", "-m", "sdd"]);
    let before = std::fs::read(&sdd).unwrap();
    let id = h.submit(GOAL);
    let goal = h.wait(&id, &["blocked"]);
    assert!(goal.report.contains("충돌") && goal.report.contains("architecture.md § 폴더 구조") && goal.report.contains("layout"), "{}", goal.report);
    assert!(h.store().list::<Approval>("approval").unwrap().iter().all(|item| item.goal_id.as_deref() != Some(id.as_str())), "no plan card is offered");
    assert_eq!(std::fs::read(&sdd).unwrap(), before, "the SDD owner is untouched");
    assert_eq!(selections(&h.store(), &project.id).unwrap()[0].preset_id, "I-MODULAR", "the selection is not changed to fit");
}

/// Security `AI와 만들기` through the real provider path (mock AI): refusal and a stale approval send
/// nothing; one approval sends the previewed payload once; the answer never becomes a goal turn,
/// a selection or a retry, and a restart keeps the record.
#[test]
fn an_ai_preset_draft_reaches_the_provider_only_once_after_exact_approval() {
    use crate::technology_presets::{ai_draft_request, approve_ai_draft, destination, finish_ai_draft, refuse_ai_draft, request_ai_draft, selections};
    let Some(h) = harness(&[]) else { return };
    let store = h.store();
    let project: crate::model::Project = store.get("project", "selected").unwrap().unwrap();
    let snapshot = store.snapshot().unwrap();
    let (agent, connection) = snapshot.agents.iter().find_map(|agent| crate::engine::discussion_runtime(&snapshot.agents, &snapshot.connections, &agent.id).ok()).expect("a member with a connection");
    let target = destination(&agent, &connection);
    let calls = std::cell::Cell::new(0);
    let call = |payload: &str| { calls.set(calls.get() + 1); crate::engine::send_preset_draft(&connection, payload) };
    let refused = request_ai_draft(&store, &project.id, "주문 관리 웹앱 구조 초안", &target).unwrap();
    assert!(refused.payload.contains("주문 관리 웹앱 구조 초안") && refused.destination.connection_id == connection.id);
    refuse_ai_draft(&store, &project.id, &refused.id).unwrap();
    assert!(approve_ai_draft(&store, &project.id, &refused.id, &refused.payload_sha256, &refused.destination_sha256, &target).is_err());
    let mut moved = target.clone();
    moved.model_id = format!("{}-other", moved.model_id);
    let stale = request_ai_draft(&store, &project.id, "주문 관리 웹앱 구조 초안", &target).unwrap();
    assert!(approve_ai_draft(&store, &project.id, &stale.id, &stale.payload_sha256, &stale.destination_sha256, &moved).is_err(), "another model is another destination");
    assert_eq!(calls.get(), 0);
    let approved = request_ai_draft(&store, &project.id, "주문 관리 웹앱 구조 초안", &target).unwrap();
    let sending = approve_ai_draft(&store, &project.id, &approved.id, &approved.payload_sha256, &approved.destination_sha256, &target).unwrap();
    let done = finish_ai_draft(&store, &approved.id, call(&sending.payload)).unwrap();
    assert_eq!(calls.get(), 1);
    // The mock does not know this request and answers in prose: recorded as failed, no draft, no retry.
    assert_eq!((done.status.as_str(), done.draft_id.as_deref()), ("failed", None), "{}", done.note);
    assert!(approve_ai_draft(&store, &project.id, &approved.id, &approved.payload_sha256, &approved.destination_sha256, &target).is_err());
    assert!(store.list::<crate::model::GoalTurn>("goal_turn").unwrap().is_empty(), "nothing is posted to a goal conversation");
    assert!(store.list::<Approval>("approval").unwrap().is_empty() && selections(&store, &project.id).unwrap().is_empty());
    let reopened = Store::open(&h.data.join("organization.sqlite")).unwrap();
    assert_eq!(ai_draft_request(&reopened, &approved.id).unwrap().unwrap().status, "failed", "a restart neither resends nor forgets");
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

/// ADR-0068 fixture: only the mock provider's *answers* are adapted. The engine still creates
/// the plan, approval, isolated worktree, integration proof, transition and independent QA.
/// The second source task depends on the first and remains untouched in the linked pass.
fn reuse_harness(modes: &[&str]) -> Option<Harness> {
    let h = harness(modes)?;
    let original = std::fs::read_to_string(crate::mock_ai::script_path()).unwrap();
    let plan_start = "const tasks = scenario.tasks.map(task => ({ ...task }));";
    let plan_return = "return JSON.stringify({ tasks, reusedTaskIds: [], squad:";
    let implementation = "if (repair) content += remark(file);";
    let written = "written.push(file);";
    let repair_extra = "if (options.modes.has('needs-scope-once') || approved.includes('readme.md')) files.push('README.md');";
    assert!(original.contains(plan_start) && original.contains(plan_return) && original.contains(implementation) && original.contains(written) && original.contains(repair_extra));
    let adapted = original
        .replace(plan_start, "const tasks = scenario.tasks.map(task => ({ ...task }));\n      if (prompt.includes('Previous goal ID:')) tasks.splice(1);")
        .replace(plan_return, "return JSON.stringify({ tasks, reusedTaskIds: prompt.includes('Previous goal ID:') ? between(prompt, 'Engine-verified reuse candidates: ', '\\n').split(',').map(x => x.trim()).filter(Boolean) : [], squad:")
        .replace(implementation, "if (repair) content += remark(file);\n        if (prompt.includes('REUSE_REVISION')) content += '\\n// revised by linked goal\\n';")
        .replace(written, "written.push(file);\n        if (file === 'todo.js' && prompt.includes('REUSE_FAILURE')) writeFileSync(join(dirname(options.state), 'project', file), '// outside edit during isolated work\\n');\n        if (file === 'todo.js' && prompt.includes('REUSE_OUTSIDE')) writeFileSync(insideFolder(cwd, 'index.html'), '<!-- undeclared edit -->\\n');")
        .replace(repair_extra, "if (options.modes.has('needs-scope-once') || approved.includes('readme.md')) files.push('README.md');\n      if (options.modes.has('reuse-other-owner')) files.push('index.html');");
    let script = h.data.join("reuse-mock.mjs");
    std::fs::write(&script, adapted).unwrap();
    let mut scenario: serde_json::Value = serde_json::from_slice(&std::fs::read(crate::mock_ai::script_path().parent().unwrap().join("scenarios/todo-app.json")).unwrap()).unwrap();
    scenario["tasks"][1]["dependsOn"] = serde_json::json!([0]);
    let scenario_path = h.data.join("reuse-scenario.json");
    std::fs::write(&scenario_path, serde_json::to_vec(&scenario).unwrap()).unwrap();
    let mut connection = h.store().list::<crate::model::AiConnection>("connection").unwrap().remove(0);
    let script_arg = crate::mock_ai::script_path().to_string_lossy().into_owned();
    let position = connection.args.iter().position(|arg| arg == &script_arg).expect("mock script argument");
    connection.args[position] = script.to_string_lossy().into_owned();
    connection.args.extend(["--scenario".into(), scenario_path.to_string_lossy().into_owned()]);
    h.store().put("connection", &connection.id, &connection, None).unwrap();
    Some(h)
}

fn linked_reuse_goal(h: &Harness, source: &str) -> String {
    linked_reuse_goal_with_intent(h, source, "REUSE_REVISION: improve the todo rules while preserving the screen")
}

fn linked_reuse_goal_with_intent(h: &Harness, source: &str, intent: &str) -> String {
    let guidance = h.store().propose_goal_followup(source, intent, "cto").unwrap();
    let next = h.store().create_goal_revision(&guidance.id, &guidance.text).unwrap();
    crate::engine::start(h.app.handle().clone(), next.id.clone()).unwrap();
    next.id
}

fn enable_reuse_mock_modes(h: &Harness, modes: &str) {
    let mut connection = h.store().list::<crate::model::AiConnection>("connection").unwrap().remove(0);
    connection.args.extend(["--mode".into(), modes.into()]);
    h.store().put("connection", &connection.id, &connection, None).unwrap();
}

/// The blocked case must reach the same serialized snapshot consumed by the UI, not only the
/// private store table. Check both the typed record and its camelCase wire representation.
fn snapshot_reuse_conflict(h: &Harness, goal_id: &str, path: &str, kind: &str) -> crate::model::ReuseConflict {
    let snapshot = h.store().snapshot().unwrap();
    let wire = serde_json::to_value(&snapshot).unwrap();
    let conflict = snapshot.reuse_conflicts.into_iter().find(|item| item.goal_id == goal_id && item.path == path && item.kind == kind).expect("saved conflict is present in the snapshot");
    assert!(!conflict.recovery.is_empty() && !conflict.missing_evidence.is_empty(), "the UI needs a recovery path and missing proof: {conflict:?}");
    let item = wire["reuseConflicts"].as_array().unwrap().iter().find(|item| item["goalId"] == goal_id && item["path"] == path && item["kind"] == kind).expect("saved conflict is serialized for the UI");
    assert!(item["recovery"].as_str().is_some());
    assert!(item["missingEvidence"].as_array().is_some());
    assert_eq!(item["beforeSha256"].as_str(), conflict.before_sha256.as_deref());
    assert_eq!(item["afterSha256"].as_str(), conflict.after_sha256.as_deref());
    assert_eq!(item["currentSha256"].as_str(), conflict.current_sha256.as_deref());
    assert_eq!(item["approvedFiles"].as_array().unwrap().len(), conflict.approved_files.len());
    let reopened = Store::open(&h.data.join("organization.sqlite")).unwrap();
    let persisted = reopened.snapshot().unwrap().reuse_conflicts.into_iter().find(|item| item.goal_id == goal_id && item.path == path && item.kind == kind).expect("conflict remains visible after restart");
    assert_eq!(persisted.current_sha256, conflict.current_sha256);
    assert_eq!(persisted.missing_evidence, conflict.missing_evidence);
    assert_eq!(persisted.recovery, conflict.recovery);
    conflict
}

fn assert_saved_transition_chain(h: &Harness, transition: &crate::model::ReuseFileTransition) {
    let reopened = Store::open(&h.data.join("organization.sqlite")).unwrap();
    let task: Task = reopened.get("task", &transition.task_id).unwrap().unwrap();
    let approval: Approval = reopened.get("approval", &transition.approval_id).unwrap().unwrap();
    let proof: crate::model::TaskIntegrationProof = reopened.get("task_proof", &task.id).unwrap().unwrap();
    let snapshot = reopened.snapshot().unwrap();
    assert_eq!(task.goal_id, transition.goal_id);
    assert_eq!(task.status, "completed");
    assert_eq!(approval.goal_id.as_deref(), Some(transition.goal_id.as_str()));
    assert_eq!(approval.status, "approved");
    assert!(transition.approved_files.iter().any(|file| file.eq_ignore_ascii_case(&transition.path)));
    assert!(task.files.iter().any(|file| file.eq_ignore_ascii_case(&transition.path)));
    assert_eq!(proof.goal_id, transition.goal_id);
    assert_eq!(proof.integrated_at, transition.integrated_at);
    assert_eq!(proof.files.iter().find(|file| file.path.eq_ignore_ascii_case(&transition.path)).unwrap().sha256, transition.after_sha256);
    assert!(snapshot.events.iter().any(|event| event.id.to_string() == transition.integration_event
        && event.kind == "task_integrated"
        && event.goal_id.as_deref() == Some(transition.goal_id.as_str())
        && event.task_id.as_deref() == Some(task.id.as_str())));
    let key = format!("{}:{}:{}", transition.goal_id, task.id, transition.path);
    let history: Vec<crate::model::ReuseFileTransition> = reopened.list("reuse_file_transition_history").unwrap();
    assert!(history.iter().any(|item| item.goal_id == transition.goal_id && item.task_id == task.id && item.path == transition.path && item.after_sha256 == transition.after_sha256), "transition history for {key} survives restart");
}

#[test]
fn approved_linked_reuse_integrates_new_evidence_then_independent_qa_checks_it_after_restart() {
    let Some(h) = reuse_harness(&[]) else { return };
    let first = h.submit(GOAL);
    h.wait(&first, &["awaiting_approval"]);
    h.approve(&first);
    h.wait(&first, &["completed"]);
    let old_tasks = h.tasks(&first);
    let rules = old_tasks.iter().find(|task| task.files.contains(&"todo.js".to_string())).unwrap();
    let screen = old_tasks.iter().find(|task| task.files.contains(&"index.html".to_string())).unwrap();
    assert!(screen.depends_on.contains(&rules.id));
    let old_rules: crate::model::TaskIntegrationProof = h.store().get("task_proof", &rules.id).unwrap().unwrap();
    let old_sha = old_rules.files.iter().find(|file| file.path == "todo.js").unwrap().sha256.clone();
    let next = linked_reuse_goal(&h, &first);
    h.wait(&next, &["awaiting_approval"]);
    let approval = h.pending_approval(&next);
    let guidance = h.store().list::<crate::model::GoalGuidance>("goal_guidance").unwrap().into_iter().find(|item| item.replacement_goal_id.as_deref() == Some(&next)).unwrap();
    let decision = guidance.continuation.unwrap();
    assert!(decision.replaced_task_ids.contains(&rules.id), "overlapping source task is not fully reused");
    assert!(decision.reused_task_ids.contains(&screen.id), "untouched dependent task remains reused");
    assert!(!decision.reused_task_ids.contains(&rules.id), "a source task with overlapping files cannot also be reused");
    assert!(approval.reason.contains("todo.js"), "overlap is visible before approval: {}", approval.reason);
    assert!(h.store().list::<crate::model::ReuseFileTransition>("reuse_file_transition").unwrap().is_empty());
    h.approve(&next);
    let goal = h.wait(&next, &["completed"]);
    let changed = h.tasks(&next).into_iter().find(|task| task.files.contains(&"todo.js".to_string())).unwrap();
    assert_eq!(changed.status, "completed");
    let transition = h.store().list::<crate::model::ReuseFileTransition>("reuse_file_transition").unwrap().into_iter().find(|item| item.goal_id == next && item.path == "todo.js").expect("actual integration creates transition");
    assert_eq!(transition.source_task_id, rules.id);
    assert_eq!(transition.task_id, changed.id);
    assert_eq!(transition.approval_id, approval.id);
    assert!(transition.approved_files.contains(&"todo.js".to_string()));
    assert_eq!(transition.before_sha256, old_sha);
    assert_ne!(transition.after_sha256, old_sha);
    assert_eq!(transition.after_sha256, crate::engine::digest_project_file(&h.root, "todo.js").unwrap().sha256);
    assert_saved_transition_chain(&h, &transition);
    assert!(h.store().snapshot().unwrap().events.iter().any(|event| event.id.to_string() == transition.integration_event && event.kind == "task_integrated" && event.task_id.as_deref() == Some(&changed.id)));
    assert!(!h.store().list::<crate::model::ReuseFileTransition>("reuse_file_transition").unwrap().iter().any(|item| item.path == "index.html"));
    let screen_proof: crate::model::TaskIntegrationProof = h.store().get("task_proof", &screen.id).unwrap().unwrap();
    assert_eq!(screen_proof.files.iter().find(|file| file.path == "index.html").unwrap().sha256, crate::engine::digest_project_file(&h.root, "index.html").unwrap().sha256, "the untouched dependent keeps its original proof");
    assert!(h.store().list::<crate::model::DeliverableReview>("deliverable_review").unwrap().iter().any(|review| review.goal_id == next && review.task_id == changed.id && review.verdict == "PASS"), "the current deliverable was independently reviewed");
    assert!(h.event_kinds(&next).contains(&"qa_review".to_string()));
    assert!(goal.report.contains("QA result:\nPASS"));
    assert!(h.store().snapshot().unwrap().reuse_conflicts.iter().all(|item| item.goal_id != next));
    let reopened = Store::open(&h.data.join("organization.sqlite")).unwrap();
    let persisted: crate::model::ReuseFileTransition = reopened.get("reuse_file_transition", &format!("{}:{}", next, "todo.js")).unwrap().unwrap();
    assert_eq!(persisted.after_sha256, transition.after_sha256);
    assert_eq!(persisted.integration_event, transition.integration_event);
    assert!(reopened.snapshot().unwrap().reuse_conflicts.iter().all(|item| item.goal_id != next));
    let third = linked_reuse_goal(&h, &next);
    h.wait(&third, &["awaiting_approval"]);
    let third_guidance = h.store().list::<crate::model::GoalGuidance>("goal_guidance").unwrap().into_iter().find(|item| item.replacement_goal_id.as_deref() == Some(&third)).unwrap();
    assert!(third_guidance.continuation.unwrap().replaced_task_ids.contains(&rules.id));
    assert!(h.store().get::<crate::model::ReuseFileTransition>("reuse_file_transition", &format!("{}:{}", next, "todo.js")).unwrap().is_some(), "the next plan preserves its predecessor's evidence");
    assert!(h.store().list::<crate::model::ReuseFileTransition>("reuse_file_transition").unwrap().iter().all(|item| item.goal_id != third), "another linked plan does not invent integration evidence");
}

#[test]
fn linked_reuse_qa_repair_inside_scope_integrates_but_empty_repair_creates_no_transition() {
    let Some(h) = reuse_harness(&[]) else { return };
    let first = h.submit(GOAL);
    h.wait(&first, &["awaiting_approval"]);
    h.approve(&first);
    h.wait(&first, &["completed"]);
    enable_reuse_mock_modes(&h, "qa-fail-once,repair-no-change");
    let next = linked_reuse_goal(&h, &first);
    h.wait(&next, &["awaiting_approval"]);
    h.approve(&next);
    let scope = wait_scope_approval(&h, &next);
    assert!(scope.reason.contains("todo.js"));
    h.approve(&next);
    let goal = h.wait(&next, &["completed"]);
    assert_eq!(goal.review_loops_used, 1);
    assert!(h.event_kinds(&next).contains(&"task_unchanged".to_string()));
    let repairs: Vec<Task> = h.tasks(&next).into_iter().filter(|task| task.title.starts_with(crate::delegation::REPAIR_PREFIX)).collect();
    assert_eq!(repairs.len(), 1);
    assert!(repairs[0].files.iter().all(|file| ["todo.js", "todo.test.js"].contains(&file.as_str())));
    let transitions: Vec<crate::model::ReuseFileTransition> = h.store().list("reuse_file_transition_history").unwrap();
    assert!(transitions.iter().any(|item| item.goal_id == next));
    assert!(!transitions.iter().any(|item| item.task_id == repairs[0].id), "a no-change QA repair has no new ownership event");
    let reviews: Vec<crate::model::DeliverableReview> = h.store().list("deliverable_review").unwrap();
    assert!(reviews.iter().any(|item| item.goal_id == next && item.round == 0 && item.verdict == "FAIL"));
    assert!(reviews.iter().any(|item| item.goal_id == next && item.round == 1 && item.verdict == "PASS"), "unchanged current artifacts were independently reviewed again");
}

#[test]
fn linked_reuse_qa_repair_inside_approved_files_records_a_second_integration() {
    let Some(h) = reuse_harness(&[]) else { return };
    let first = h.submit(GOAL);
    h.wait(&first, &["awaiting_approval"]);
    h.approve(&first);
    h.wait(&first, &["completed"]);
    enable_reuse_mock_modes(&h, "qa-fail-once");
    let next = linked_reuse_goal(&h, &first);
    h.wait(&next, &["awaiting_approval"]);
    h.approve(&next);
    let scope = wait_scope_approval(&h, &next);
    assert!(scope.reason.contains("todo.js"));
    h.approve(&next);
    let goal = h.wait(&next, &["completed"]);
    assert_eq!(goal.review_loops_used, 1);
    let repair = h.tasks(&next).into_iter().find(|task| task.title.starts_with(crate::delegation::REPAIR_PREFIX)).unwrap();
    let transitions: Vec<crate::model::ReuseFileTransition> = h.store().list("reuse_file_transition_history").unwrap();
    let repaired = transitions.iter().find(|item| item.task_id == repair.id && item.path == "todo.js").expect("the real repair integration advances the file's evidence");
    let previous_key = repaired.previous_transition_key.as_ref().expect("repair links to the first integration");
    let previous: crate::model::ReuseFileTransition = h.store().get("reuse_file_transition_history", previous_key).unwrap().expect("prior transition retained");
    assert_eq!(repaired.before_sha256, previous.after_sha256);
    assert_eq!(repaired.source_task_id, previous.source_task_id);
    assert_saved_transition_chain(&h, repaired);
    assert!(repaired.approved_files.contains(&"todo.js".to_string()));
    assert_eq!(repaired.after_sha256, crate::engine::digest_project_file(&h.root, "todo.js").unwrap().sha256);
    assert!(h.store().snapshot().unwrap().events.iter().any(|event| event.id.to_string() == repaired.integration_event && event.kind == "task_integrated" && event.task_id.as_deref() == Some(&repair.id)));
    let reviews: Vec<crate::model::DeliverableReview> = h.store().list("deliverable_review").unwrap();
    assert!(reviews.iter().any(|item| item.goal_id == next && item.round == 0 && item.verdict == "FAIL"));
    assert!(reviews.iter().any(|item| item.goal_id == next && item.round == 1 && item.verdict == "PASS"), "the repaired current artifact received a new independent QA verdict");
    assert!(reviews.iter().any(|item| item.goal_id == next && item.task_id == previous.task_id && item.round == 1 && item.verdict == "PASS"), "QA independently reviewed the original deliverable after its repair");
}

#[test]
fn linked_reuse_external_change_is_not_promoted_to_integration_evidence() {
    let Some(h) = reuse_harness(&[]) else { return };
    let first = h.submit(GOAL);
    h.wait(&first, &["awaiting_approval"]);
    h.approve(&first);
    h.wait(&first, &["completed"]);
    std::fs::write(h.root.join("todo.js"), "// external change without approval\n").unwrap();
    let next = linked_reuse_goal(&h, &first);
    h.wait(&next, &["awaiting_approval"]);
    let conflicts: Vec<crate::model::ReuseConflict> = h.store().list("reuse_conflict").unwrap();
    assert!(conflicts.iter().any(|item| item.path == "todo.js" && item.kind == "unknown_change"), "external bytes have no task/approval/event provenance: {conflicts:?}");
    let conflict = snapshot_reuse_conflict(&h, &next, "todo.js", "unknown_change");
    assert!(conflict.task_id.is_none() && conflict.approval_id.is_none());
    assert_ne!(conflict.before_sha256, conflict.current_sha256);
    assert!(conflict.after_sha256.is_none() && conflict.approved_files.is_empty());
    assert!(conflict.diff.as_deref().is_some_and(|diff| diff.contains("external change")));
    assert!(h.store().list::<crate::model::ReuseFileTransition>("reuse_file_transition").unwrap().is_empty());
}

#[test]
fn linked_reuse_undeclared_edit_is_blocked_and_exposed_in_the_snapshot() {
    let Some(h) = reuse_harness(&[]) else { return };
    let first = h.submit(GOAL);
    h.wait(&first, &["awaiting_approval"]);
    h.approve(&first);
    h.wait(&first, &["completed"]);
    let next = linked_reuse_goal_with_intent(&h, &first, "REUSE_REVISION REUSE_OUTSIDE: improve the todo rules");
    h.wait(&next, &["awaiting_approval"]);
    h.approve(&next);
    h.wait(&next, &["blocked"]);
    let conflict = snapshot_reuse_conflict(&h, &next, "index.html", "approval_scope_conflict");
    assert!(!conflict.approved_files.iter().any(|file| file == "index.html"));
    assert!(conflict.task_id.is_some());
    assert!(conflict.before_sha256.is_some() && conflict.after_sha256.is_some());
    assert!(!h.store().list::<crate::model::ReuseFileTransition>("reuse_file_transition").unwrap().iter().any(|item| item.goal_id == next));
}

#[test]
fn linked_reuse_repair_scope_addition_waits_for_a_separate_approval() {
    let Some(h) = reuse_harness(&[]) else { return };
    let first = h.submit(GOAL);
    h.wait(&first, &["awaiting_approval"]);
    h.approve(&first);
    h.wait(&first, &["completed"]);
    enable_reuse_mock_modes(&h, "qa-fail-once,needs-scope-once");
    let next = linked_reuse_goal(&h, &first);
    h.wait(&next, &["awaiting_approval"]);
    h.approve(&next);
    let scope = wait_scope_approval(&h, &next);
    assert!(scope.reason.contains("README.md"));
    assert!(!h.store().list::<crate::model::ReuseFileTransition>("reuse_file_transition").unwrap().iter().any(|item| item.path == "README.md"));
    h.deny(&next);
    let deadline = Instant::now() + Duration::from_secs(30);
    while h.goal(&next).status != "blocked" {
        assert!(Instant::now() < deadline, "scope denial did not block");
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(h.goal(&next).block_kind, "needs_scope");
    assert!(!h.store().list::<crate::model::ReuseFileTransition>("reuse_file_transition").unwrap().iter().any(|item| item.path == "README.md"));
}

#[test]
fn linked_reuse_repair_cannot_borrow_another_tasks_old_file_approval() {
    let Some(h) = reuse_harness(&[]) else { return };
    let first = h.submit(GOAL);
    h.wait(&first, &["awaiting_approval"]);
    h.approve(&first);
    h.wait(&first, &["completed"]);
    enable_reuse_mock_modes(&h, "qa-fail-once,reuse-other-owner");
    let next = linked_reuse_goal(&h, &first);
    h.wait(&next, &["awaiting_approval"]);
    h.approve(&next);
    let scope = wait_scope_approval(&h, &next);
    assert!(scope.reason.contains("index.html"), "the other task's prior approval is insufficient: {}", scope.reason);
    assert!(!h.store().list::<crate::model::ReuseFileTransition>("reuse_file_transition").unwrap().iter().any(|item| item.path == "index.html"));
    h.deny(&next);
    let deadline = Instant::now() + Duration::from_secs(30);
    while h.goal(&next).status != "blocked" {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(h.goal(&next).block_kind, "needs_scope");
}

#[test]
fn linked_reuse_hash_tamper_after_integration_blocks_the_next_pass() {
    let Some(h) = reuse_harness(&[]) else { return };
    let first = h.submit(GOAL);
    h.wait(&first, &["awaiting_approval"]);
    h.approve(&first);
    h.wait(&first, &["completed"]);
    let second = linked_reuse_goal(&h, &first);
    h.wait(&second, &["awaiting_approval"]);
    h.approve(&second);
    h.wait(&second, &["completed"]);
    let prior: crate::model::ReuseFileTransition = h.store().list::<crate::model::ReuseFileTransition>("reuse_file_transition").unwrap().into_iter().find(|item| item.goal_id == second && item.path == "todo.js").unwrap();
    std::fs::write(h.root.join("todo.js"), "// tampered after integration\n").unwrap();
    let third = linked_reuse_goal(&h, &second);
    h.wait(&third, &["awaiting_approval"]);
    let conflicts: Vec<crate::model::ReuseConflict> = h.store().list("reuse_conflict").unwrap();
    assert!(conflicts.iter().any(|item| item.path == "todo.js" && item.kind == "evidence_hash_mismatch" && item.after_sha256.as_deref() == Some(&prior.after_sha256)), "tamper must be attributed to the transitioned file: {conflicts:?}");
    let conflict = snapshot_reuse_conflict(&h, &third, "todo.js", "evidence_hash_mismatch");
    assert_eq!(conflict.after_sha256.as_deref(), Some(prior.after_sha256.as_str()));
    assert_ne!(conflict.after_sha256, conflict.current_sha256);
    assert!(conflict.task_id.is_some() && conflict.approval_id.is_some());
    assert!(conflict.approved_files.iter().any(|file| file == "todo.js"));
    assert!(conflict.diff.as_deref().is_some_and(|diff| diff.contains("tampered after integration")));
    assert!(!h.store().list::<crate::model::ReuseFileTransition>("reuse_file_transition").unwrap().iter().any(|item| item.goal_id == third));
}

#[test]
fn linked_reuse_untouched_dependent_still_checks_its_original_file_proof() {
    let Some(h) = reuse_harness(&[]) else { return };
    let first = h.submit(GOAL);
    h.wait(&first, &["awaiting_approval"]);
    h.approve(&first);
    h.wait(&first, &["completed"]);
    let screen = h.tasks(&first).into_iter().find(|task| task.files.contains(&"index.html".to_string())).unwrap();
    let original: crate::model::TaskIntegrationProof = h.store().get("task_proof", &screen.id).unwrap().unwrap();
    let second = linked_reuse_goal(&h, &first);
    h.wait(&second, &["awaiting_approval"]);
    h.approve(&second);
    h.wait(&second, &["completed"]);
    std::fs::write(h.root.join("index.html"), "<!-- outside edit to reused dependent -->\n").unwrap();
    let third = linked_reuse_goal(&h, &second);
    h.wait(&third, &["awaiting_approval"]);
    let conflict = snapshot_reuse_conflict(&h, &third, "index.html", "unknown_change");
    assert_eq!(conflict.source_task_id, screen.id);
    assert_eq!(conflict.before_sha256.as_deref(), original.files.iter().find(|file| file.path == "index.html").map(|file| file.sha256.as_str()));
    assert_ne!(conflict.before_sha256, conflict.current_sha256);
    assert!(conflict.task_id.is_none() && conflict.approval_id.is_none());
    assert!(!h.store().list::<crate::model::ReuseFileTransition>("reuse_file_transition").unwrap().iter().any(|item| item.goal_id == third && item.path == "index.html"));
}

#[test]
fn linked_reuse_integration_failure_after_approval_creates_no_transition() {
    let Some(h) = reuse_harness(&[]) else { return };
    let first = h.submit(GOAL);
    h.wait(&first, &["awaiting_approval"]);
    h.approve(&first);
    h.wait(&first, &["completed"]);
    let next = linked_reuse_goal_with_intent(&h, &first, "REUSE_REVISION REUSE_FAILURE: improve the todo rules");
    h.wait(&next, &["awaiting_approval"]);
    h.approve(&next);
    let deadline = Instant::now() + Duration::from_secs(90);
    while h.goal(&next).status != "blocked" {
        assert!(Instant::now() < deadline, "integration conflict did not block: {}", h.goal(&next).status);
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(!h.event_kinds(&next).contains(&"task_integrated".to_string()));
    assert!(!h.store().list::<crate::model::ReuseFileTransition>("reuse_file_transition").unwrap().iter().any(|item| item.goal_id == next));
    let conflict = snapshot_reuse_conflict(&h, &next, "todo.js", "integration_conflict");
    assert!(conflict.approval_id.is_some(), "approval alone cannot create a transition");
    assert_eq!(conflict.current_sha256, conflict.before_sha256, "failed patch never becomes selected-project evidence");
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
    // ADR-0060: a criteria card, and a conversation with only CEO actions left.
    let Some(stuck) = harness(&["qa-fail-always"]) else { return };
    let stuck_id = stuck.submit(GOAL);
    stuck.wait(&stuck_id, &["awaiting_approval"]);
    stuck.approve(&stuck_id);
    let _ = criteria_card(&stuck, &stuck_id);
    write("criteria", serde_json::json!({ "snapshot": stuck.store().snapshot().unwrap(), "goalId": stuck_id }));
    let Some(ceo) = harness(&["objective-ceo"]) else { return };
    put_goal(&ceo, A, "blocked", "2026-01-01T00:00:01Z", "할 일 앱을 만들고 GitHub에 푸시해 주세요");
    put_goal(&ceo, B, "completed", "2026-01-01T00:00:02Z", "멈춘 이유를 찾아요");
    put_link(&ceo, A, B);
    crate::objective::check(ceo.app.handle(), A, "resume").unwrap();
    write("ceo", serde_json::json!({ "snapshot": ceo.store().snapshot().unwrap(), "goalId": B }));
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
    assert!(card.reason.contains("PM 검토:"), "Dev 요청은 PM 검토 후 CEO에게 와야 합니다: {}", card.reason);
    assert!(!h.root.join("todo.js").exists(), "the refused task is not integrated while the card waits");
    let request: crate::permissions::CommandRequest = h.store().get("command_request", &card.id).unwrap().unwrap();
    assert_eq!(request.rules.iter().map(|rule| (rule.key(), rule.class.as_str())).collect::<Vec<_>>(), vec![("Bash(node --test)".to_string(), "safe")], "PM selects only the needed command from a chained refusal");
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
fn pm_selection_keeps_unneeded_script_out_of_ceo_card() {
    let Some(h) = harness(&["needs-permission-once", "pm-extra-denial"]) else { return };
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    h.approve(&id);
    let card = wait_operation(&h, &id, crate::permissions::OPERATION);
    let request: crate::permissions::CommandRequest = h.store().get("command_request", &card.id).unwrap().unwrap();
    assert!(request.rules.iter().any(|rule| rule.prefix == "node --test"));
    assert!(!request.rules.iter().any(|rule| rule.prefix.contains("const") || rule.prefix.contains("if(s")));
    let safe = request.rules.iter().filter(|rule| rule.class == "safe").map(crate::permissions::Rule::key).collect::<Vec<_>>();
    crate::permissions_flow::decide(&h.store(), &card.id, &safe).unwrap();
    h.wait(&id, &["completed"]);
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
fn pm_can_send_a_denied_dev_command_to_qa_without_asking_ceo() {
    let Some(h) = harness(&["needs-permission-once", "pm-continue-without"]) else { return };
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    h.approve(&id);
    h.wait(&id, &["completed"]);
    assert!(h.event_kinds(&id).contains(&"permission_pm_reviewed".to_string()));
    assert!(!h.store().list::<Approval>("approval").unwrap().iter().any(|item| item.goal_id.as_deref() == Some(&id) && item.operation == crate::permissions::OPERATION));
    assert!(granted(&h, &id).is_empty(), "PM의 결정은 새 명령 권한을 만들지 않습니다");
}

#[test]
fn pm_retry_uses_no_new_grant_and_does_not_escalate_unselected_denials() {
    let Some(h) = harness(&["needs-permission-once", "pm-retry-existing"]) else { return };
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    h.approve(&id);
    h.wait(&id, &["completed"]);
    assert!(h.event_kinds(&id).contains(&"task_resumed_after_pm_review".to_string()));
    assert!(granted(&h, &id).is_empty(), "PM 재시도는 새 명령 권한을 만들지 않습니다");
    assert!(!h.store().list::<Approval>("approval").unwrap().iter().any(|item| item.goal_id.as_deref() == Some(&id) && item.operation == crate::permissions::OPERATION));
}

#[test]
fn malformed_pm_permission_review_does_not_request_unfiltered_commands() {
    let Some(h) = harness(&["needs-permission-once", "pm-malformed"]) else { return };
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    h.approve(&id);
    h.wait(&id, &["completed"]);
    assert!(granted(&h, &id).is_empty());
    assert!(h.event_kinds(&id).contains(&"permission_pm_reviewed".to_string()));
    assert!(!h.store().list::<Approval>("approval").unwrap().iter().any(|item| item.goal_id.as_deref() == Some(&id) && item.operation == crate::permissions::OPERATION));
}

#[test]
fn auto_mode_grants_an_exact_read_only_to_the_task() {
    let Some(h) = harness(&["needs-auto-read-once"]) else { return };
    let project: crate::model::Project = h.store().get("project", "selected").unwrap().unwrap();
    crate::permission_mode::set(&h.store(), "project", &project.id, "auto").unwrap();
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    assert_eq!(crate::permission_mode::effective(&h.store(), &id).unwrap(), "auto");
    crate::permission_mode::set(&h.store(), "conversation", &id, "manual").unwrap();
    assert_eq!(crate::permission_mode::effective(&h.store(), &id).unwrap(), "manual");
    crate::permission_mode::set(&h.store(), "conversation", &id, "inherit").unwrap();
    assert_eq!(crate::permission_mode::effective(&h.store(), &id).unwrap(), "auto");
    h.approve(&id);
    h.wait(&id, &["completed"]);
    assert!(h.event_kinds(&id).contains(&"command_auto_granted".to_string()));
    assert!(h.event_kinds(&id).contains(&"task_resumed_auto".to_string()));
    assert!(granted(&h, &id).is_empty(), "auto approval must stay task-local");
    assert!(!h.store().list::<Approval>("approval").unwrap().iter().any(|item| item.operation == crate::permissions::OPERATION));
}

#[test]
fn full_auto_only_grants_declared_file_deletion_in_a_task_copy() {
    let Some(h) = harness(&["needs-auto-remove-once"]) else { return };
    let project: crate::model::Project = h.store().get("project", "selected").unwrap().unwrap();
    crate::permission_mode::set(&h.store(), "project", &project.id, "full_auto").unwrap();
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    h.approve(&id);
    h.wait(&id, &["completed"]);
    assert!(h.event_kinds(&id).contains(&"command_auto_granted".to_string()));
    assert!(granted(&h, &id).is_empty());
}

#[test]
fn qa_auto_mode_only_resumes_an_exact_read_in_its_copy() {
    let Some(h) = harness(&["qa-needs-auto-read-once"]) else { return };
    let project: crate::model::Project = h.store().get("project", "selected").unwrap().unwrap();
    crate::permission_mode::set(&h.store(), "project", &project.id, "full_auto").unwrap();
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    h.approve(&id);
    h.wait(&id, &["completed"]);
    assert!(h.event_kinds(&id).contains(&"command_auto_granted".to_string()));
    assert!(!h.store().list::<Approval>("approval").unwrap().iter().any(|item| item.operation == crate::permissions::OPERATION));
    assert!(granted(&h, &id).is_empty());
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
fn a_missing_precondition_gets_a_repair_before_any_ceo_card_and_dropping_after_no_progress_completes() {
    let Some(h) = harness(&["qa-precondition"]) else { return };
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    h.approve(&id);
    let (card, request) = criteria_card(&h, &id);
    assert_eq!(h.goal(&id).review_loops_used, 1, "the team first tried to repair the failed file");
    assert!(h.tasks(&id).iter().any(|task| task.title.starts_with(crate::delegation::REPAIR_PREFIX)), "a repair ran before the card");
    let item = request.items.iter().find(|item| item.criterion == STUCK).unwrap();
    assert_eq!(item.kind, "no_progress");
    assert!(item.reason.contains("Git 이력"), "{}", item.reason);
    assert_eq!((item.proposal_action.as_str(), item.proposal_text.as_str()), ("replace", REPLACEMENT), "the planner's suggestion is on the card");
    assert!(card.reason.contains("재작업해도 그대로"));
    assert!(crate::criteria::decide(&h.store(), &card.id, &[decision(item, "replace", "두 줄\n기준")], false, false).is_err(), "a replacement is one line");
    crate::criteria::decide(&h.store(), &card.id, &[decision(item, "drop", "")], false, false).unwrap();
    let goal = wait_status(&h, &id, "completed");
    assert_eq!(goal.review_loops_used, 1);
    assert!(h.tasks(&id).iter().all(|task| !task.acceptance.iter().any(|text| text == STUCK)), "the dropped criterion is gone");
    let saved: crate::criteria::CriteriaRequest = h.store().get("criteria_request", &card.id).unwrap().unwrap();
    assert_eq!((saved.decisions[0].action.as_str(), saved.decisions[0].old.as_str()), ("drop", STUCK), "the change is recorded with the old text");
    assert!(h.event_kinds(&id).contains(&"criteria_changed".to_string()));
    assert!(crate::criteria::decide(&h.store(), &card.id, &[], false, false).is_err(), "a decided card cannot be decided again");
}

#[test]
fn ceo_can_keep_a_falsely_blocked_criterion_and_request_a_file_repair() {
    let Some(h) = harness(&["qa-precondition"]) else { return };
    let id = h.submit(GOAL);
    h.wait(&id, &["awaiting_approval"]);
    h.approve(&id);
    let (card, request) = criteria_card(&h, &id);
    assert_eq!(h.goal(&id).review_loops_used, 1);
    assert!(crate::criteria::decide(&h.store(), &card.id, &[decision(&request.items[0], "drop", "")], false, true).is_err(), "repair must preserve the criterion");
    crate::criteria::decide(&h.store(), &card.id, &[], false, true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    while h.goal(&id).review_loops_used == 1 && Instant::now() < deadline { std::thread::sleep(Duration::from_millis(100)); }
    assert_eq!(h.goal(&id).review_loops_used, 2, "the existing goal starts another repair instead of rechecking unchanged files");
    assert!(h.tasks(&id).iter().any(|task| task.title.starts_with(crate::delegation::REPAIR_PREFIX)));
    assert!(h.tasks(&id).iter().any(|task| task.acceptance.iter().any(|text| text == STUCK)), "the original criterion remains");
    assert!(h.event_kinds(&id).contains(&"criteria_repair_requested".to_string()));
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
    crate::criteria::decide(&h.store(), &card.id, &[], true, false).unwrap();
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
    crate::criteria::decide(&h.store(), &card.id, &[decision(item, "replace", REPLACEMENT)], false, false).unwrap();
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
