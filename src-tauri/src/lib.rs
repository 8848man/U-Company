mod delegation;
mod discovery;
mod engine;
mod git_bootstrap;
mod meeting;
mod mock_ai;
mod goal_result;
#[cfg(test)]
mod engine_e2e;
mod model;
mod platform;
mod presets;
mod provider;
pub mod provider_harness;
mod runtime;
mod spec_import;
mod sdd_origin;
mod plan_review;
mod team_fix;
mod references;
mod curation;
mod squad;
mod archive;
mod storage;

use model::{Agent, AiConnection, Approval, DiscoveredAi, Goal, GoalGuidance, GoalTurn, Meeting, Project, RolePreset, Snapshot, Task};
use storage::{NewEvent, Store};
use tauri::{Emitter, Manager};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct BackendInfo { protocol_version: u32, build_id: String }

#[tauri::command]
fn get_backend_info() -> BackendInfo { BackendInfo { protocol_version: 39, build_id: meeting::meeting_build() } }

#[tauri::command]
fn get_snapshot(store: tauri::State<'_, Store>) -> Result<Snapshot, String> { store.snapshot() }

#[tauri::command]
fn discover_ai(store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Vec<DiscoveredAi> {
    let results = discovery::discover();
    let mut changed = false;
    for mut saved in store.list::<AiConnection>("connection").unwrap_or_default().into_iter().filter(|item| item.kind == "cli" && ["codex", "claude", "gemini", "opencode"].contains(&item.id.as_str())) {
        let checked = results.iter().find(|item| item.connection.id == saved.id && item.connection.executable == saved.executable)
            .cloned().unwrap_or_else(|| discovery::inspect_cli_connection(&saved));
        if saved.status != checked.connection.status || saved.capabilities != checked.connection.capabilities {
            saved.status = checked.connection.status;
            saved.capabilities = checked.connection.capabilities;
            if store.put("connection", &saved.id.clone(), &saved, None).is_ok() { changed = true; }
        }
    }
    if changed { let _ = app.emit("state-changed", ()); }
    results
}

#[tauri::command]
fn check_connection(id: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<DiscoveredAi, String> {
    let mut saved: AiConnection = store.get("connection", &id)?.ok_or("등록된 AI 연결을 찾지 못했습니다")?;
    if saved.kind != "cli" { return Err("CLI 연결만 이 방법으로 검사할 수 있습니다".into()); }
    let checked = discovery::inspect_cli_connection(&saved);
    saved.status = checked.connection.status.clone();
    saved.capabilities = checked.connection.capabilities.clone();
    store.put("connection", &saved.id.clone(), &saved, None)?;
    let _ = app.emit("state-changed", ());
    Ok(checked)
}

#[tauri::command]
fn start_connection_login(id: String, store: tauri::State<'_, Store>) -> Result<String, String> {
    let saved: AiConnection = store.get("connection", &id)?.ok_or("등록된 AI 연결을 찾지 못했습니다")?;
    discovery::start_login(&saved)
}

#[tauri::command]
fn list_connection_models(connection: AiConnection) -> Result<Vec<String>, String> { discovery::list_models(&connection) }

#[tauri::command]
fn get_role_presets() -> Vec<RolePreset> { presets::all() }

#[tauri::command]
fn preview_spec_import(project_id: String, sources: Vec<spec_import::SpecSource>, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<spec_import::SpecDraft, String> {
    let _: Project = store.get("project_entry", &project_id)?.ok_or("등록된 프로젝트를 선택해 주세요")?;
    let revision = store.get::<spec_import::SpecProfile>("spec_profile", &project_id)?.map(|profile| profile.revision).unwrap_or(0);
    let draft = spec_import::preview(&project_id, revision, sources)?;
    store.stage_spec_import(&draft)?;
    let _ = app.emit("state-changed", ());
    Ok(draft)
}

#[tauri::command]
fn apply_spec_import(draft_id: String, bindings: Vec<storage::SpecAgentBinding>, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<spec_import::SpecProfile, String> {
    let profile = store.apply_spec_import(&draft_id, &bindings)?;
    let _ = app.emit("state-changed", ());
    Ok(profile)
}

#[tauri::command]
fn get_portable_spec_status(project_id: String, store: tauri::State<'_, Store>) -> Result<spec_import::PortableStatus, String> {
    let project: Project = store.get("project_entry", &project_id)?.ok_or("등록된 프로젝트를 선택해 주세요")?;
    let profile = store.get::<spec_import::SpecProfile>("spec_profile", &project_id)?;
    Ok(spec_import::portable_status(&project, profile.as_ref()))
}

#[tauri::command]
fn preview_portable_spec(project_id: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<spec_import::SpecDraft, String> {
    let project: Project = store.get("project_entry", &project_id)?.ok_or("등록된 프로젝트를 선택해 주세요")?;
    let revision = store.get::<spec_import::SpecProfile>("spec_profile", &project_id)?.map(|profile| profile.revision).unwrap_or(0);
    let draft = spec_import::preview_portable(&project, revision)?;
    store.stage_spec_import(&draft)?;
    let _ = app.emit("state-changed", ());
    Ok(draft)
}

#[tauri::command]
fn create_portable_spec(project_id: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<spec_import::PortableStatus, String> {
    let project: Project = store.get("project_entry", &project_id)?.ok_or("등록된 프로젝트를 선택해 주세요")?;
    let profile = store.get::<spec_import::SpecProfile>("spec_profile", &project_id)?.ok_or("먼저 프로젝트 명세를 적용해 주세요")?;
    let status = spec_import::create_portable(&project, &profile)?;
    let _ = app.emit("state-changed", ());
    Ok(status)
}

#[tauri::command]
fn get_sdd_origin_status(project_id: String, store: tauri::State<'_, Store>) -> Result<sdd_origin::OriginStatus, String> {
    let project: Project = store.get("project_entry", &project_id)?.ok_or("등록된 프로젝트를 선택해 주세요")?;
    let mut status = sdd_origin::status(&project)?;
    status.muted = sdd_origin::muted(&store, &project.id);
    Ok(status)
}

#[tauri::command]
fn plan_sdd_origin(project_id: String, action: String, store: tauri::State<'_, Store>) -> Result<sdd_origin::OriginPlan, String> {
    let project: Project = store.get("project_entry", &project_id)?.ok_or("등록된 프로젝트를 선택해 주세요")?;
    sdd_origin::plan(&project, &action)
}

#[tauri::command]
fn apply_sdd_origin(project_id: String, action: String, digest: String, confirmed: bool, goal_id: Option<String>, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<sdd_origin::OriginStatus, String> {
    let project: Project = store.get("project_entry", &project_id)?.ok_or("등록된 프로젝트를 선택해 주세요")?;
    // ADR-0045: from the planning card, check the question is still open before writing anything.
    let origin = match &goal_id { Some(id) => Some(open_goal_origin(&store, id, &project.id)?), None => None };
    let mut status = sdd_origin::apply(&project, &action, &digest, confirmed)?;
    status.muted = sdd_origin::muted(&store, &project.id);
    if let Some(origin) = origin { record_origin_answer(&store, origin, &format!("applied:{action}"))?; }
    let _ = app.emit("state-changed", ());
    Ok(status)
}

fn open_goal_origin(store: &Store, goal_id: &str, project_id: &str) -> Result<model::GoalOrigin, String> {
    let origin: model::GoalOrigin = store.get("goal_origin", goal_id)?.ok_or("이 목표에는 구조 질문이 없습니다")?;
    sdd_origin::check_answerable(&origin, project_id)?;
    Ok(origin)
}

fn record_origin_answer(store: &Store, mut origin: model::GoalOrigin, decision: &str) -> Result<(), String> {
    origin.decision = Some(decision.into());
    origin.decided_at = Some(model::now());
    let message = format!("CEO answered the structure question: {decision}");
    store.put("goal_origin", &origin.goal_id.clone(), &origin, Some(storage::NewEvent { kind: "goal_origin_answered".into(), goal_id: Some(origin.goal_id.clone()), task_id: None, agent_id: None, message }))
}

/// ADR-0045: answer the planning card's structure question without writing project files.
/// `muted` also turns on "이 폴더는 묻지 않기" for the goal's project. Neither answer touches plan approval.
#[tauri::command]
fn decide_goal_origin(goal_id: String, decision: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<(), String> {
    if decision != "declined" && decision != "muted" { return Err("알 수 없는 답입니다".into()); }
    let saved: model::GoalOrigin = store.get("goal_origin", &goal_id)?.ok_or("이 목표에는 구조 질문이 없습니다")?;
    let origin = open_goal_origin(&store, &goal_id, &saved.project_id)?;
    if decision == "muted" { store.put("sdd_origin_mute", &origin.project_id.clone(), &model::OriginMute { project_id: origin.project_id.clone(), muted: true, changed_at: model::now() }, None)?; }
    record_origin_answer(&store, origin, &decision)?;
    let _ = app.emit("state-changed", ());
    Ok(())
}

/// 설정 › 프로젝트 명세 shows and can undo "이 폴더는 묻지 않기".
#[tauri::command]
fn set_sdd_origin_mute(project_id: String, muted: bool, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<(), String> {
    let _: Project = store.get("project_entry", &project_id)?.ok_or("등록된 프로젝트를 선택해 주세요")?;
    store.put("sdd_origin_mute", &project_id, &model::OriginMute { project_id: project_id.clone(), muted, changed_at: model::now() }, Some(storage::NewEvent { kind: "sdd_origin_mute_changed".into(), goal_id: None, task_id: None, agent_id: None, message: if muted { "Structure question muted for this project".into() } else { "Structure question enabled for this project".into() } }))?;
    let _ = app.emit("state-changed", ());
    Ok(())
}

#[tauri::command]
fn create_portable_spec_from_draft(draft_id: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<spec_import::PortableStatus, String> {
    let draft: spec_import::SpecDraft = store.get("spec_draft", &draft_id)?.ok_or("명세 초안을 찾지 못했습니다. 다시 미리보기를 만들어 주세요")?;
    let project: Project = store.get("project_entry", &draft.project_id)?.ok_or("등록된 프로젝트를 선택해 주세요")?;
    let revision = store.get::<spec_import::SpecProfile>("spec_profile", &draft.project_id)?.map(|profile| profile.revision).unwrap_or(0);
    if revision != draft.base_revision { return Err("명세가 변경되었습니다. 다시 미리보기를 확인해 주세요".into()); }
    let status = spec_import::create_portable(&project, &draft.profile)?;
    let _ = app.emit("state-changed", ());
    Ok(status)
}

#[tauri::command]
fn pick_project_folder() -> Result<Option<String>, String> { platform::pick_folder() }

#[tauri::command]
fn set_project(path: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<Project, String> {
    let discovered = platform::adopt_project(&path)?;
    let project = match store.list::<Project>("project_entry")?.into_iter().find(|existing| existing.path.eq_ignore_ascii_case(&discovered.path)) {
        Some(existing) => platform::refresh_registered_project(existing, discovered),
        None => discovered,
    };
    store.select_project(&project)?;
    let _ = app.emit("state-changed", ());
    Ok(project)
}

#[tauri::command]
fn switch_project(id: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<Project, String> {
    let project: Project = store.get("project_entry", &id)?.ok_or("Project is not registered in this workspace")?;
    let actual = platform::adopt_project(&project.path).map_err(|error| format!("This project is unavailable: {error}"))?;
    if !actual.path.eq_ignore_ascii_case(&project.path) { return Err("Project folder location changed".into()); }
    let project = platform::refresh_registered_project(project, actual);
    store.select_project(&project)?;
    let _ = app.emit("state-changed", ());
    Ok(project)
}

#[tauri::command]
fn set_project_review_limit(id: String, limit: u32, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<Project, String> {
    if limit > 5 { return Err("재검토 반복 횟수는 0~5회여야 합니다".into()); }
    let mut project: Project = store.get("project_entry", &id)?.ok_or("Project is not registered")?;
    project.review_loop_limit = limit;
    store.put("project_entry", &id, &project, None)?;
    if store.get::<Project>("project", "selected")?.as_ref().is_some_and(|selected| selected.id == id) {
        store.select_project(&project)?;
    }
    let _ = app.emit("state-changed", ());
    Ok(project)
}

#[tauri::command]
fn set_goal_review_limit(id: String, limit: u32, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<Goal, String> {
    if limit > 5 { return Err("재검토 반복 횟수는 0~5회여야 합니다".into()); }
    let mut goal: Goal = store.get("goal", &id)?.ok_or("Goal not found")?;
    if goal.kind != "execution" { return Err("회의 목표에는 개발 QA 한도가 없습니다".into()); }
    goal.review_loop_limit = Some(limit);
    store.put("goal", &id, &goal, Some(NewEvent::goal("goal_review_limit_changed", &goal, &format!("QA 재작업 최대 {limit}회"))))?;
    let _ = app.emit("state-changed", ());
    Ok(goal)
}

#[tauri::command]
fn set_goal_work_group(id: String, group: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<Goal, String> {
    let label = group.trim();
    if label.chars().count() > 80 { return Err("업무 묶음 이름은 80자 이하여야 합니다".into()); }
    let mut goal: Goal = store.get("goal", &id)?.ok_or("Goal not found")?;
    if goal.kind != "execution" { return Err("회의 목표에는 실행 업무 묶음을 설정할 수 없습니다".into()); }
    goal.work_group = if label.is_empty() { None } else { Some(label.into()) };
    store.put("goal", &id, &goal, Some(NewEvent::goal("goal_group_changed", &goal, "Goal display group changed")))?;
    let _ = app.emit("state-changed", ());
    Ok(goal)
}

#[tauri::command]
fn create_project(parent_path: String, name: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<Project, String> {
    let project = platform::create_project(&parent_path, &name)?;
    store.select_project(&project)?;
    let _ = app.emit("state-changed", ());
    Ok(project)
}

#[tauri::command]
fn save_agent(mut agent: Agent, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<Agent, String> {
    // ADR-0041: the contract and slot role of external staff change only through their own actions.
    let saved: Option<Agent> = store.get("agent", &agent.id)?;
    agent.external = saved.as_ref().and_then(|item| item.external.clone());
    if let Some(saved) = saved.as_ref().filter(|item| item.is_external()) {
        agent.role = saved.role.clone();
        if agent.enabled && !saved.enabled { return Err("계약이 끝난 외부 인력은 '정식 채용'으로만 다시 쓸 수 있습니다".into()); }
    }
    if agent.external.is_none() && agent.id.starts_with("ext-") { return Err("외부 인력은 캐릭터 파일 가져오기로만 추가할 수 있습니다".into()); }
    if let Some(appearance) = &agent.appearance { appearance.validate()?; }
    if agent.name.trim().is_empty() || agent.role.trim().is_empty() { return Err("Name and role are required".into()); }
    if !(0..=100).contains(&agent.x) || !(0..=100).contains(&agent.y) { return Err("Office position must be 0–100".into()); }
    if !agent.enabled && store.get::<Agent>("agent", &agent.id)?.as_ref().is_some_and(|saved| saved.enabled) {
        let unfinished = store.unfinished_task_count(&agent.id)?;
        if unfinished > 0 { return Err(format!("{}에게 끝나지 않은 업무 {}개가 있습니다. 업무를 완료하거나 재배정한 뒤 숨겨 주세요.", agent.name, unfinished)); }
    }
    store.put("agent", &agent.id, &agent, None)?;
    let _ = app.emit("state-changed", ());
    Ok(agent)
}

#[tauri::command]
fn assign_team_connection(connection_id: String, agent_ids: Vec<String>, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<usize, String> {
    let count = store.assign_connection_to_agents(&connection_id, &agent_ids)?;
    let _ = app.emit("state-changed", ());
    Ok(count)
}

#[tauri::command]
fn save_connection(mut connection: AiConnection, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<AiConnection, String> {
    if !["cli", "http", "plugin"].contains(&connection.kind.as_str()) { return Err("Unsupported connection type".into()); }
    if connection.kind == "cli" {
        if let Some(repaired) = discovery::repaired_known_windows_cli_path(std::path::Path::new(&connection.executable), &connection.id) {
            connection.executable = repaired.to_string_lossy().into_owned();
        }
        discovery::runtime_command(&connection)?;
    }
    if connection.kind == "http" && !connection.endpoint.starts_with("http://127.0.0.1:") && !connection.endpoint.starts_with("http://localhost:") && !connection.endpoint.starts_with("https://") {
        return Err("Use a loopback HTTP or HTTPS endpoint".into());
    }
    if connection.kind != "http" && !std::path::Path::new(&connection.executable).is_file() { return Err(format!("실행 파일이 없습니다: {}. 자동 발견 결과를 다시 확인하거나 정확한 경로를 입력하세요", connection.executable)); }
    if connection.kind == "http" {
        let models = discovery::list_models(&connection)?;
        if !models.contains(&connection.model_id) { return Err("목록에서 사용 가능한 모델을 선택해 주세요".into()); }
        connection.status = "ready".into();
        connection.capabilities = vec!["text".into()];
    }
    if connection.kind == "cli" {
        let checked = discovery::inspect_cli_connection(&connection);
        connection.status = checked.connection.status;
        connection.capabilities = checked.connection.capabilities;
    }
    store.register_connection_and_assign_unassigned(&connection)?;
    let _ = app.emit("state-changed", ());
    Ok(connection)
}

/// ADR-0054: files a finished goal changed and plain ways to try them. Read-only.
#[tauri::command]
fn get_goal_result(goal_id: String, store: tauri::State<'_, Store>) -> Result<goal_result::GoalResult, String> { goal_result::result(&store, &goal_id) }

/// ADR-0054: where this project's goals stopped, by kind.
#[tauri::command]
fn get_block_stats(project_id: String, store: tauri::State<'_, Store>) -> Result<Vec<goal_result::BlockStat>, String> {
    Ok(goal_result::block_stats(&store.list::<goal_result::BlockRecord>("goal_block")?, &project_id))
}

/// ADR-0054: opens a registered project's folder in the system file manager.
#[tauri::command]
fn open_project_folder(project_id: String, store: tauri::State<'_, Store>) -> Result<(), String> {
    let project: Project = store.get("project_entry", &project_id)?.ok_or("프로젝트를 찾을 수 없습니다")?;
    goal_result::open_folder(&project.path)
}

/// ADR-0052: register the token-free mock AI (a `plugin` connection running the repository's adapter).
/// Like any new connection it fills only teammates that have no connection yet.
#[tauri::command]
fn add_mock_connection(modes: Vec<String>, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<AiConnection, String> {
    let connection = mock_ai::local_connection(&modes)?;
    store.register_connection_and_assign_unassigned(&connection)?;
    let _ = app.emit("state-changed", ());
    Ok(connection)
}

/// ADR-0049: pasted text over 4,000 characters, or an attached `.md`/`.txt`, held until the goal is sent.
#[tauri::command]
fn stage_text_reference(name: String, text: String, source: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<references::Reference, String> {
    let project: Project = store.get("project", "selected")?.ok_or("먼저 프로젝트 폴더를 선택해 주세요")?;
    if source == "paste" && text.chars().count() <= references::INLINE_MAX { return Err("짧은 글은 목표 문장에 그대로 넣어 주세요".into()); }
    let reference = references::stage_text(&store, &curation::staging_root(&app)?, &project, &name, &text, &source)?;
    if reference.curation == "pending" { curation::start(app.clone(), reference.id.clone()); }
    let _ = app.emit("state-changed", ());
    Ok(reference)
}

/// ADR-0049: an image the screen already resized to WebP without metadata.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn stage_image_reference(name: String, data_base64: String, original_bytes: u64, original_sha256: String, caption: String, detail: bool, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<references::Reference, String> {
    let project: Project = store.get("project", "selected")?.ok_or("먼저 프로젝트 폴더를 선택해 주세요")?;
    if data_base64.len() > references::IMAGE_MAX_BYTES * 4 / 3 + 16 { return Err("변환한 이미지가 4MB를 넘어요".into()); }
    let bytes = references::decode_base64(&data_base64)?;
    let reference = references::stage_image(&store, &curation::staging_root(&app)?, &project, &bytes, references::ImageMeta { original_name: name, original_bytes, original_sha256, caption, detail })?;
    curation::start(app.clone(), reference.id.clone());
    let _ = app.emit("state-changed", ());
    Ok(reference)
}

#[tauri::command]
fn discard_reference(id: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<(), String> {
    references::discard_staged(&store, &curation::staging_root(&app)?, &id)?;
    let _ = app.emit("state-changed", ());
    Ok(())
}

/// ADR-0049: `요구사항` or `참고` on a chip; may start the curator when it is on.
#[tauri::command]
fn set_reference_role(id: String, role: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<references::Reference, String> {
    let (reference, start) = references::set_role(&store, &curation::staging_root(&app)?, &id, &role)?;
    if start { curation::start(app.clone(), reference.id.clone()); }
    if reference.status != "staged" {
        if let Some(project) = store.get::<Project>("project_entry", &reference.project_id)? { references::write_indexes(&store, &project)?; }
    }
    let _ = app.emit("state-changed", ());
    Ok(reference)
}

/// ADR-0049: the curator is off by default (an extra call per long text); the CEO turns it on.
#[tauri::command]
fn set_curator_enabled(enabled: bool, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<(), String> {
    references::set_curator_enabled(&store, enabled)?;
    let _ = app.emit("state-changed", ());
    Ok(())
}

#[tauri::command]
fn set_reference_caption(id: String, caption: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<references::Reference, String> {
    let reference = references::set_caption(&store, &id, &caption)?;
    if reference.status != "staged" {
        if let Some(project) = store.get::<Project>("project_entry", &reference.project_id)? { references::write_indexes(&store, &project)?; }
    }
    let _ = app.emit("state-changed", ());
    Ok(reference)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn submit_goal(intent: String, lineage_meeting_id: Option<String>, lineage_package_sha256: Option<String>, reference_ids: Option<Vec<String>>, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<Goal, String> {
    if intent.trim().len() < 10 { return Err("Describe the goal in at least 10 characters".into()); }
    let project: Project = store.get("project", "selected")?.ok_or("먼저 프로젝트 폴더를 선택해 주세요")?;
    let goal = Goal { id: model::id(), kind: "execution".into(), project_id: Some(project.id), intent: intent.trim().into(), status: "new".into(), report: String::new(), created_at: model::now(), resumed_from: None, review_loops_used: 0, review_loop_limit: Some(project.review_loop_limit), work_group: None, block_kind: String::new() };
    match (lineage_meeting_id, lineage_package_sha256) {
        (Some(meeting_id), Some(sha)) => meeting::submit_with_lineage(&store, &goal, &meeting_id, &sha)?,
        (None, None) => store.put("goal", &goal.id, &goal, Some(NewEvent::goal("goal_created", &goal, "CEO goal received")))?,
        _ => return Err("회의 인계 정보가 불완전합니다".into()),
    }
    // ADR-0049: attached material moves into the goal folder; the goal sentence gets one line per item.
    let reference_ids = reference_ids.unwrap_or_default();
    let mut goal = goal;
    if !reference_ids.is_empty() {
        let project: Project = store.get("project", "selected")?.ok_or("먼저 프로젝트 폴더를 선택해 주세요")?;
        match references::attach_to_goal(&store, &curation::staging_root(&app)?, &project, &goal, &reference_ids) {
            Ok(attached) => {
                let lines = attached.iter().map(|item| format!("- {} {}: {} ({})", item.label, if item.kind == "image" { "이미지" } else { "글" }, references::one_line(&item.title, 60), if item.kind == "image" { format!("{}×{}", item.width, item.height) } else { format!("{}자", item.chars) })).collect::<Vec<_>>().join("\n");
                goal.intent = format!("{}\n\n참조 자료 (목록: {}/INDEX.md, 필요한 것만 열어 보세요):\n{lines}", goal.intent, references::goal_folder(&goal));
                store.put("goal", &goal.id, &goal, None)?;
            }
            Err(error) => {
                store.transact(|tx| { tx.execute("DELETE FROM objects WHERE kind='goal' AND id=?1", [goal.id.as_str()]).map_err(|e| e.to_string())?; Ok(()) })?;
                let _ = app.emit("state-changed", ());
                return Err(format!("참조 자료를 목표에 넣지 못해 목표를 만들지 않았습니다: {error}"));
            }
        }
    }
    let _ = app.emit("state-changed", ());
    Ok(goal)
}

fn meeting_transport(store: &Store, meeting_id: Option<&str>) -> Result<meeting::ProviderTransport, String> {
    let project_id = match meeting_id {
        Some(id) => store.get::<Meeting>("meeting", id)?.ok_or("회의를 찾을 수 없습니다")?.project_id,
        None => store.get::<Project>("project", "selected")?.ok_or("먼저 프로젝트를 선택해 주세요")?.id,
    };
    let project: Project = store.get("project_entry", &project_id)?.ok_or("회의 프로젝트를 찾을 수 없습니다")?;
    let root = std::path::Path::new(&project.path).canonicalize().map_err(|e| format!("프로젝트 경로를 확인할 수 없습니다: {e}"))?;
    Ok(meeting::ProviderTransport { project_root: root })
}

/// Runs the meeting's approved packets off the UI thread (chair planning or debate stages).
fn spawn_meeting_dispatch(app: tauri::AppHandle, meeting_id: String) {
    std::thread::spawn(move || {
        let store = app.state::<Store>();
        let notify = || { let _ = app.emit("state-changed", ()); };
        let result = meeting_transport(&store, Some(&meeting_id)).and_then(|transport| meeting::run_dispatch(&store, &meeting_id, &transport, &notify));
        if let Err(error) = result {
            let _ = store.transact(|tx| { let reason = format!("회의 진행 중 오류: {error}"); let meeting: Option<Meeting> = storage::tx_get(tx, "meeting", &meeting_id)?; if let Some(meeting) = meeting { storage::tx_event(tx, "meeting_error", &meeting.goal_id, &reason)?; } Ok(()) });
            let _ = meeting::stop(&store, &meeting_id);
        }
        notify();
    });
}

#[tauri::command]
fn meeting_readiness(store: tauri::State<'_, Store>) -> Result<meeting::MeetingReadiness, String> {
    let transport = meeting_transport(&store, None)?;
    meeting::readiness(&store, &transport)
}

#[tauri::command]
fn start_meeting_from_home(topic: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<Meeting, String> {
    let transport = meeting_transport(&store, None)?;
    let created = meeting::start_from_home(&store, &topic, &transport)?;
    let _ = app.emit("state-changed", ());
    spawn_meeting_dispatch(app, created.id.clone());
    Ok(created)
}

#[tauri::command]
fn update_meeting_plan(meeting_id: String, base_revision: u32, plan: meeting::PlanInput, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<model::MeetingPlan, String> {
    let transport = meeting_transport(&store, Some(&meeting_id))?;
    let saved = meeting::update_plan(&store, &meeting_id, base_revision, &plan, &transport)?;
    let _ = app.emit("state-changed", ());
    Ok(saved)
}

#[tauri::command]
fn approve_meeting_plan(meeting_id: String, revision: u32, digest: String, mode: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<Meeting, String> {
    let transport = meeting_transport(&store, Some(&meeting_id))?;
    let approved = meeting::approve_plan(&store, &meeting_id, revision, &digest, &mode, &transport)?;
    let _ = app.emit("state-changed", ());
    if meeting::STAGES.contains(&approved.stage.as_str()) { spawn_meeting_dispatch(app, meeting_id); }
    Ok(approved)
}

#[tauri::command]
fn reject_meeting_plan(meeting_id: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<(), String> {
    meeting::reject_plan(&store, &meeting_id)?;
    let _ = app.emit("state-changed", ());
    Ok(())
}

#[tauri::command]
fn revoke_meeting_plan(meeting_id: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<(), String> {
    meeting::revoke_plan(&store, &meeting_id)?;
    let _ = app.emit("state-changed", ());
    Ok(())
}

#[tauri::command]
fn decide_transmission(approval_id: String, preview_revision: u32, payload_sha256: String, approved: bool, confirm_duplicate: bool, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<(), String> {
    let approval: model::TransmissionApproval = store.get("transmission_approval", &approval_id)?.ok_or("전송 승인 기록을 찾을 수 없습니다")?;
    let transport = meeting_transport(&store, Some(&approval.meeting_id))?;
    meeting::decide(&store, &approval_id, preview_revision, &payload_sha256, approved, confirm_duplicate, &transport)?;
    let _ = app.emit("state-changed", ());
    Ok(())
}

#[tauri::command]
fn dispatch_meeting(meeting_id: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<(), String> {
    // per_stage mode: explicit CEO command after every packet of the stage is approved.
    let transport = meeting_transport(&store, Some(&meeting_id))?;
    meeting::closed_text_dispatch(&store, &meeting_id, &transport)?;
    let _ = app.emit("state-changed", ());
    spawn_meeting_dispatch(app, meeting_id);
    Ok(())
}

#[tauri::command]
fn stop_meeting(meeting_id: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<(), String> {
    meeting::stop(&store, &meeting_id)?;
    let _ = app.emit("state-changed", ());
    Ok(())
}

#[tauri::command]
fn resume_meeting(meeting_id: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<(), String> {
    let transport = meeting_transport(&store, Some(&meeting_id))?;
    let replanning = meeting::resume(&store, &meeting_id, &transport)?;
    let _ = app.emit("state-changed", ());
    if replanning { spawn_meeting_dispatch(app, meeting_id); }
    Ok(())
}

#[tauri::command]
fn retry_meeting_packet(meeting_id: String, packet_id: String, confirm_duplicate: bool, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<(), String> {
    let run = meeting::retry_packet(&store, &meeting_id, &packet_id, confirm_duplicate)?;
    let _ = app.emit("state-changed", ());
    if run { spawn_meeting_dispatch(app, meeting_id); }
    Ok(())
}

#[tauri::command]
fn continue_meeting_source_snapshot(meeting_id: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<(), String> {
    let prepared = meeting::continue_source_snapshot(&store, &meeting_id)?;
    let _ = app.emit("state-changed", ());
    if meeting::STAGES.contains(&prepared.stage.as_str()) { spawn_meeting_dispatch(app, meeting_id); }
    Ok(())
}

#[tauri::command]
fn close_meeting_partial(meeting_id: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<(), String> {
    meeting::close_partial(&store, &meeting_id)?;
    let _ = app.emit("state-changed", ());
    Ok(())
}

#[tauri::command]
fn record_meeting_decision(meeting_id: String, kind: String, recommendation_ids: Vec<String>, note: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<Option<String>, String> {
    let draft = meeting::record_decision(&store, &meeting_id, &kind, &recommendation_ids, &note)?;
    let _ = app.emit("state-changed", ());
    Ok(draft)
}

#[tauri::command]
fn ask_goal(goal_id: String, agent_id: String, question: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<GoalTurn, String> {
    let question = question.trim();
    if question.chars().count() < 4 || question.chars().count() > 4000 { return Err("질문은 4~4000자로 적어 주세요".into()); }
    let goal: Goal = store.get("goal", &goal_id)?.ok_or("목표를 찾을 수 없습니다")?;
    if goal.kind != "execution" { return Err("회의 목표는 기존 팀 대화를 사용할 수 없습니다".into()); }
    if goal.project_id.is_none() { return Err("프로젝트가 연결되지 않은 목표에는 질문할 수 없습니다".into()); }
    let snapshot = store.snapshot()?;
    let (agent, selected) = engine::discussion_runtime(&snapshot.agents, &snapshot.connections, &agent_id)?;
    let turn = GoalTurn { id: model::id(), goal_id, question: question.into(), answer: String::new(), status: "pending".into(), asked_at: model::now(), answered_at: None, agent_id: agent.id.clone(), agent_name: agent.name.clone(), connection_name: selected.name.clone(), model_id: selected.model_id.clone(), guidance_id: None, route: None };
    store.enqueue_goal_turn(&turn)?;
    let _ = app.emit("state-changed", ());
    let turn_id = turn.id.clone();
    std::thread::spawn(move || engine::answer_goal_question(&app, &turn_id, agent, selected));
    Ok(turn)
}

#[tauri::command]
fn send_goal_message(goal_id: String, agent_id: String, message: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<GoalTurn, String> {
    let message = message.trim();
    if message.chars().count() < 4 || message.chars().count() > 4000 { return Err("메시지는 4~4000자로 적어 주세요".into()); }
    let goal: Goal = store.get("goal", &goal_id)?.ok_or("목표를 찾을 수 없습니다")?;
    if goal.kind != "execution" { return Err("회의 목표는 기존 팀 대화를 사용할 수 없습니다".into()); }
    if goal.project_id.is_none() { return Err("프로젝트가 연결되지 않은 목표에는 메시지를 보낼 수 없습니다".into()); }
    let snapshot = store.snapshot()?;
    let (agent, selected) = engine::discussion_runtime(&snapshot.agents, &snapshot.connections, &agent_id)?;
    let turn = GoalTurn { id: model::id(), goal_id, question: message.into(), answer: String::new(), status: "pending".into(), asked_at: model::now(), answered_at: None, agent_id: agent.id.clone(), agent_name: agent.name.clone(), connection_name: selected.name.clone(), model_id: selected.model_id.clone(), guidance_id: None, route: None };
    store.enqueue_goal_message(&turn)?;
    let _ = app.emit("state-changed", ());
    let turn_id = turn.id.clone();
    std::thread::spawn(move || engine::answer_goal_message(&app, &turn_id, agent, selected));
    Ok(turn)
}

/// ADR-0048: the recovery card's one-click fix request.
#[tauri::command]
fn request_team_fix(goal_id: String, app: tauri::AppHandle) -> Result<GoalTurn, String> {
    engine::request_team_fix(app, goal_id)
}

#[tauri::command]
fn propose_goal_change(goal_id: String, text: String, agent_id: Option<String>, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<GoalGuidance, String> {
    let text = text.trim();
    if text.chars().count() < 10 || text.chars().count() > 4000 { return Err("방향 변경 제안은 10~4000자로 적어 주세요".into()); }
    let guidance = store.propose_goal_guidance(&goal_id, text, agent_id.as_deref(), false)?;
    let _ = app.emit("state-changed", ());
    Ok(guidance)
}

#[tauri::command]
fn create_goal_revision(guidance_id: String, intent: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<Goal, String> {
    let intent = intent.trim();
    if intent.chars().count() < 10 || intent.chars().count() > 12000 { return Err("새 목표는 10~12000자로 적어 주세요".into()); }
    let revised = store.create_goal_revision(&guidance_id, intent)?;
    let _ = app.emit("state-changed", ());
    Ok(revised)
}

#[tauri::command]
fn preview_git_bootstrap(goal_id: String, store: tauri::State<'_, Store>) -> Result<git_bootstrap::Preview, String> {
    let goal: Goal = store.get("goal", &goal_id)?.ok_or("목표를 찾을 수 없습니다")?;
    if goal.kind != "execution" { return Err("회의 목표는 Git 초기화 복구 대상이 아닙니다".into()); }
    if goal.status != "blocked" { return Err("중단된 Git 초기화 업무에서만 이 복구 작업을 사용할 수 있습니다".into()); }
    let tasks: Vec<Task> = store.list::<Task>("task")?.into_iter().filter(|task| task.goal_id == goal_id).collect();
    if !engine::bootstrap_recovery_allowed(&tasks) { return Err("Git 초기화에 실패한 단일 업무가 확인되지 않았습니다. 업무 범위를 검토해 주세요".into()); }
    let project_id = goal.project_id.ok_or("프로젝트가 연결되지 않은 목표입니다")?;
    let project: Project = store.get("project_entry", &project_id)?.ok_or("등록된 프로젝트를 찾을 수 없습니다")?;
    let selected: Project = store.get("project", "selected")?.ok_or("프로젝트를 선택해 주세요")?;
    if selected.id != project.id { return Err("이 목표의 프로젝트를 먼저 선택해 주세요".into()); }
    git_bootstrap::preview(&project)
}

#[tauri::command]
fn bootstrap_project_git(goal_id: String, fingerprint: String, selected_files: Vec<String>, author_name: String, author_email: String, message: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<git_bootstrap::ResultInfo, String> {
    let goal: Goal = store.get("goal", &goal_id)?.ok_or("목표를 찾을 수 없습니다")?;
    if goal.kind != "execution" { return Err("회의 목표는 Git 초기화 복구 대상이 아닙니다".into()); }
    if goal.status != "blocked" { return Err("중단된 Git 초기화 업무에서만 이 복구 작업을 사용할 수 있습니다".into()); }
    let tasks: Vec<Task> = store.list::<Task>("task")?.into_iter().filter(|task| task.goal_id == goal_id).collect();
    if !engine::bootstrap_recovery_allowed(&tasks) { return Err("Git 초기화에 실패한 단일 업무가 확인되지 않았습니다. 업무 범위를 검토해 주세요".into()); }
    let project_id = goal.project_id.as_deref().ok_or("프로젝트가 연결되지 않은 목표입니다")?;
    let project: Project = store.get("project_entry", project_id)?.ok_or("등록된 프로젝트를 찾을 수 없습니다")?;
    let selected_project: Project = store.get("project", "selected")?.ok_or("프로젝트를 선택해 주세요")?;
    if selected_project.id != project.id || project.kind != "folder" { return Err("선택한 일반 폴더 프로젝트에서만 초기화할 수 있습니다".into()); }
    git_bootstrap::validate_request(&project, &fingerprint, &selected_files, &author_name, &author_email, &message)?;
    let approval = Approval { id: model::id(), goal_id: Some(goal.id.clone()), operation: "git_bootstrap".into(), reason: format!("{}에서 main 초기화 및 첫 커밋 승인. 파일: {}. 작성자: {} <{}>. 메시지: {}. 미리보기: {}", project.path, selected_files.join(", "), author_name, author_email, message, fingerprint), status: "approved".into(), created_at: model::now() };
    store.put("approval", &approval.id, &approval, Some(NewEvent { kind: "approval_decided".into(), goal_id: Some(goal.id.clone()), task_id: None, agent_id: None, message: "CEO approved exact Git bootstrap scope".into() }))?;
    let outcome = git_bootstrap::bootstrap(&project, &fingerprint, &selected_files, &author_name, &author_email, &message);
    match outcome {
        Ok(mut result) => {
            let actual = platform::adopt_project(&project.path)?;
            let refreshed = platform::refresh_registered_project(project, actual);
            store.select_project(&refreshed)?;
            store.append_event(NewEvent::goal("git_bootstrap_completed", &goal, &format!("main 첫 커밋 {} · {}개 파일", result.commit_id, result.committed_files.len())))?;
            result.qa_started = match engine::verify_bootstrap_goal(app.clone(), &goal.id, &result.commit_id, &result.committed_files) {
                Ok(true) => true,
                Ok(false) => {
                    let _ = store.append_event(NewEvent::goal("git_bootstrap_qa_unavailable", &goal, "커밋 파일이 실패 업무의 선언 파일과 다르거나 다른 미완료 업무가 있어 자동 QA를 시작하지 않았습니다. 목표 대화에서 남은 업무 범위를 조정해 주세요"));
                    false
                }
                Err(error) => {
                    let _ = store.append_event(NewEvent::goal("git_bootstrap_qa_unavailable", &goal, &error));
                    false
                }
            };
            let _ = app.emit("state-changed", ());
            Ok(result)
        }
        Err(error) => {
            let _ = store.append_event(NewEvent::goal("git_bootstrap_failed", &goal, &error));
            let _ = app.emit("state-changed", ());
            Err(error)
        }
    }
}

#[tauri::command]
fn save_task(task: Task, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<(), String> {
    if task.title.trim().is_empty() { return Err("Task title is required".into()); }
    let goal: Option<Goal> = store.get("goal", &task.goal_id)?;
    if goal.is_none() { return Err("Goal not found".into()); }
    if goal.as_ref().is_some_and(|item| item.kind != "execution") { return Err("회의 목표에는 개발 업무를 저장할 수 없습니다".into()); }
    let agent: Option<Agent> = store.get("agent", &task.assignee_id)?;
    if agent.is_none() { return Err("Assignee not found".into()); }
    if task.depends_on.contains(&task.id) { return Err("Task cannot depend on itself".into()); }
    store.put("task", &task.id, &task, Some(NewEvent::task("task_saved", &task, &task.title)))?;
    let _ = app.emit("state-changed", ());
    Ok(())
}

#[tauri::command]
fn decide_approval(id: String, approved: bool, squad_digest: Option<String>, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<(), String> {
    let approval: Approval = store.get("approval", &id)?.ok_or("Approval not found")?;
    if let Some(goal_id) = &approval.goal_id {
        let goal: Goal = store.get("goal", goal_id)?.ok_or("승인 목표를 찾을 수 없습니다")?;
        if goal.kind != "execution" { return Err("회의 전송은 별도 승인 명령을 사용하세요".into()); }
    }
    if approval.status != "pending" { return Err("Approval already decided".into()); }
    if approval.goal_id.as_ref().is_some_and(|goal_id| store.list::<GoalGuidance>("goal_guidance").map(|items| items.iter().any(|item| &item.goal_id == goal_id && ["pending", "review_required"].contains(&item.status.as_str()))).unwrap_or(true)) { return Err("CEO 방향 변경 제안이 있어 기존 계획을 승인할 수 없습니다. 목표 대화에서 새 버전을 검토해 주세요".into()); }
    // ADR-0041: approving a write also locks the squad the CEO saw, in one transaction.
    store.transact(|tx| {
        let current: Approval = storage::tx_get(tx, "approval", &id)?.ok_or("Approval not found")?;
        if current.status != "pending" { return Err("Approval already decided".into()); }
        let mut decided = current.clone();
        decided.status = if approved { "approved" } else { "denied" }.into();
        squad::lock_in_tx(tx, &current, approved, squad_digest.as_deref())?;
        storage::tx_put(tx, "approval", &id, &decided)?;
        storage::tx_event(tx, "approval_decided", decided.goal_id.as_deref().unwrap_or(""), &format!("{}: {}", decided.operation, decided.status))
    })?;
    let _ = app.emit("state-changed", ());
    Ok(())
}

/// ADR-0046 `개선`: the CEO's note sends the pending plan back to the PM on the same goal. Not an approval.
#[tauri::command]
fn revise_plan(goal_id: String, approval_id: String, note: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<GoalTurn, String> {
    let turn = plan_review::request_revision(&store, &goal_id, &approval_id, &note)?;
    let _ = app.emit("state-changed", ());
    Ok(turn)
}

/// ADR-0046 `거부`: deny the pending plan and close the goal as cancelled. No project file changes.
#[tauri::command]
fn cancel_goal(goal_id: String, approval_id: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<(), String> {
    plan_review::cancel(&store, &goal_id, &approval_id)?;
    let _ = app.emit("state-changed", ());
    Ok(())
}

#[tauri::command]
fn swap_squad_member(goal_id: String, from_agent_id: String, to_agent_id: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<model::Squad, String> {
    let squad = squad::swap(&store, &goal_id, &from_agent_id, &to_agent_id)?;
    let _ = app.emit("state-changed", ());
    Ok(squad)
}

#[tauri::command]
fn add_squad_member(goal_id: String, slot: String, agent_id: String, task_id: Option<String>, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<model::Squad, String> {
    let squad = squad::add(&store, &goal_id, &slot, &agent_id, task_id.as_deref())?;
    let _ = app.emit("state-changed", ());
    Ok(squad)
}

fn worktree_root(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(app.path().app_data_dir().map_err(|e| e.to_string())?.join("worktrees"))
}

#[tauri::command]
fn archive_goal(goal_id: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<(), String> {
    archive::archive(&store, &goal_id)?;
    let _ = app.emit("state-changed", ());
    Ok(())
}

#[tauri::command]
fn restore_goal(goal_id: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<(), String> {
    archive::restore(&store, &goal_id)?;
    let _ = app.emit("state-changed", ());
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupUsage { goal_id: String, bytes: u64 }

#[tauri::command]
fn backup_usage(store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<Vec<BackupUsage>, String> {
    Ok(archive::usage(&store, &worktree_root(&app)?)?.into_iter().map(|(goal_id, bytes)| BackupUsage { goal_id, bytes }).collect())
}

#[tauri::command]
fn purge_goal(goal_id: String, confirmed: bool, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<usize, String> {
    // ADR-0049: read what the reference cleanup needs before the goal's records are deleted.
    let goal: Option<Goal> = store.get("goal", &goal_id)?;
    let heir = references::heir_of(&store, &goal_id)?;
    let removed = archive::purge(&store, &goal_id, confirmed, &worktree_root(&app)?)?;
    if let Some(goal) = goal {
        if let Some(project) = goal.project_id.as_deref().and_then(|id| store.get::<Project>("project_entry", id).ok().flatten()) {
            references::purge_goal(&store, &project, &goal, heir).map_err(|error| format!("목표는 삭제했지만 참조 자료를 정리하지 못했습니다: {error}"))?;
        }
    }
    let _ = app.emit("state-changed", ());
    Ok(removed)
}

#[tauri::command]
fn preview_external_staff(file_name: String, text: String) -> Result<squad::CharacterPreview, String> { squad::parse_character(&file_name, &text) }

#[tauri::command]
fn import_external_staff(file_name: String, text: String, connection_id: String, model_id: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<Agent, String> {
    let agent = squad::import(&store, &file_name, &text, &connection_id, &model_id)?;
    let _ = app.emit("state-changed", ());
    Ok(agent)
}

#[tauri::command]
fn export_character(agent_id: String, store: tauri::State<'_, Store>) -> Result<String, String> {
    let agent: Agent = store.get("agent", &agent_id)?.ok_or("팀원을 찾을 수 없습니다")?;
    squad::export(&agent)
}

#[tauri::command]
fn hire_external_staff(agent_id: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<Agent, String> {
    let agent = squad::hire(&store, &agent_id)?;
    let _ = app.emit("state-changed", ());
    Ok(agent)
}

#[tauri::command]
fn end_external_contract(agent_id: String, store: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<Agent, String> {
    let agent = squad::end_contract(&store, &agent_id)?;
    let _ = app.emit("state-changed", ());
    Ok(agent)
}

#[tauri::command]
fn run_goal(goal_id: String, app: tauri::AppHandle) -> Result<(), String> { engine::start(app, goal_id) }

#[tauri::command]
fn resume_goal(goal_id: String, app: tauri::AppHandle) -> Result<(), String> { engine::resume_approved(app, goal_id) }

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let data_dir = match std::env::var_os("UTOPIA_DATA_DIR") {
                Some(path) => std::path::PathBuf::from(path),
                None => app.path().app_data_dir()?,
            };
            eprintln!("AI Organization OS data: {}", data_dir.display());
            std::fs::create_dir_all(&data_dir).map_err(|e| format!("Create data directory: {e}"))?;
            let store = Store::open(&data_dir.join("organization.sqlite")).map_err(|e| format!("Open data store: {e}"))?;
            if meeting::fixture_enabled() {
                // Debug-only offline QA responder; labelled as not AI and never used by execution goals.
                let fixture = meeting::fixture_connection();
                if store.get::<AiConnection>("connection", &fixture.id).map_err(|e| e.to_string())?.is_none() { store.put("connection", &fixture.id, &fixture, None).map_err(|e| e.to_string())?; }
                eprintln!("Meeting offline fixture connection enabled (UTOPIA_MEETING_FIXTURE=1, debug build)");
            }
            app.manage(store);
            engine::resume_auto_guidance(app.handle());
            // ADR-0049: unsent attachments older than a week go; indexes are rebuilt from the records.
            if let Ok(root) = curation::staging_root(app.handle()) {
                let store = app.state::<Store>();
                let _ = references::cleanup_stale(&store, &root);
                if let Ok(projects) = store.list::<Project>("project_entry") { for project in projects { let _ = references::write_indexes(&store, &project); } }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_backend_info, get_snapshot, discover_ai, check_connection, start_connection_login, list_connection_models, get_role_presets, preview_spec_import, apply_spec_import, get_portable_spec_status, preview_portable_spec, create_portable_spec_from_draft, create_portable_spec, get_sdd_origin_status, plan_sdd_origin, apply_sdd_origin, decide_goal_origin, set_sdd_origin_mute, pick_project_folder, set_project, switch_project, set_project_review_limit, set_goal_review_limit, set_goal_work_group, create_project, save_agent, assign_team_connection, save_connection, add_mock_connection, get_goal_result, get_block_stats, open_project_folder, submit_goal, meeting_readiness, start_meeting_from_home, update_meeting_plan, approve_meeting_plan, reject_meeting_plan, revoke_meeting_plan, decide_transmission, dispatch_meeting, stop_meeting, resume_meeting, retry_meeting_packet, continue_meeting_source_snapshot, close_meeting_partial, record_meeting_decision, ask_goal, send_goal_message, request_team_fix, stage_text_reference, stage_image_reference, discard_reference, set_reference_caption, set_reference_role, set_curator_enabled, propose_goal_change, create_goal_revision, preview_git_bootstrap, bootstrap_project_git, save_task, decide_approval, revise_plan, cancel_goal, swap_squad_member, add_squad_member, archive_goal, restore_goal, backup_usage, purge_goal, preview_external_staff, import_external_staff, export_character, hire_external_staff, end_external_contract, run_goal, resume_goal])
        .run(tauri::generate_context!())
        .expect("error while running AI Organization OS");
}
