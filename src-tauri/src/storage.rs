use crate::model::{Agent, Approval, Event, Goal, GoalGuidance, GoalTurn, Meeting, MeetingPacket, Project, Snapshot, Task, TokenUsageRecord, TransmissionApproval};
use crate::spec_import::{SpecDraft, SpecProfile};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;
use std::sync::Mutex;

pub struct Store(pub Mutex<Connection>);

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecAgentBinding { pub key: String, pub agent_id: Option<String> }
const SCHEMA_VERSION: i64 = 4;

impl Store {
    pub fn decide_transmission_record(&self, approval_id: &str, revision: u32, digest: &str, approved: bool, eligibility: &str) -> Result<(), String> {
        let mut db = self.0.lock().map_err(|e| e.to_string())?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        let payload: String = tx.query_row("SELECT payload FROM objects WHERE kind='transmission_approval' AND id=?1", [approval_id], |row| row.get(0)).optional().map_err(|e| e.to_string())?.ok_or("Transmission approval not found")?;
        let mut approval: TransmissionApproval = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
        if approval.status != "pending" || approval.preview_revision != revision || approval.payload_sha256 != digest { return Err("전송 미리보기가 변경되었습니다. 다시 확인해 주세요".into()); }
        let payload: String = tx.query_row("SELECT payload FROM objects WHERE kind='meeting_packet' AND id=?1", [&approval.packet_id], |row| row.get(0)).optional().map_err(|e| e.to_string())?.ok_or("Meeting packet not found")?;
        let mut packet: MeetingPacket = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
        if packet.status != "pending" || packet.meeting_id != approval.meeting_id || packet.preview_revision != revision || packet.payload_sha256 != digest || crate::meeting::digest(&packet.payload) != digest { return Err("전송 원문이 미리보기와 다릅니다".into()); }
        let payload: String = tx.query_row("SELECT payload FROM objects WHERE kind='meeting' AND id=?1", [&approval.meeting_id], |row| row.get(0)).optional().map_err(|e| e.to_string())?.ok_or("Meeting not found")?;
        let mut meeting: Meeting = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
        if meeting.stage != "awaiting_transfer_approval" { return Err("회의가 전송 승인을 기다리고 있지 않습니다".into()); }
        if packet.stage != meeting.active_stage || packet.template_version != crate::meeting::PACKET_TEMPLATE_VERSION { return Err("이전 버전 또는 다른 단계의 패킷은 승인할 수 없습니다. 회의를 다시 준비해 주세요".into()); }
        approval.status = if approved { "approved" } else { "denied" }.into();
        approval.decided_at = Some(crate::model::now());
        approval.dispatch_contract = crate::meeting::DISPATCH_CONTRACT;
        approval.mode = "exact".into();
        approval.eligibility = if approved { eligibility.into() } else { String::new() };
        packet.status = approval.status.clone();
        if !approved { meeting.stage = "blocked".into(); meeting.block_reason = "CEO가 전송 패킷을 거절했습니다. 거절한 내용은 전송되지 않았습니다. 새 미리보기로 재개하거나 부분 결과로 마칠 수 있습니다".into(); meeting.updated_at = crate::model::now(); }
        tx.execute("UPDATE objects SET payload=?1 WHERE kind='transmission_approval' AND id=?2", params![serde_json::to_string(&approval).map_err(|e| e.to_string())?, approval.id]).map_err(|e| e.to_string())?;
        tx.execute("UPDATE objects SET payload=?1 WHERE kind='meeting_packet' AND id=?2", params![serde_json::to_string(&packet).map_err(|e| e.to_string())?, packet.id]).map_err(|e| e.to_string())?;
        if !approved { tx.execute("UPDATE objects SET payload=?1 WHERE kind='meeting' AND id=?2", params![serde_json::to_string(&meeting).map_err(|e| e.to_string())?, meeting.id]).map_err(|e| e.to_string())?; }
        tx.execute("INSERT INTO events(at,kind,goal_id,task_id,agent_id,message) VALUES(?1,'transmission_decided',?2,NULL,NULL,?3)", params![crate::model::now(), meeting.goal_id, format!("packet {}: {}", packet.id, approval.status)]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }
    /// Runs `f` inside one SQLite transaction under the store lock; meeting state transitions use this
    /// so approval checks, dispatch reservation, stop and revocation are totally ordered.
    pub fn transact<R>(&self, f: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<R, String>) -> Result<R, String> {
        let mut db = self.0.lock().map_err(|e| e.to_string())?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        let result = f(&tx)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(result)
    }

    pub fn open(path: &Path) -> Result<Self, String> {
        let mut connection = Connection::open(path).map_err(|e| e.to_string())?;
        migrate(&mut connection)?;
        let store = Self(Mutex::new(connection));
        store.seed_agents()?;
        store.fill_blank_agent_instructions()?;
        store.repair_saved_cli_paths()?;
        store.fill_missing_goal_limits()?;
        store.recover_interrupted()?;
        Ok(store)
    }

    pub fn put<T: Serialize>(&self, kind: &str, id: &str, value: &T, event: Option<NewEvent>) -> Result<(), String> {
        let payload = serde_json::to_string(value).map_err(|e| e.to_string())?;
        let mut db = self.0.lock().map_err(|e| e.to_string())?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO objects(kind,id,payload) VALUES(?1,?2,?3) ON CONFLICT(kind,id) DO UPDATE SET payload=excluded.payload", params![kind, id, payload]).map_err(|e| e.to_string())?;
        if let Some(e) = event {
            tx.execute("INSERT INTO events(at,kind,goal_id,task_id,agent_id,message) VALUES(?1,?2,?3,?4,?5,?6)", params![crate::model::now(), e.kind, e.goal_id, e.task_id, e.agent_id, e.message]).map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn enqueue_goal_turn(&self, turn: &GoalTurn) -> Result<(), String> {
        self.enqueue_conversation_turn(turn, "goal_question", "CEO asked the goal team a question")
    }

    pub fn enqueue_goal_message(&self, turn: &GoalTurn) -> Result<(), String> {
        self.enqueue_conversation_turn(turn, "goal_message", "CEO sent a goal conversation message")
    }

    fn enqueue_conversation_turn(&self, turn: &GoalTurn, event_kind: &str, event_message: &str) -> Result<(), String> {
        let mut db = self.0.lock().map_err(|e| e.to_string())?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        let goal_payload: String = tx.query_row("SELECT payload FROM objects WHERE kind='goal' AND id=?1", [&turn.goal_id], |row| row.get(0)).optional().map_err(|e| e.to_string())?.ok_or("Goal not found")?;
        let goal: Goal = serde_json::from_str(&goal_payload).map_err(|e| e.to_string())?;
        if goal.kind != "execution" { return Err("회의 목표는 기존 팀 대화를 사용할 수 없습니다".into()); }
        let existing: Vec<GoalTurn> = {
            let mut stmt = tx.prepare("SELECT payload FROM objects WHERE kind='goal_turn'").map_err(|e| e.to_string())?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
            rows.map(|r| serde_json::from_str(&r.map_err(|e| e.to_string())?).map_err(|e| e.to_string())).collect::<Result<_, _>>()?
        };
        if existing.iter().any(|item| item.goal_id == turn.goal_id && item.status == "pending") {
            return Err("이 목표에 대한 답변을 기다리고 있습니다".into());
        }
        tx.execute("INSERT INTO objects(kind,id,payload) VALUES('goal_turn',?1,?2)", params![turn.id, serde_json::to_string(turn).map_err(|e| e.to_string())?]).map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO events(at,kind,goal_id,task_id,agent_id,message) VALUES(?1,?2,?3,NULL,?4,?5)", params![crate::model::now(), event_kind, turn.goal_id, turn.agent_id, event_message]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn propose_goal_guidance(&self, goal_id: &str, text: &str, addressed_agent_id: Option<&str>, auto_continue: bool) -> Result<GoalGuidance, String> {
        let mut db = self.0.lock().map_err(|e| e.to_string())?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        let payload: String = tx.query_row("SELECT payload FROM objects WHERE kind='goal' AND id=?1", [goal_id], |r| r.get(0)).optional().map_err(|e| e.to_string())?.ok_or("목표를 찾을 수 없습니다")?;
        let mut goal: Goal = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
        if goal.kind != "execution" { return Err("회의 목표는 실행 방향 변경을 사용할 수 없습니다".into()); }
        if goal.project_id.is_none() { return Err("프로젝트가 연결되지 않은 목표는 수정할 수 없습니다".into()); }
        if goal.status == "completed" { return Err("완료된 목표는 새 목표로 후속 작업을 만들어 주세요".into()); }
        if let Some(agent_id) = addressed_agent_id {
            let payload: Option<String> = tx.query_row("SELECT payload FROM objects WHERE kind='agent' AND id=?1", [agent_id], |r| r.get(0)).optional().map_err(|e| e.to_string())?;
            let agent: Agent = serde_json::from_str(&payload.ok_or("지침을 받을 팀원을 찾지 못했습니다")?).map_err(|e| e.to_string())?;
            if !agent.enabled { return Err("비활성 팀원에게는 지침을 전달할 수 없습니다".into()); }
        }
        let existing: Vec<GoalGuidance> = {
            let mut stmt = tx.prepare("SELECT payload FROM objects WHERE kind='goal_guidance'").map_err(|e| e.to_string())?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
            rows.map(|r| serde_json::from_str(&r.map_err(|e| e.to_string())?).map_err(|e| e.to_string())).collect::<Result<_, _>>()?
        };
        if existing.iter().any(|item| item.goal_id == goal_id && ["pending", "review_required"].contains(&item.status.as_str())) { return Err("이 목표에는 검토 중인 방향 변경 제안이 이미 있습니다".into()); }
        if existing.iter().any(|item| item.goal_id == goal_id && item.status == "revised" && item.replacement_goal_id.is_some()) { return Err("이 목표는 새 버전으로 이어졌습니다. 새 목표를 열어 변경 지침을 전달해 주세요".into()); }
        let waiting_for_engine = !["new", "blocked"].contains(&goal.status.as_str());
        let guidance = GoalGuidance { id: crate::model::id(), goal_id: goal_id.into(), text: text.into(), status: if waiting_for_engine { "pending" } else { "review_required" }.into(), created_at: crate::model::now(), checkpoint: if waiting_for_engine { None } else { Some("실행 대기 상태".into()) }, replacement_goal_id: None, addressed_agent_id: addressed_agent_id.map(str::to_string), worker_handoff: None, continuation: None, auto_continue };
        tx.execute("INSERT INTO objects(kind,id,payload) VALUES('goal_guidance',?1,?2)", params![guidance.id, serde_json::to_string(&guidance).map_err(|e| e.to_string())?]).map_err(|e| e.to_string())?;
        if goal.status == "new" {
            goal.resumed_from = Some("new".into());
            goal.status = "blocked".into();
            goal.report = "CEO의 방향 변경 제안을 검토하고 새 목표 버전을 만드세요.".into();
            tx.execute("UPDATE objects SET payload=?1 WHERE kind='goal' AND id=?2", params![serde_json::to_string(&goal).map_err(|e| e.to_string())?, goal.id]).map_err(|e| e.to_string())?;
        }
        let pending_approvals: Vec<Approval> = {
            let mut stmt = tx.prepare("SELECT payload FROM objects WHERE kind='approval'").map_err(|e| e.to_string())?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
            rows.map(|r| serde_json::from_str(&r.map_err(|e| e.to_string())?).map_err(|e| e.to_string())).collect::<Result<_, _>>()?
        };
        for mut approval in pending_approvals.into_iter().filter(|item| item.goal_id.as_deref() == Some(goal_id) && item.status == "pending") {
            approval.status = "superseded".into();
            tx.execute("UPDATE objects SET payload=?1 WHERE kind='approval' AND id=?2", params![serde_json::to_string(&approval).map_err(|e| e.to_string())?, approval.id]).map_err(|e| e.to_string())?;
        }
        tx.execute("INSERT INTO events(at,kind,goal_id,task_id,agent_id,message) VALUES(?1,'goal_guidance_proposed',?2,NULL,?3,'CEO proposed a goal direction change; teammate routing is queued until revision analysis')", params![crate::model::now(), goal_id, addressed_agent_id]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(guidance)
    }

    pub fn complete_goal_if_clear(&self, goal: &Goal) -> Result<(), String> {
        let mut db = self.0.lock().map_err(|e| e.to_string())?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        let payload: String = tx.query_row("SELECT payload FROM objects WHERE kind='goal' AND id=?1", [&goal.id], |r| r.get(0)).optional().map_err(|e| e.to_string())?.ok_or("Goal not found")?;
        let current: Goal = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
        if current.status != "validating" { return Err("Goal stage changed before completion".into()); }
        let guidance: Vec<GoalGuidance> = {
            let mut stmt = tx.prepare("SELECT payload FROM objects WHERE kind='goal_guidance'").map_err(|e| e.to_string())?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
            rows.map(|r| serde_json::from_str(&r.map_err(|e| e.to_string())?).map_err(|e| e.to_string())).collect::<Result<_, _>>()?
        };
        if guidance.iter().any(|item| item.goal_id == goal.id && ["pending", "review_required"].contains(&item.status.as_str())) { return Err("CEO direction change reached the final checkpoint".into()); }
        tx.execute("UPDATE objects SET payload=?1 WHERE kind='goal' AND id=?2", params![serde_json::to_string(goal).map_err(|e| e.to_string())?, goal.id]).map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO events(at,kind,goal_id,task_id,agent_id,message) VALUES(?1,'goal_completed',?2,NULL,NULL,'CEO report ready')", params![crate::model::now(), goal.id]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn create_goal_revision(&self, guidance_id: &str, intent: &str) -> Result<Goal, String> {
        let mut db = self.0.lock().map_err(|e| e.to_string())?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        let payload: String = tx.query_row("SELECT payload FROM objects WHERE kind='goal_guidance' AND id=?1", [guidance_id], |r| r.get(0)).optional().map_err(|e| e.to_string())?.ok_or("방향 변경 제안을 찾을 수 없습니다")?;
        let mut guidance: GoalGuidance = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
        if guidance.status != "review_required" { return Err("기존 목표가 안전 지점에서 멈춘 뒤 새 목표를 만들 수 있습니다".into()); }
        let payload: String = tx.query_row("SELECT payload FROM objects WHERE kind='goal' AND id=?1", [&guidance.goal_id], |r| r.get(0)).optional().map_err(|e| e.to_string())?.ok_or("기존 목표를 찾을 수 없습니다")?;
        let old: Goal = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
        if old.kind != "execution" { return Err("회의 목표는 실행 목표 버전을 만들 수 없습니다".into()); }
        if old.status != "blocked" { return Err("기존 목표가 중단되었는지 먼저 확인해 주세요".into()); }
        let tasks: Vec<Task> = {
            let mut stmt = tx.prepare("SELECT payload FROM objects WHERE kind='task'").map_err(|e| e.to_string())?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
            rows.map(|r| serde_json::from_str(&r.map_err(|e| e.to_string())?).map_err(|e| e.to_string())).collect::<Result<_, _>>()?
        };
        if tasks.iter().any(|task| task.goal_id == old.id && task.status == "running") { return Err("아직 실행 중인 업무가 있습니다. 종료 후 다시 시도해 주세요".into()); }
        let project_id = old.project_id.clone().ok_or("기존 프로젝트를 확인할 수 없습니다")?;
        let registered: Option<String> = tx.query_row("SELECT payload FROM objects WHERE kind='project_entry' AND id=?1", [&project_id], |r| r.get(0)).optional().map_err(|e| e.to_string())?;
        if registered.is_none() { return Err("기존 프로젝트가 등록되어 있지 않습니다".into()); }
        let revised = Goal { id: crate::model::id(), kind: "execution".into(), project_id: Some(project_id), intent: intent.into(), status: "new".into(), report: String::new(), created_at: crate::model::now(), resumed_from: None, review_loops_used: 0, review_loop_limit: old.review_loop_limit, work_group: old.work_group.clone(), block_kind: String::new() };
        tx.execute("INSERT INTO objects(kind,id,payload) VALUES('goal',?1,?2)", params![revised.id, serde_json::to_string(&revised).map_err(|e| e.to_string())?]).map_err(|e| e.to_string())?;
        guidance.status = "revised".into();
        guidance.replacement_goal_id = Some(revised.id.clone());
        tx.execute("UPDATE objects SET payload=?1 WHERE kind='goal_guidance' AND id=?2", params![serde_json::to_string(&guidance).map_err(|e| e.to_string())?, guidance.id]).map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO events(at,kind,goal_id,task_id,agent_id,message) VALUES(?1,'goal_revision_created',?2,NULL,NULL,?3)", params![crate::model::now(), revised.id, format!("Follow-up revision of {}", old.id)]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(revised)
    }

    pub fn append_event(&self, event: NewEvent) -> Result<(), String> {
        let db = self.0.lock().map_err(|e| e.to_string())?;
        db.execute("INSERT INTO events(at,kind,goal_id,task_id,agent_id,message) VALUES(?1,?2,?3,?4,?5,?6)", params![crate::model::now(), event.kind, event.goal_id, event.task_id, event.agent_id, event.message]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get<T: DeserializeOwned>(&self, kind: &str, id: &str) -> Result<Option<T>, String> {
        let db = self.0.lock().map_err(|e| e.to_string())?;
        let payload: Option<String> = db.query_row("SELECT payload FROM objects WHERE kind=?1 AND id=?2", params![kind,id], |r| r.get(0)).optional().map_err(|e| e.to_string())?;
        payload.map(|s| serde_json::from_str(&s).map_err(|e| e.to_string())).transpose()
    }

    pub fn list<T: DeserializeOwned>(&self, kind: &str) -> Result<Vec<T>, String> {
        let db = self.0.lock().map_err(|e| e.to_string())?;
        let mut stmt = db.prepare("SELECT payload FROM objects WHERE kind=?1 ORDER BY id").map_err(|e| e.to_string())?;
        let rows = stmt.query_map([kind], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
        rows.map(|r| serde_json::from_str(&r.map_err(|e| e.to_string())?).map_err(|e| e.to_string())).collect()
    }

    pub fn snapshot(&self) -> Result<Snapshot, String> {
        let project = self.get("project", "selected")?;
        let projects = self.list("project_entry")?;
        let agents = self.list("agent")?;
        let connections = self.list("connection")?;
        let goals = self.list("goal")?;
        let meetings = self.list::<Meeting>("meeting")?;
        let meeting_packets = self.list::<MeetingPacket>("meeting_packet")?;
        let transmission_approvals = self.list::<TransmissionApproval>("transmission_approval")?;
        let meeting_contributions = self.list::<crate::model::MeetingContribution>("meeting_contribution")?;
        let goal_lineage = self.list::<crate::model::GoalLineage>("goal_lineage")?;
        let goal_turns = self.list("goal_turn")?;
        let goal_guidance = self.list("goal_guidance")?;
        let tasks = self.list("task")?;
        let deliverable_reviews = self.list::<crate::model::DeliverableReview>("deliverable_review")?;
        let mut squads = self.list::<crate::model::Squad>("squad")?;
        for squad in squads.iter_mut() { squad.digest = crate::squad::digest(squad); }
        let goal_summaries = self.list::<crate::model::GoalSummary>("goal_summary")?;
        let mut goal_origins = self.list::<crate::model::GoalOrigin>("goal_origin")?;
        let approvals = self.list("approval")?;
        let usage: Vec<TokenUsageRecord> = self.list("usage")?;
        let spec_profiles: Vec<SpecProfile> = self.list::<SpecProfile>("spec_profile")?.into_iter().filter(|item| project.as_ref().is_some_and(|selected: &Project| selected.id == item.project_id)).collect();
        let spec_drafts: Vec<SpecDraft> = self.list::<SpecDraft>("spec_draft")?.into_iter().filter(|item| project.as_ref().is_some_and(|selected: &Project| selected.id == item.project_id)).collect();
        let db = self.0.lock().map_err(|e| e.to_string())?;
        let mut stmt = db.prepare("SELECT id,at,kind,goal_id,task_id,agent_id,message FROM events ORDER BY id DESC LIMIT 250").map_err(|e| e.to_string())?;
        let mut events = stmt.query_map([], |r| Ok(Event { id: r.get(0)?, at: r.get(1)?, kind: r.get(2)?, goal_id: r.get(3)?, task_id: r.get(4)?, agent_id: r.get(5)?, message: r.get(6)? })).map_err(|e| e.to_string())?.map(|r| r.map_err(|e| e.to_string())).collect::<Result<Vec<_>,_>>()?;
        drop(stmt);
        drop(db);
        // ADR-0043: archived goals and everything that belongs to them are hidden from every screen;
        // the backup dialog sees only their summaries. Token usage stays so totals remain truthful.
        let archives = self.list::<crate::archive::GoalArchive>("goal_archive")?;
        let hidden: std::collections::BTreeSet<String> = archives.iter().map(|item| item.goal_id.clone()).collect();
        let archived_goals = crate::archive::summaries(&goals, &archives, &meetings, &goal_summaries);
        let mut goals = goals;
        let mut meetings = meetings;
        let (mut meeting_packets, mut transmission_approvals, mut meeting_contributions, mut goal_lineage, mut goal_turns, mut goal_guidance, mut tasks, mut deliverable_reviews, mut goal_summaries, mut approvals) = (meeting_packets, transmission_approvals, meeting_contributions, goal_lineage, goal_turns, goal_guidance, tasks, deliverable_reviews, goal_summaries, approvals);
        if !hidden.is_empty() {
            let hidden_meetings: std::collections::BTreeSet<String> = meetings.iter().filter(|item| hidden.contains(&item.goal_id)).map(|item| item.id.clone()).collect();
            goals.retain(|item: &Goal| !hidden.contains(&item.id));
            meetings.retain(|item| !hidden.contains(&item.goal_id));
            meeting_packets.retain(|item| !hidden_meetings.contains(&item.meeting_id));
            transmission_approvals.retain(|item| !hidden_meetings.contains(&item.meeting_id));
            meeting_contributions.retain(|item| !hidden_meetings.contains(&item.meeting_id));
            goal_lineage.retain(|item| !hidden.contains(&item.goal_id) && !hidden_meetings.contains(&item.meeting_id));
            goal_turns.retain(|item: &GoalTurn| !hidden.contains(&item.goal_id));
            goal_guidance.retain(|item: &GoalGuidance| !hidden.contains(&item.goal_id) && !item.replacement_goal_id.as_ref().is_some_and(|id| hidden.contains(id)));
            tasks.retain(|item: &Task| !hidden.contains(&item.goal_id));
            deliverable_reviews.retain(|item| !hidden.contains(&item.goal_id));
            squads.retain(|item| !hidden.contains(&item.goal_id));
            goal_summaries.retain(|item| !hidden.contains(&item.goal_id));
            goal_origins.retain(|item| !hidden.contains(&item.goal_id));
            approvals.retain(|item: &Approval| !item.goal_id.as_ref().is_some_and(|id| hidden.contains(id)));
            events.retain(|item| !item.goal_id.as_ref().is_some_and(|id| hidden.contains(id)));
        }
        Ok(Snapshot { project, projects, agents, connections, goals, meetings, meeting_packets, transmission_approvals, meeting_contributions, goal_lineage, goal_turns, goal_guidance, tasks, deliverable_reviews, squads, goal_summaries, goal_origins, archived_goals, events, approvals, usage, spec_profiles, spec_drafts })
    }

    pub fn stage_spec_import(&self, draft: &SpecDraft) -> Result<(), String> {
        let mut db = self.0.lock().map_err(|e| e.to_string())?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        let previous: Vec<SpecDraft> = {
            let mut stmt = tx.prepare("SELECT payload FROM objects WHERE kind='spec_draft'").map_err(|e| e.to_string())?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0)).map_err(|e| e.to_string())?;
            rows.map(|row| serde_json::from_str(&row.map_err(|e| e.to_string())?).map_err(|e| e.to_string())).collect::<Result<_, _>>()?
        };
        for item in previous.into_iter().filter(|item| item.project_id == draft.project_id) {
            tx.execute("DELETE FROM objects WHERE kind='spec_draft' AND id=?1", [item.id]).map_err(|e| e.to_string())?;
        }
        tx.execute("INSERT INTO objects(kind,id,payload) VALUES('spec_draft',?1,?2)", params![draft.id, serde_json::to_string(draft).map_err(|e| e.to_string())?]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn apply_spec_import(&self, draft_id: &str, bindings: &[SpecAgentBinding]) -> Result<SpecProfile, String> {
        let mut db = self.0.lock().map_err(|e| e.to_string())?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        let payload: String = tx.query_row("SELECT payload FROM objects WHERE kind='spec_draft' AND id=?1", [draft_id], |row| row.get(0)).optional().map_err(|e| e.to_string())?.ok_or("명세 초안을 찾지 못했습니다. 다시 미리보기를 만들어 주세요")?;
        let draft: SpecDraft = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
        let project_payload: String = tx.query_row("SELECT payload FROM objects WHERE kind='project_entry' AND id=?1", [&draft.project_id], |row| row.get(0)).optional().map_err(|e| e.to_string())?.ok_or("프로젝트가 더 이상 등록되어 있지 않습니다")?;
        let project: Project = serde_json::from_str(&project_payload).map_err(|e| e.to_string())?;
        crate::spec_import::verify_portable_draft(&project, &draft)?;
        let current: Option<String> = tx.query_row("SELECT payload FROM objects WHERE kind='spec_profile' AND id=?1", [&draft.project_id], |row| row.get(0)).optional().map_err(|e| e.to_string())?;
        let revision = current.as_deref().map(|value| serde_json::from_str::<SpecProfile>(value).map(|item| item.revision).map_err(|e| e.to_string())).transpose()?.unwrap_or(0);
        if revision != draft.base_revision { return Err("명세가 다른 창에서 변경되었습니다. 다시 미리보기를 확인해 주세요".into()); }
        let agents: Vec<Agent> = {
            let mut stmt = tx.prepare("SELECT payload FROM objects WHERE kind='agent'").map_err(|e| e.to_string())?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0)).map_err(|e| e.to_string())?;
            rows.map(|row| serde_json::from_str(&row.map_err(|e| e.to_string())?).map_err(|e| e.to_string())).collect::<Result<_, _>>()?
        };
        let mut profile = draft.profile;
        let mut seen = std::collections::HashSet::new();
        for binding in bindings {
            if !seen.insert(binding.key.as_str()) { return Err("같은 역할 연결이 중복되었습니다".into()); }
            let spec = profile.agents.iter_mut().find(|item| item.key == binding.key).ok_or("초안에 없는 역할 키가 전달되었습니다")?;
            if let Some(id) = &binding.agent_id {
                let agent = agents.iter().find(|agent| &agent.id == id && agent.enabled).ok_or("연결할 활성 팀원을 찾지 못했습니다")?;
                spec.bound_agent_id = Some(agent.id.clone());
            }
        }
        crate::spec_import::validate_profile(&profile)?;
        profile.revision = revision.checked_add(1).ok_or("명세 버전 한도를 넘었습니다")?;
        profile.imported_at = crate::model::now();
        let payload = serde_json::to_string(&profile).map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO objects(kind,id,payload) VALUES('spec_profile',?1,?2) ON CONFLICT(kind,id) DO UPDATE SET payload=excluded.payload", params![profile.project_id, payload]).map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO objects(kind,id,payload) VALUES('spec_profile_version',?1,?2)", params![format!("{}:{}", profile.project_id, profile.revision), payload]).map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM objects WHERE kind='spec_draft' AND id=?1", [draft_id]).map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO events(at,kind,goal_id,task_id,agent_id,message) VALUES(?1,'spec_import_applied',NULL,NULL,NULL,?2)", params![crate::model::now(), format!("Project {} spec v{} applied ({} roles, {} workflow steps, {} exceptions)", profile.project_id, profile.revision, profile.agents.len(), profile.workflow.len(), profile.exceptions.len())]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(profile)
    }

    pub fn unfinished_task_count(&self, agent_id: &str) -> Result<usize, String> {
        Ok(self.list::<Task>("task")?.into_iter().filter(|task| task.assignee_id == agent_id && ["pending", "running", "awaiting_integration"].contains(&task.status.as_str())).count())
    }

    pub fn select_project(&self, project: &Project) -> Result<(), String> {
        let payload = serde_json::to_string(project).map_err(|e| e.to_string())?;
        let mut db = self.0.lock().map_err(|e| e.to_string())?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO objects(kind,id,payload) VALUES('project_entry',?1,?2) ON CONFLICT(kind,id) DO UPDATE SET payload=excluded.payload", params![project.id, payload]).map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO objects(kind,id,payload) VALUES('project','selected',?1) ON CONFLICT(kind,id) DO UPDATE SET payload=excluded.payload", [payload]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    fn fill_missing_goal_limits(&self) -> Result<(), String> {
        let projects: Vec<Project> = self.list("project_entry")?;
        for mut goal in self.list::<Goal>("goal")? {
            if goal.review_loop_limit.is_some() { continue; }
            let preset = goal.project_id.as_ref()
                .and_then(|id| projects.iter().find(|project| &project.id == id))
                .map(|project| project.review_loop_limit)
                .unwrap_or(crate::model::default_review_loop_limit());
            goal.review_loop_limit = Some(preset);
            self.put("goal", &goal.id, &goal, None)?;
        }
        Ok(())
    }

    pub fn register_connection_and_assign_unassigned(&self, connection: &crate::model::AiConnection) -> Result<usize, String> {
        let agents = self.list::<Agent>("agent")?;
        let payload = serde_json::to_string(connection).map_err(|e| e.to_string())?;
        let mut db = self.0.lock().map_err(|e| e.to_string())?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO objects(kind,id,payload) VALUES('connection',?1,?2) ON CONFLICT(kind,id) DO UPDATE SET payload=excluded.payload", params![connection.id, payload]).map_err(|e| e.to_string())?;
        let mut assigned = 0;
        for mut agent in agents {
            if !agent.connection_id.is_empty() { continue; }
            agent.connection_id = connection.id.clone();
            let payload = serde_json::to_string(&agent).map_err(|e| e.to_string())?;
            tx.execute("UPDATE objects SET payload=?1 WHERE kind='agent' AND id=?2", params![payload, agent.id]).map_err(|e| e.to_string())?;
            assigned += 1;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(assigned)
    }

    pub fn assign_connection_to_agents(&self, connection_id: &str, agent_ids: &[String]) -> Result<usize, String> {
        if agent_ids.is_empty() || agent_ids.len() > 100 { return Err("변경할 활성 팀원을 1~100명 선택해 주세요".into()); }
        let unique: std::collections::HashSet<&str> = agent_ids.iter().map(String::as_str).collect();
        if unique.len() != agent_ids.len() { return Err("팀원 선택에 중복이 있습니다".into()); }
        let mut db = self.0.lock().map_err(|e| e.to_string())?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        let payload: String = tx.query_row("SELECT payload FROM objects WHERE kind='connection' AND id=?1", [connection_id], |r| r.get(0)).optional().map_err(|e| e.to_string())?.ok_or("등록된 AI를 선택해 주세요")?;
        let connection: crate::model::AiConnection = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
        let mut agents = Vec::new();
        for id in agent_ids {
            let payload: String = tx.query_row("SELECT payload FROM objects WHERE kind='agent' AND id=?1", [id], |r| r.get(0)).optional().map_err(|e| e.to_string())?.ok_or_else(|| format!("팀원을 찾지 못했습니다: {id}"))?;
            let mut agent: Agent = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
            if !agent.enabled { return Err(format!("숨긴 팀원 {}은(는) 먼저 복원해 주세요", agent.name)); }
            if connection.kind == "http" && (agent.role.to_ascii_lowercase().contains("implement") || agent.role.contains("개발")) {
                return Err(format!("{}은(는) 파일 작업을 맡으므로 HTTP 텍스트 모델을 배정할 수 없습니다", agent.name));
            }
            agent.connection_id = connection_id.into();
            agent.model_id.clear();
            agents.push(agent);
        }
        for agent in &agents {
            tx.execute("UPDATE objects SET payload=?1 WHERE kind='agent' AND id=?2", params![serde_json::to_string(agent).map_err(|e| e.to_string())?, agent.id]).map_err(|e| e.to_string())?;
        }
        tx.execute("INSERT INTO events(at,kind,goal_id,task_id,agent_id,message) VALUES(?1,'team_connection_changed',NULL,NULL,NULL,?2)", params![crate::model::now(), format!("{} teammates assigned to {}", agents.len(), connection.name)]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(agents.len())
    }

    fn repair_saved_cli_paths(&self) -> Result<(), String> {
        for mut connection in self.list::<crate::model::AiConnection>("connection")? {
            if connection.kind != "cli" { continue; }
            if let Some(repaired) = crate::discovery::repaired_known_windows_cli_path(Path::new(&connection.executable), &connection.id) {
                connection.executable = repaired.to_string_lossy().into_owned();
                connection.status = "needs_auth_check".into();
                connection.capabilities = vec!["agent".into(), "files".into(), "tools".into()];
                self.put("connection", &connection.id.clone(), &connection, None)?;
            }
        }
        Ok(())
    }

    fn seed_agents(&self) -> Result<(), String> {
        let existing: Vec<Agent> = self.list("agent")?;
        if !existing.is_empty() { return Ok(()); }
        for (id, name, role, reports_to, x, y) in [
            ("cto", "CTO", "architecture and delegation", None, 14, 20),
            ("pm", "PM", "planning and task decomposition", Some("cto"), 38, 20),
            ("developer-a", "Developer A", "implementation", Some("pm"), 22, 55),
            ("developer-b", "Developer B", "implementation", Some("pm"), 52, 55),
            ("qa", "QA / Reviewer", "validation and independent review", Some("cto"), 76, 45),
        ] {
            let agent = Agent { id: id.into(), name: name.into(), role: role.into(), instructions: crate::presets::for_seed_id(id).instructions, reports_to: reports_to.map(str::to_string), connection_id: String::new(), model_id: String::new(), x, y, enabled: true, external: None, appearance: None };
            self.put("agent", id, &agent, None)?;
        }
        Ok(())
    }

    pub fn fill_blank_agent_instructions(&self) -> Result<(), String> {
        for mut agent in self.list::<Agent>("agent")? {
            if agent.instructions.trim().is_empty() {
                agent.instructions = crate::presets::for_seed_id(&agent.id).instructions;
                self.put("agent", &agent.id.clone(), &agent, None)?;
            }
        }
        Ok(())
    }

    fn recover_interrupted(&self) -> Result<(), String> {
        for mut meeting in self.list::<Meeting>("meeting")? {
            if ["proposing", "critiquing", "revising", "evaluating"].contains(&meeting.stage.as_str()) {
                meeting.stage = "blocked".into();
                meeting.block_reason = "앱이 회의 호출 중에 종료되었습니다. 진행 중이던 호출은 응답 여부를 알 수 없어 자동으로 다시 보내지 않습니다".into();
                meeting.updated_at = crate::model::now();
                self.put("meeting", &meeting.id.clone(), &meeting, None)?;
            }
        }
        for mut packet in self.list::<MeetingPacket>("meeting_packet")? {
            if packet.status == "dispatching" {
                packet.status = "outcome_unknown".into();
                if let Some(dispatch) = packet.dispatch.as_mut() { dispatch.outcome = "outcome_unknown".into(); dispatch.error = "앱이 종료되어 제공자 응답 여부를 확인할 수 없습니다".into(); }
                self.put("meeting_packet", &packet.id.clone(), &packet, None)?;
            }
        }
        crate::meeting::expire_on_restart(self)?;
        for mut guidance in self.list::<GoalGuidance>("goal_guidance")? {
            if guidance.status == "pending" {
                guidance.status = "review_required".into();
                guidance.checkpoint = Some("app_restart".into());
                self.put("goal_guidance", &guidance.id.clone(), &guidance, None)?;
            }
        }
        let waiting_goals: Vec<String> = self.list::<GoalGuidance>("goal_guidance")?.into_iter().filter(|item| item.status == "review_required").map(|item| item.goal_id).collect();
        for mut approval in self.list::<Approval>("approval")? {
            if approval.status == "pending" && approval.goal_id.as_ref().is_some_and(|id| waiting_goals.contains(id)) {
                approval.status = "superseded".into();
                self.put("approval", &approval.id.clone(), &approval, None)?;
            }
        }
        for mut turn in self.list::<GoalTurn>("goal_turn")? {
            if turn.status == "pending" {
                turn.status = "interrupted".into();
                turn.answer = "앱이 종료되어 응답 완료를 확인할 수 없습니다. 다시 질문해 주세요.".into();
                turn.answered_at = Some(crate::model::now());
                self.put("goal_turn", &turn.id.clone(), &turn, None)?;
            }
        }
        for mut usage in self.list::<TokenUsageRecord>("usage")? {
            if usage.status == "active" {
                usage.status = "interrupted".into();
                self.put("usage", &usage.id.clone(), &usage, None)?;
            }
        }
        for mut task in self.list::<Task>("task")? {
            if task.status == "running" {
                task.status = "blocked".into();
                task.result = "The app stopped while this task was running. Inspect its worktree and resume deliberately.".into();
                self.put("task", &task.id.clone(), &task, Some(NewEvent::task("task_interrupted", &task, &task.result)))?;
            }
        }
        for mut goal in self.list::<Goal>("goal")? {
            if ["analyzing", "planning", "awaiting_approval", "executing", "integrating", "validating"].contains(&goal.status.as_str()) {
                goal.resumed_from = Some(goal.status.clone());
                goal.status = "blocked".into();
                goal.block_kind = "interrupted".into();
                self.put("goal", &goal.id.clone(), &goal, Some(NewEvent::goal("goal_interrupted", &goal, "App restart requires recovery review")))?;
            }
        }
        Ok(())
    }
}

fn migrate(connection: &mut Connection) -> Result<(), String> {
    connection.execute_batch("PRAGMA journal_mode=WAL;").map_err(|e| e.to_string())?;
    let mut version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0)).map_err(|e| e.to_string())?;
    if version > SCHEMA_VERSION { return Err(format!("Database schema version {version} is newer than this app supports ({SCHEMA_VERSION})")); }
    if version == 0 {
        let tx = connection.transaction().map_err(|e| e.to_string())?;
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS objects (
               kind TEXT NOT NULL, id TEXT NOT NULL, payload TEXT NOT NULL,
               PRIMARY KEY(kind, id)
             );
             CREATE TABLE IF NOT EXISTS events (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               at TEXT NOT NULL, kind TEXT NOT NULL, goal_id TEXT, task_id TEXT,
               agent_id TEXT, message TEXT NOT NULL
             );
             PRAGMA user_version=1;"
        ).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        version = 1;
    }
    if version == 1 {
        let tx = connection.transaction().map_err(|e| e.to_string())?;
        let old: Option<String> = tx.query_row("SELECT payload FROM objects WHERE kind='project' AND id='selected'", [], |row| row.get(0)).optional().map_err(|e| e.to_string())?;
        if let Some(payload) = old {
            let mut project: Project = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
            if project.id.is_empty() { project.id = crate::model::id(); }
            let payload = serde_json::to_string(&project).map_err(|e| e.to_string())?;
            tx.execute("INSERT INTO objects(kind,id,payload) VALUES('project_entry',?1,?2) ON CONFLICT(kind,id) DO UPDATE SET payload=excluded.payload", params![project.id, payload]).map_err(|e| e.to_string())?;
            tx.execute("UPDATE objects SET payload=?1 WHERE kind='project' AND id='selected'", [payload]).map_err(|e| e.to_string())?;
        }
        tx.execute_batch("PRAGMA user_version=2;").map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        version = 2;
    }
    if version == 2 {
        let tx = connection.transaction().map_err(|e| e.to_string())?;
        {
            let mut stmt = tx.prepare("SELECT id,payload FROM objects WHERE kind='goal'").map_err(|e| e.to_string())?;
            let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).map_err(|e| e.to_string())?;
            let goals = rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
            for (id, payload) in goals {
                let mut value: serde_json::Value = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
                let object = value.as_object_mut().ok_or("Goal payload is not an object")?;
                object.insert("kind".into(), serde_json::Value::String("execution".into()));
                tx.execute("UPDATE objects SET payload=?1 WHERE kind='goal' AND id=?2", params![serde_json::to_string(&value).map_err(|e| e.to_string())?, id]).map_err(|e| e.to_string())?;
            }
        }
        tx.execute_batch("PRAGMA user_version=3;").map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        version = 3;
    }
    if version == 3 {
        // Version 4: Office-character meetings (ADR-0036). Earlier-model meetings are closed
        // read-only by `meeting::expire_on_restart`; the bump makes older binaries refuse this file.
        connection.execute_batch("PRAGMA user_version=4;").map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn tx_get<T: DeserializeOwned>(tx: &rusqlite::Transaction<'_>, kind: &str, id: &str) -> Result<Option<T>, String> {
    let payload: Option<String> = tx.query_row("SELECT payload FROM objects WHERE kind=?1 AND id=?2", params![kind, id], |r| r.get(0)).optional().map_err(|e| e.to_string())?;
    payload.map(|s| serde_json::from_str(&s).map_err(|e| e.to_string())).transpose()
}

pub fn tx_list<T: DeserializeOwned>(tx: &rusqlite::Transaction<'_>, kind: &str) -> Result<Vec<T>, String> {
    let mut stmt = tx.prepare("SELECT payload FROM objects WHERE kind=?1 ORDER BY id").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([kind], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
    rows.map(|r| serde_json::from_str(&r.map_err(|e| e.to_string())?).map_err(|e| e.to_string())).collect()
}

pub fn tx_put<T: Serialize>(tx: &rusqlite::Transaction<'_>, kind: &str, id: &str, value: &T) -> Result<(), String> {
    let payload = serde_json::to_string(value).map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO objects(kind,id,payload) VALUES(?1,?2,?3) ON CONFLICT(kind,id) DO UPDATE SET payload=excluded.payload", params![kind, id, payload]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn tx_event(tx: &rusqlite::Transaction<'_>, kind: &str, goal_id: &str, message: &str) -> Result<(), String> {
    tx.execute("INSERT INTO events(at,kind,goal_id,task_id,agent_id,message) VALUES(?1,?2,?3,NULL,NULL,?4)", params![crate::model::now(), kind, goal_id, message]).map_err(|e| e.to_string())?;
    Ok(())
}

pub struct NewEvent {
    pub kind: String,
    pub goal_id: Option<String>,
    pub task_id: Option<String>,
    pub agent_id: Option<String>,
    pub message: String,
}

impl NewEvent {
    pub fn goal(kind: &str, goal: &Goal, message: &str) -> Self {
        Self { kind: kind.into(), goal_id: Some(goal.id.clone()), task_id: None, agent_id: None, message: message.into() }
    }
    pub fn task(kind: &str, task: &Task, message: &str) -> Self {
        Self { kind: kind.into(), goal_id: Some(task.goal_id.clone()), task_id: Some(task.id.clone()), agent_id: Some(task.assignee_id.clone()), message: message.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn simultaneous_goal_questions_enqueue_only_one_pending_turn() {
        use std::sync::{Arc, Barrier};
        let dir = std::env::temp_dir().join(format!("utopia-turn-race-{}", crate::model::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Arc::new(Store::open(&dir.join("state.sqlite")).unwrap());
        let goal = Goal { id: "goal".into(), kind: "execution".into(), project_id: Some("p".into()), intent: "Ask the team".into(), status: "new".into(), report: String::new(), created_at: crate::model::now(), resumed_from: None, review_loops_used: 0, review_loop_limit: Some(2), work_group: None, block_kind: String::new() };
        store.put("goal", &goal.id, &goal, None).unwrap();
        let mut meeting_goal = goal.clone();
        meeting_goal.id = "meeting-goal".into();
        meeting_goal.kind = "deliberation".into();
        store.put("goal", &meeting_goal.id, &meeting_goal, None).unwrap();
        let mut blocked_turn = GoalTurn { id: "turn-meeting".into(), goal_id: "meeting-goal".into(), question: "What is next?".into(), answer: String::new(), status: "pending".into(), asked_at: crate::model::now(), answered_at: None, agent_id: "cto".into(), agent_name: "CTO".into(), connection_name: "Local".into(), model_id: String::new(), guidance_id: None, route: None };
        assert!(store.enqueue_goal_turn(&blocked_turn).is_err(), "meeting goals reject execution conversation turns");
        blocked_turn.goal_id = "missing".into();
        assert!(store.enqueue_goal_turn(&blocked_turn).is_err());
        let barrier = Arc::new(Barrier::new(2));
        let handles: Vec<_> = (0..2).map(|index| {
            let store = Arc::clone(&store);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let turn = GoalTurn { id: format!("turn-{index}"), goal_id: "goal".into(), question: "What is next?".into(), answer: String::new(), status: "pending".into(), asked_at: crate::model::now(), answered_at: None, agent_id: "cto".into(), agent_name: "CTO".into(), connection_name: "Local".into(), model_id: String::new(), guidance_id: None, route: None };
                barrier.wait();
                store.enqueue_goal_turn(&turn)
            })
        }).collect();
        let results: Vec<_> = handles.into_iter().map(|handle| handle.join().unwrap()).collect();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(store.list::<GoalTurn>("goal_turn").unwrap().len(), 1);
        assert_eq!(store.snapshot().unwrap().events.iter().filter(|event| event.kind == "goal_question").count(), 1);
        drop(store);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn direction_change_and_completion_cannot_both_commit() {
        use std::sync::{Arc, Barrier};
        let dir = std::env::temp_dir().join(format!("utopia-guidance-race-{}", crate::model::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Arc::new(Store::open(&dir.join("state.sqlite")).unwrap());
        let goal = Goal { id: "g".into(), kind: "execution".into(), project_id: Some("p".into()), intent: "Original goal".into(), status: "validating".into(), report: String::new(), created_at: crate::model::now(), resumed_from: None, review_loops_used: 0, review_loop_limit: Some(2), work_group: None, block_kind: String::new() };
        store.put("goal", &goal.id, &goal, None).unwrap();
        let barrier = Arc::new(Barrier::new(3));
        let proposing = { let store = store.clone(); let barrier = barrier.clone(); std::thread::spawn(move || { barrier.wait(); store.propose_goal_guidance("g", "Change direction now", None, false).is_ok() }) };
        let completing = { let store = store.clone(); let barrier = barrier.clone(); let mut goal = goal.clone(); std::thread::spawn(move || { barrier.wait(); goal.status = "completed".into(); store.complete_goal_if_clear(&goal).is_ok() }) };
        barrier.wait();
        let proposal_won = proposing.join().unwrap();
        let completion_won = completing.join().unwrap();
        assert_ne!(proposal_won, completion_won);
        let saved: Goal = store.get("goal", "g").unwrap().unwrap();
        assert_eq!(saved.status == "completed", completion_won);
        assert_eq!(store.list::<GoalGuidance>("goal_guidance").unwrap().len(), usize::from(proposal_won));
        drop(store);
        let resolved = dir.canonicalize().unwrap();
        assert!(resolved.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(resolved.file_name().unwrap().to_string_lossy().starts_with("utopia-guidance-race-"));
        std::fs::remove_dir_all(resolved).unwrap();
    }
    #[test]
    fn direction_change_atomically_supersedes_pending_approval() {
        let dir = std::env::temp_dir().join(format!("utopia-guidance-approval-{}", crate::model::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("state.sqlite")).unwrap();
        let goal = Goal { id: "g".into(), kind: "execution".into(), project_id: Some("p".into()), intent: "Original goal".into(), status: "awaiting_approval".into(), report: String::new(), created_at: crate::model::now(), resumed_from: None, review_loops_used: 0, review_loop_limit: Some(2), work_group: None, block_kind: String::new() };
        store.put("goal", &goal.id, &goal, None).unwrap();
        let approval = Approval { id: "a".into(), goal_id: Some(goal.id), operation: "project_write".into(), reason: "Old files".into(), status: "pending".into(), created_at: crate::model::now() };
        store.put("approval", &approval.id, &approval, None).unwrap();
        let guidance = store.propose_goal_guidance("g", "Change the target files", Some("developer-a"), true).unwrap();
        assert_eq!(guidance.status, "pending");
        assert!(guidance.auto_continue);
        assert_eq!(guidance.addressed_agent_id.as_deref(), Some("developer-a"));
        assert_eq!(store.get::<Approval>("approval", "a").unwrap().unwrap().status, "superseded");
        drop(store);
        let reopened = Store::open(&dir.join("state.sqlite")).unwrap();
        let restored = reopened.get::<GoalGuidance>("goal_guidance", &guidance.id).unwrap().unwrap();
        assert_eq!(restored.addressed_agent_id.as_deref(), Some("developer-a"));
        assert!(restored.auto_continue);
        drop(reopened);
        let resolved = dir.canonicalize().unwrap();
        assert!(resolved.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(resolved.file_name().unwrap().to_string_lossy().starts_with("utopia-guidance-approval-"));
        std::fs::remove_dir_all(resolved).unwrap();
    }
    #[test]
    fn duplicate_revision_requests_create_one_linked_goal() {
        use std::sync::{Arc, Barrier};
        let dir = std::env::temp_dir().join(format!("utopia-revision-race-{}", crate::model::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Arc::new(Store::open(&dir.join("state.sqlite")).unwrap());
        store.select_project(&Project { id: "p".into(), path: "C:/test".into(), name: "Test".into(), kind: "git".into(), review_loop_limit: 3 }).unwrap();
        let goal = Goal { id: "g".into(), kind: "execution".into(), project_id: Some("p".into()), intent: "Original goal".into(), status: "blocked".into(), report: String::new(), created_at: crate::model::now(), resumed_from: Some("executing".into()), review_loops_used: 1, review_loop_limit: Some(3), work_group: Some("Group".into()), block_kind: String::new() };
        store.put("goal", &goal.id, &goal, None).unwrap();
        let guidance = GoalGuidance { id: "pivot".into(), goal_id: goal.id.clone(), text: "Change target".into(), status: "review_required".into(), created_at: crate::model::now(), checkpoint: Some("task wave".into()), replacement_goal_id: None, addressed_agent_id: None, worker_handoff: None, continuation: None, auto_continue: false };
        store.put("goal_guidance", &guidance.id, &guidance, None).unwrap();
        let barrier = Arc::new(Barrier::new(3));
        let handles = (0..2).map(|_| { let store = store.clone(); let barrier = barrier.clone(); std::thread::spawn(move || { barrier.wait(); store.create_goal_revision("pivot", "Revised goal outcome") }) }).collect::<Vec<_>>();
        barrier.wait();
        let results = handles.into_iter().map(|handle| handle.join().unwrap()).collect::<Vec<_>>();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        let snapshot = store.snapshot().unwrap();
        assert_eq!(snapshot.goals.len(), 2);
        let replacement = snapshot.goals.iter().find(|item| item.id != "g").unwrap();
        assert_eq!(replacement.review_loop_limit, Some(3));
        assert_eq!(replacement.work_group.as_deref(), Some("Group"));
        assert_eq!(snapshot.goal_guidance[0].replacement_goal_id.as_deref(), Some(replacement.id.as_str()));
        drop(store);
        let resolved = dir.canonicalize().unwrap();
        assert!(resolved.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(resolved.file_name().unwrap().to_string_lossy().starts_with("utopia-revision-race-"));
        std::fs::remove_dir_all(resolved).unwrap();
    }
    #[test]
    fn goal_conversation_survives_restart_and_marks_unfinished_reply() {
        let dir = std::env::temp_dir().join(format!("utopia-goal-turn-{}", crate::model::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.sqlite");
        {
            let store = Store::open(&path).unwrap();
            for (id, status, answer) in [("answered", "answered", "CTO answer"), ("pending", "pending", "")] {
                let turn = GoalTurn { id: id.into(), goal_id: "goal-one".into(), question: "What is the acceptance criterion?".into(), answer: answer.into(), status: status.into(), asked_at: crate::model::now(), answered_at: None, agent_id: "cto".into(), agent_name: "CTO".into(), connection_name: "Local AI".into(), model_id: "model-x".into(), guidance_id: None, route: None };
                store.put("goal_turn", id, &turn, None).unwrap();
            }
        }
        let snapshot = Store::open(&path).unwrap().snapshot().unwrap();
        assert_eq!(snapshot.goal_turns.len(), 2);
        assert_eq!(snapshot.goal_turns.iter().find(|turn| turn.id == "answered").unwrap().answer, "CTO answer");
        assert_eq!(snapshot.goal_turns.iter().find(|turn| turn.id == "answered").unwrap().agent_name, "CTO");
        let interrupted = snapshot.goal_turns.iter().find(|turn| turn.id == "pending").unwrap();
        assert_eq!(interrupted.status, "interrupted");
        assert!(interrupted.answer.contains("다시 질문"));
        let resolved = dir.canonicalize().unwrap();
        assert!(resolved.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(resolved.file_name().unwrap().to_string_lossy().starts_with("utopia-goal-turn-"));
        std::fs::remove_dir_all(resolved).unwrap();
    }
    #[test]
    fn restart_preserves_goal_guidance_and_invalidates_old_pending_approval() {
        let dir = std::env::temp_dir().join(format!("utopia-goal-guidance-{}", crate::model::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.sqlite");
        {
            let store = Store::open(&path).unwrap();
            let goal = Goal { id: "g".into(), kind: "execution".into(), project_id: Some("p".into()), intent: "Original approved scope".into(), status: "awaiting_approval".into(), report: String::new(), created_at: crate::model::now(), resumed_from: None, review_loops_used: 0, review_loop_limit: Some(2), work_group: None, block_kind: String::new() };
            store.put("goal", &goal.id, &goal, None).unwrap();
            let guidance = GoalGuidance { id: "change".into(), goal_id: goal.id.clone(), text: "Change the deliverable".into(), status: "pending".into(), created_at: crate::model::now(), checkpoint: None, replacement_goal_id: None, addressed_agent_id: None, worker_handoff: None, continuation: None, auto_continue: false };
            store.put("goal_guidance", &guidance.id, &guidance, None).unwrap();
            let approval = Approval { id: "approval".into(), goal_id: Some(goal.id), operation: "project_write".into(), reason: "Old scope".into(), status: "pending".into(), created_at: crate::model::now() };
            store.put("approval", &approval.id, &approval, None).unwrap();
        }
        let snapshot = Store::open(&path).unwrap().snapshot().unwrap();
        assert_eq!(snapshot.goal_guidance[0].status, "review_required");
        assert_eq!(snapshot.goal_guidance[0].checkpoint.as_deref(), Some("app_restart"));
        assert_eq!(snapshot.approvals[0].status, "superseded");
        assert_eq!(snapshot.goals[0].status, "blocked");
        let resolved = dir.canonicalize().unwrap();
        assert!(resolved.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(resolved.file_name().unwrap().to_string_lossy().starts_with("utopia-goal-guidance-"));
        std::fs::remove_dir_all(resolved).unwrap();
    }
    #[test]
    fn agent_progress_event_survives_restart_with_scope() {
        let dir = std::env::temp_dir().join(format!("utopia-progress-event-{}", crate::model::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.sqlite");
        {
            let store = Store::open(&path).unwrap();
            store.append_event(NewEvent { kind: "runtime_progress".into(), goal_id: Some("goal".into()), task_id: Some("task".into()), agent_id: Some("developer-a".into()), message: "파일 변경 중".into() }).unwrap();
        }
        let snapshot = Store::open(&path).unwrap().snapshot().unwrap();
        let event = snapshot.events.iter().find(|event| event.kind == "runtime_progress").unwrap();
        assert_eq!(event.goal_id.as_deref(), Some("goal"));
        assert_eq!(event.task_id.as_deref(), Some("task"));
        assert_eq!(event.agent_id.as_deref(), Some("developer-a"));
        assert_eq!(event.message, "파일 변경 중");
        let resolved = dir.canonicalize().unwrap();
        assert!(resolved.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(resolved.file_name().unwrap().to_string_lossy().starts_with("utopia-progress-event-"));
        std::fs::remove_dir_all(resolved).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn restart_repairs_saved_known_npm_shim_only() {
        let dir = std::env::temp_dir().join(format!("utopia-cli-repair-{}", crate::model::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("claude"), "#!/bin/sh\n").unwrap();
        std::fs::write(dir.join("claude.cmd"), "@echo off\r\necho test\r\n").unwrap();
        let path = dir.join("state.sqlite");
        {
            let store = Store::open(&path).unwrap();
            for id in ["claude", "custom"] {
                let connection = crate::model::AiConnection { id: id.into(), name: id.into(), kind: "cli".into(), executable: dir.join("claude").to_string_lossy().into_owned(), endpoint: String::new(), args: vec![], model_id: String::new(), status: "failed".into(), capabilities: vec![] };
                store.put("connection", id, &connection, None).unwrap();
            }
        }
        let store = Store::open(&path).unwrap();
        let repaired: crate::model::AiConnection = store.get("connection", "claude").unwrap().unwrap();
        assert_eq!(repaired.executable, dir.join("claude.cmd").to_string_lossy());
        assert_eq!(repaired.status, "needs_auth_check");
        let custom: crate::model::AiConnection = store.get("connection", "custom").unwrap().unwrap();
        assert_eq!(custom.executable, dir.join("claude").to_string_lossy());
        drop(store);
        let resolved = dir.canonicalize().unwrap();
        assert!(resolved.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(resolved.file_name().unwrap().to_string_lossy().starts_with("utopia-cli-repair-"));
        std::fs::remove_dir_all(resolved).unwrap();
    }
    #[test]
    fn connection_registration_fills_only_unassigned_members() {
        let dir = std::env::temp_dir().join(format!("utopia-connection-assign-{}", crate::model::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("state.sqlite")).unwrap();
        let mut cto: Agent = store.get("agent", "cto").unwrap().unwrap();
        cto.connection_id = "existing".into();
        store.put("agent", "cto", &cto, None).unwrap();
        let connection = crate::model::AiConnection { id: "new".into(), name: "New AI".into(), kind: "cli".into(), executable: "tool.cmd".into(), endpoint: String::new(), args: vec![], model_id: String::new(), status: "ready".into(), capabilities: vec![] };
        assert_eq!(store.register_connection_and_assign_unassigned(&connection).unwrap(), 4);
        assert_eq!(store.register_connection_and_assign_unassigned(&connection).unwrap(), 0);
        assert_eq!(store.get::<Agent>("agent", "cto").unwrap().unwrap().connection_id, "existing");
        assert_eq!(store.get::<Agent>("agent", "pm").unwrap().unwrap().connection_id, "new");
        drop(store);
        let resolved = dir.canonicalize().unwrap();
        assert!(resolved.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(resolved.file_name().unwrap().to_string_lossy().starts_with("utopia-connection-assign-"));
        std::fs::remove_dir_all(resolved).unwrap();
    }

    #[test]
    fn switching_projects_preserves_goal_binding_after_restart() {
        let dir = std::env::temp_dir().join(format!("utopia-project-binding-{}", crate::model::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.sqlite");
        let first = Project { id: "first".into(), path: "C:/first".into(), name: "First".into(), kind: "git".into(), review_loop_limit: 2 };
        let second = Project { id: "second".into(), path: "C:/second".into(), name: "Second".into(), kind: "git".into(), review_loop_limit: 2 };
        {
            let store = Store::open(&path).unwrap();
            store.select_project(&first).unwrap();
            let goal = Goal { id: "goal-a".into(), kind: "execution".into(), project_id: Some(first.id.clone()), intent: "Keep this goal in project one".into(), status: "new".into(), report: String::new(), created_at: crate::model::now(), resumed_from: None, review_loops_used: 0, review_loop_limit: None, work_group: Some("Release".into()), block_kind: String::new() };
            store.put("goal", &goal.id, &goal, None).unwrap();
            store.select_project(&second).unwrap();
        }
        let snapshot = Store::open(&path).unwrap().snapshot().unwrap();
        assert_eq!(snapshot.project.unwrap().id, second.id);
        assert_eq!(snapshot.projects.len(), 2);
        assert_eq!(snapshot.goals[0].project_id.as_deref(), Some(first.id.as_str()));
        assert_eq!(snapshot.goals[0].work_group.as_deref(), Some("Release"));
        let resolved = dir.canonicalize().unwrap();
        assert!(resolved.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(resolved.file_name().unwrap().to_string_lossy().starts_with("utopia-project-binding-"));
        std::fs::remove_dir_all(resolved).unwrap();
    }
    #[test]
    fn restart_preserves_review_limit_and_token_totals() {
        let dir = std::env::temp_dir().join(format!("utopia-usage-restart-{}", crate::model::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.sqlite");
        {
            let store = Store::open(&path).unwrap();
            let project = Project { id: "p".into(), path: "C:/project".into(), name: "Project".into(), kind: "git".into(), review_loop_limit: 4 };
            store.select_project(&project).unwrap();
            let usage = crate::model::TokenUsageRecord { id: "u".into(), project_id: "p".into(), goal_id: "g".into(), agent_id: "cto".into(), task_id: None, connection_id: "codex".into(), connection_name: "Codex".into(), model_id: "test-model".into(), input_tokens: 120, output_tokens: 30, source: "reported".into(), status: "completed".into(), updated_at: crate::model::now(), meeting_id: None };
            store.put("usage", "u", &usage, None).unwrap();
        }
        let snapshot = Store::open(&path).unwrap().snapshot().unwrap();
        assert_eq!(snapshot.project.unwrap().review_loop_limit, 4);
        assert_eq!(snapshot.usage[0].input_tokens + snapshot.usage[0].output_tokens, 150);
        assert_eq!(snapshot.usage[0].status, "completed");
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn legacy_goal_copies_project_limit_only_once() {
        let dir = std::env::temp_dir().join(format!("utopia-goal-limit-{}", crate::model::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.sqlite");
        {
            let store = Store::open(&path).unwrap();
            let project = Project { id: "p".into(), path: "C:/project".into(), name: "Project".into(), kind: "git".into(), review_loop_limit: 4 };
            store.select_project(&project).unwrap();
            let goal = Goal { id: "g".into(), kind: "execution".into(), project_id: Some("p".into()), intent: "legacy goal".into(), status: "new".into(), report: String::new(), created_at: crate::model::now(), resumed_from: None, review_loops_used: 0, review_loop_limit: None, work_group: None, block_kind: String::new() };
            store.put("goal", "g", &goal, None).unwrap();
        }
        let store = Store::open(&path).unwrap();
        assert_eq!(store.get::<Goal>("goal", "g").unwrap().unwrap().review_loop_limit, Some(4));
        let mut project: Project = store.get("project_entry", "p").unwrap().unwrap();
        project.review_loop_limit = 1;
        store.select_project(&project).unwrap();
        drop(store);
        assert_eq!(Store::open(&path).unwrap().get::<Goal>("goal", "g").unwrap().unwrap().review_loop_limit, Some(4));
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn unfinished_assigned_tasks_prevent_hiding_a_member() {
        let dir = std::env::temp_dir().join(format!("utopia-agent-hide-{}", crate::model::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("state.sqlite")).unwrap();
        let mut task = Task { id: "t".into(), goal_id: "g".into(), title: "Implement".into(), instructions: String::new(), files: vec!["file.txt".into()], assignee_id: "developer-a".into(), status: "pending".into(), depends_on: vec![], worktree: None, result: String::new(), session_id: None, deliverable: String::new(), acceptance: vec![], evidence: vec![], capability: String::new(), assignment_reason: String::new() };
        store.put("task", "t", &task, None).unwrap();
        assert_eq!(store.unfinished_task_count("developer-a").unwrap(), 1);
        task.status = "completed".into();
        store.put("task", "t", &task, None).unwrap();
        assert_eq!(store.unfinished_task_count("developer-a").unwrap(), 0);
        drop(store);
        let resolved = dir.canonicalize().unwrap();
        assert!(resolved.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(resolved.file_name().unwrap().to_string_lossy().starts_with("utopia-agent-hide-"));
        std::fs::remove_dir_all(resolved).unwrap();
    }
    #[test]
    fn restart_marks_running_work_as_blocked_without_completion() {
        let dir = std::env::temp_dir().join(format!("utopia-store-test-{}", crate::model::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.sqlite");
        {
            let store = Store::open(&path).unwrap();
            let goal = Goal { id: "g".into(), kind: "execution".into(), project_id: None, intent: "Implement a small feature".into(), status: "executing".into(), report: String::new(), created_at: crate::model::now(), resumed_from: None, review_loops_used: 0, review_loop_limit: None, work_group: None, block_kind: String::new() };
            store.put("goal", "g", &goal, None).unwrap();
            let task = Task { id: "t".into(), goal_id: "g".into(), title: "Change a file".into(), instructions: String::new(), files: vec!["example.txt".into()], assignee_id: "developer-a".into(), status: "running".into(), depends_on: vec![], worktree: None, result: String::new(), session_id: None, deliverable: String::new(), acceptance: vec![], evidence: vec![], capability: String::new(), assignment_reason: String::new() };
            store.put("task", "t", &task, None).unwrap();
        }
        let recovered = Store::open(&path).unwrap().snapshot().unwrap();
        assert_eq!(recovered.goals[0].status, "blocked");
        assert_eq!(recovered.goals[0].resumed_from.as_deref(), Some("executing"));
        assert_eq!(recovered.goals[0].block_kind, "interrupted");
        assert_eq!(recovered.tasks[0].status, "blocked");
        assert!(recovered.events.iter().any(|e| e.kind == "task_interrupted"));
        // An interrupted (outcome unknown) task never makes the plan resumable automatically.
        let approvals = vec![crate::model::Approval { id: "a".into(), goal_id: Some("g".into()), operation: "project_write".into(), reason: String::new(), status: "approved".into(), created_at: crate::model::now() }];
        assert!(crate::delegation::resumable(&recovered.goals[0], &recovered.tasks, &approvals).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn restart_preserves_pending_approval_and_planned_tasks() {
        use crate::model::Approval;
        let dir = std::env::temp_dir().join(format!("utopia-approval-restart-{}", crate::model::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.sqlite");
        {
            let store = Store::open(&path).unwrap();
            let goal = Goal { id: "g".into(), kind: "execution".into(), project_id: None, intent: "Two parallel tasks".into(), status: "awaiting_approval".into(), report: "CTO analysis".into(), created_at: crate::model::now(), resumed_from: None, review_loops_used: 0, review_loop_limit: None, work_group: None, block_kind: String::new() };
            store.put("goal", "g", &goal, None).unwrap();
            let approval = Approval { id: "a".into(), goal_id: Some("g".into()), operation: "project_write".into(), reason: "Apply planned files".into(), status: "pending".into(), created_at: crate::model::now() };
            store.put("approval", "a", &approval, None).unwrap();
            let task = Task { id: "t".into(), goal_id: "g".into(), title: "Create file".into(), instructions: String::new(), files: vec!["example.txt".into()], assignee_id: "developer-a".into(), status: "pending".into(), depends_on: vec![], worktree: None, result: String::new(), session_id: None, deliverable: String::new(), acceptance: vec![], evidence: vec![], capability: String::new(), assignment_reason: String::new() };
            store.put("task", "t", &task, None).unwrap();
        }
        let recovered = Store::open(&path).unwrap().snapshot().unwrap();
        assert_eq!(recovered.goals[0].status, "blocked");
        assert_eq!(recovered.goals[0].resumed_from.as_deref(), Some("awaiting_approval"));
        assert_eq!(recovered.goals[0].report, "CTO analysis");
        assert_eq!(recovered.approvals[0].status, "pending");
        assert_eq!(recovered.tasks[0].status, "pending");
        assert!(recovered.events.iter().any(|e| e.kind == "goal_interrupted"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn migrates_existing_unversioned_database_without_losing_objects() {
        let dir = std::env::temp_dir().join(format!("utopia-migration-{}", crate::model::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.sqlite");
        {
            let legacy = Connection::open(&path).unwrap();
            legacy.execute_batch(
                "CREATE TABLE objects (kind TEXT NOT NULL, id TEXT NOT NULL, payload TEXT NOT NULL, PRIMARY KEY(kind,id));
                 CREATE TABLE events (id INTEGER PRIMARY KEY AUTOINCREMENT, at TEXT NOT NULL, kind TEXT NOT NULL, goal_id TEXT, task_id TEXT, agent_id TEXT, message TEXT NOT NULL);"
            ).unwrap();
            let goal = Goal { id: "legacy".into(), kind: "execution".into(), project_id: None, intent: "Preserve me".into(), status: "completed".into(), report: "done".into(), created_at: crate::model::now(), resumed_from: None, review_loops_used: 0, review_loop_limit: None, work_group: None, block_kind: String::new() };
            let mut old_payload = serde_json::to_value(&goal).unwrap();
            old_payload.as_object_mut().unwrap().remove("kind");
            legacy.execute("INSERT INTO objects(kind,id,payload) VALUES('goal','legacy',?1)", [old_payload.to_string()]).unwrap();
            legacy.execute("INSERT INTO objects(kind,id,payload) VALUES('project','selected',?1)", [r#"{"path":"C:/old-project","name":"old-project"}"#]).unwrap();
        }
        let store = Store::open(&path).unwrap();
        assert_eq!(store.get::<Goal>("goal", "legacy").unwrap().unwrap().report, "done");
        let version: i64 = store.0.lock().unwrap().query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
        assert_eq!(version, 4);
        assert_eq!(store.get::<Goal>("goal", "legacy").unwrap().unwrap().kind, "execution");
        assert_eq!(store.snapshot().unwrap().projects.len(), 1);
        assert_eq!(store.snapshot().unwrap().project.unwrap().id, store.snapshot().unwrap().projects[0].id);
        assert_eq!(store.get::<Goal>("goal", "legacy").unwrap().unwrap().project_id, None);
        drop(store);
        let newer = Connection::open(&path).unwrap();
        newer.execute_batch("PRAGMA user_version=5").unwrap();
        drop(newer);
        assert!(Store::open(&path).err().unwrap().contains("newer than this app supports"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn bulk_connection_assignment_is_atomic_and_preserves_member_fields() {
        let dir = std::env::temp_dir().join(format!("utopia-team-bulk-{}", crate::model::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("state.sqlite")).unwrap();
        let cli = crate::model::AiConnection { id: "claude".into(), name: "Claude".into(), kind: "cli".into(), executable: "claude.exe".into(), endpoint: String::new(), args: vec![], model_id: String::new(), status: "needs_setup".into(), capabilities: vec![] };
        let http = crate::model::AiConnection { id: "http".into(), name: "HTTP".into(), kind: "http".into(), executable: String::new(), endpoint: "http://127.0.0.1:1234".into(), args: vec![], model_id: "model".into(), status: "ready".into(), capabilities: vec![] };
        store.put("connection", "claude", &cli, None).unwrap();
        store.put("connection", "http", &http, None).unwrap();
        let mut cto: Agent = store.get("agent", "cto").unwrap().unwrap();
        cto.model_id = "old-model".into();
        let original_role = cto.role.clone();
        store.put("agent", "cto", &cto, None).unwrap();
        assert_eq!(store.assign_connection_to_agents("claude", &["cto".into(), "developer-a".into()]).unwrap(), 2);
        let changed: Agent = store.get("agent", "cto").unwrap().unwrap();
        assert_eq!(changed.connection_id, "claude");
        assert!(changed.model_id.is_empty());
        assert_eq!(changed.role, original_role);
        assert!(store.get::<Agent>("agent", "pm").unwrap().unwrap().connection_id.is_empty());
        assert!(store.assign_connection_to_agents("http", &["cto".into(), "developer-a".into()]).unwrap_err().contains("HTTP"));
        assert_eq!(store.get::<Agent>("agent", "cto").unwrap().unwrap().connection_id, "claude");
        assert!(store.assign_connection_to_agents("http", &["cto".into(), "missing".into()]).is_err());
        assert_eq!(store.get::<Agent>("agent", "cto").unwrap().unwrap().connection_id, "claude");
        drop(store);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
