use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    #[serde(default)]
    pub id: String,
    pub path: String,
    pub name: String,
    #[serde(default = "default_project_kind")]
    pub kind: String,
    #[serde(default = "default_review_loop_limit")]
    pub review_loop_limit: u32,
}

pub fn default_project_kind() -> String { "git".into() }

pub fn default_review_loop_limit() -> u32 { 2 }

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub role: String,
    pub instructions: String,
    pub reports_to: Option<String>,
    pub connection_id: String,
    pub model_id: String,
    pub x: i32,
    pub y: i32,
    pub enabled: bool,
    /// Set only for external staff imported from a character file (ADR-0041).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external: Option<ExternalContract>,
    /// Office look composed from the MetroCity layers (ADR-0040); absent means the default look.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appearance: Option<Appearance>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Appearance {
    pub skin: u8,
    pub outfit: String,
    pub hair: String,
    #[serde(default)]
    pub beard: bool,
}

impl Appearance {
    /// Only choices the Office character catalog knows (src/office/metrocity.ts).
    pub fn validate(&self) -> Result<(), String> {
        let numbered = |value: &str, prefix: &str, allowed: &[u8]| value.strip_prefix(prefix).and_then(|n| n.parse::<u8>().ok()).is_some_and(|n| allowed.contains(&n));
        if self.skin > 5 { return Err("캐릭터 피부색 선택이 올바르지 않습니다".into()); }
        if !(numbered(&self.outfit, "shirt-", &[0, 1, 2, 3, 4]) || numbered(&self.outfit, "outfit-", &[1, 2, 3, 4, 5, 6]) || numbered(&self.outfit, "suit-", &[0, 1, 2, 3])) { return Err("캐릭터 옷 선택이 올바르지 않습니다".into()); }
        if !(self.hair == "none" || numbered(&self.hair, "hairs-", &[0, 1, 2, 3, 4, 5, 6, 7]) || numbered(&self.hair, "hair2-", &[0, 2, 3, 4])) { return Err("캐릭터 머리 선택이 올바르지 않습니다".into()); }
        Ok(())
    }
}

/// Goal-only contract of an imported character: `standby` → `contracted` (one goal) → `released`,
/// or `hired` when the CEO made it a regular roster member.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExternalContract {
    pub status: String,
    #[serde(default)]
    pub goal_id: Option<String>,
    pub slot: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    pub source_name: String,
    pub source_sha256: String,
    pub imported_at: String,
    #[serde(default)]
    pub released_at: Option<String>,
}

impl Agent {
    /// External staff whose contract still restricts it (standby, contracted or released).
    pub fn is_external(&self) -> bool { self.external.as_ref().is_some_and(|item| item.status != "hired") }
}

/// A goal's squad (ADR-0041): which roster members lead, write and review, and why.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Squad {
    pub goal_id: String,
    pub structure: String,
    #[serde(default)]
    pub structure_reason: String,
    pub members: Vec<SquadMember>,
    pub status: String,
    #[serde(default)]
    pub shortfall: Vec<String>,
    #[serde(default)]
    pub approval_id: Option<String>,
    #[serde(default)]
    pub changes: Vec<SquadChange>,
    pub created_at: String,
    #[serde(default)]
    pub locked_at: Option<String>,
    #[serde(default)]
    pub released_at: Option<String>,
    /// Filled in snapshots only, so the approval card can send back the digest it showed.
    #[serde(default)]
    pub digest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SquadMember {
    pub agent_id: String,
    pub slot: String,
    #[serde(default)]
    pub duty: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SquadChange {
    pub at: String,
    pub slot: String,
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConnection {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub executable: String,
    pub endpoint: String,
    pub args: Vec<String>,
    pub model_id: String,
    pub status: String,
    pub capabilities: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Goal {
    pub id: String,
    #[serde(default = "default_goal_kind")]
    pub kind: String,
    #[serde(default)]
    pub project_id: Option<String>,
    pub intent: String,
    pub status: String,
    pub report: String,
    pub created_at: String,
    pub resumed_from: Option<String>,
    #[serde(default)]
    pub review_loops_used: u32,
    #[serde(default)]
    pub review_loop_limit: Option<u32>,
    #[serde(default)]
    pub work_group: Option<String>,
    /// Engine-assigned class of the last block (ADR-0038): precall, approval_denied, interrupted,
    /// integration_conflict, qa_failed, qa_form, provider or other. Never derived from model text.
    #[serde(default)]
    pub block_kind: String,
}

pub fn default_goal_kind() -> String { "execution".into() }

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSlot {
    /// Slot ID; for Office-character meetings this is the Agent ID.
    pub id: String,
    pub viewpoint: String,
    pub instructions: String,
    pub connection_id: String,
    pub connection_fingerprint: String,
    pub model_id: String,
    /// `chair`, `participant` or `evaluator` (empty for earlier fixed-slot meetings).
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub agent_name: String,
    #[serde(default)]
    pub role: String,
    /// Read-only execution profile recorded when the slot was snapshotted.
    #[serde(default)]
    pub profile: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanMember {
    pub agent_id: String,
    #[serde(default)]
    pub stance: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSource {
    pub path: String,
    #[serde(default)]
    pub reason: String,
}

/// Immutable chair/CEO plan revision (ME-01). `digest` covers everything except `errors`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingPlan {
    pub revision: u32,
    pub digest: String,
    pub objective: String,
    #[serde(default)]
    pub constraints: Vec<String>,
    pub participants: Vec<PlanMember>,
    pub evaluator_agent_id: String,
    pub protocol: String,
    pub rounds: u32,
    pub max_calls: u32,
    #[serde(default)]
    pub sources: Vec<PlanSource>,
    #[serde(default)]
    pub success_criteria: Vec<String>,
    pub origin: String,
    #[serde(default)]
    pub errors: Vec<String>,
    pub created_at: String,
}

/// CEO approval of one exact plan revision (ME-02).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingPlanApproval {
    pub revision: u32,
    pub digest: String,
    pub mode: String,
    pub status: String,
    pub decided_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSource {
    pub id: String,
    pub path: String,
    pub sha256: String,
    #[serde(default)]
    pub byte_length: usize,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meeting {
    pub id: String,
    pub goal_id: String,
    pub project_id: String,
    pub topic: String,
    pub stage: String,
    pub slots: Vec<MeetingSlot>,
    pub sources: Vec<MeetingSource>,
    pub calls_used: u32,
    pub max_calls: u32,
    #[serde(default)]
    pub package: Option<DecisionPackage>,
    #[serde(default)]
    pub decision: Option<MeetingDecision>,
    pub created_at: String,
    pub updated_at: String,
    /// Packet stage whose packets are being decided or dispatched (`proposing`..`evaluating`).
    #[serde(default)]
    pub active_stage: String,
    /// Persisted, user-visible reason for `blocked`/`stopped`/partial closure.
    #[serde(default)]
    pub block_reason: String,
    #[serde(default)]
    pub blocked_at: String,
    #[serde(default)]
    pub block_build: String,
    #[serde(default)]
    pub use_source_snapshot: bool,
    #[serde(default)]
    pub source_change_noted: bool,
    /// Retained only so earlier-model meetings deserialize; new meetings never use it.
    #[serde(default)]
    pub scoped_grant: Option<ScopedGrant>,
    /// 0 = earlier fixed-slot model (read-only), 2 = Office-character model.
    #[serde(default)]
    pub model_version: u32,
    #[serde(default)]
    pub chair_agent_id: String,
    #[serde(default)]
    pub plan: Option<MeetingPlan>,
    #[serde(default)]
    pub plan_approval: Option<MeetingPlanApproval>,
    /// Plain-language one-line topic from the chair's plan reply (ADR-0042); display only.
    #[serde(default)]
    pub topic_summary: String,
}

/// Meeting-scoped `미리 승인` grant (ME-02). Only usable when every destination is closed-text eligible.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopedGrant {
    pub id: String,
    pub status: String,
    pub granted_at: String,
    pub expires_at: String,
    pub remaining_calls: u32,
    pub destination_fingerprints: Vec<String>,
    pub source_hashes: Vec<String>,
    pub template_version: u32,
    pub byte_cap: usize,
    #[serde(default)]
    pub ended_reason: String,
    #[serde(default)]
    pub policy: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PacketDispatch {
    pub started_at: String,
    pub finished_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub command_chars: usize,
    pub connection_fingerprint: String,
    pub connection_name: String,
    pub model_id: String,
    pub runtime_version: String,
    pub isolation_policy: String,
    pub outcome: String,
    pub error: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub usage_source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceCheck {
    pub owner_id: String,
    pub source_id: String,
    pub quote: String,
    pub status: String,
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub occurrences: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceLink {
    pub rel: String,
    pub target: String,
}

/// Engine-built argument-graph node of a decision package (ME-01 traceability).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceNode {
    pub id: String,
    pub kind: String,
    pub author: String,
    pub stage: String,
    pub packet_id: String,
    pub packet_sha256: String,
    pub text: String,
    pub links: Vec<TraceLink>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingContribution {
    pub id: String,
    pub meeting_id: String,
    pub packet_id: String,
    pub stage: String,
    pub slot_id: String,
    pub raw_response: String,
    pub content: serde_json::Value,
    pub source_checks: Vec<SourceCheck>,
    pub connection_fingerprint: String,
    pub model_id: String,
    pub offline_fixture: bool,
    pub created_at: String,
    #[serde(default)]
    pub normalizations: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageClaim {
    pub id: String,
    pub slot_id: String,
    pub text: String,
    pub revised: bool,
    pub source_status: String,
    pub assumptions: Vec<String>,
    pub unknowns: Vec<String>,
    pub evaluator_verdict: String,
    pub evaluator_note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageRecommendation {
    pub id: String,
    pub text: String,
    pub claim_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageDissent {
    pub critique_id: String,
    pub critic_slot_id: String,
    pub target_claim_id: String,
    pub text: String,
    pub disposition: String,
    pub response: String,
}

/// A plain-language open issue grouped by the evaluator (ADR-0042); display only.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackageIssue {
    pub text: String,
    pub critique_ids: Vec<String>,
}

/// Plain-language one-line goal summary from the CTO analysis reply (ADR-0042); display only.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GoalSummary {
    pub goal_id: String,
    pub text: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageProvenance {
    pub slot_id: String,
    pub viewpoint: String,
    pub connection_name: String,
    pub model_id: String,
    pub connection_fingerprint: String,
    pub offline_fixture: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageUsage {
    pub packet_id: String,
    pub stage: String,
    pub slot_id: String,
    pub outcome: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub usage_source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionPackage {
    pub id: String,
    pub sha256: String,
    pub partial: bool,
    pub termination_reason: String,
    pub recommendations: Vec<PackageRecommendation>,
    pub claims: Vec<PackageClaim>,
    pub alternatives: Vec<String>,
    pub options: Vec<String>,
    pub dissent: Vec<PackageDissent>,
    pub provenance: Vec<PackageProvenance>,
    pub usage: Vec<PackageUsage>,
    pub independence_note: String,
    pub created_at: String,
    #[serde(default)]
    pub trace: Vec<TraceNode>,
    #[serde(default)]
    pub plan_digest: String,
    /// Evaluator's plain-language whole-meeting summary, recommended proposal and grouped issues (ADR-0042).
    #[serde(default)]
    pub overview: String,
    #[serde(default)]
    pub recommended_id: String,
    #[serde(default)]
    pub issues: Vec<PackageIssue>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingDecision {
    pub kind: String,
    pub recommendation_ids: Vec<String>,
    pub note: String,
    pub package_sha256: String,
    pub decided_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalLineage {
    pub goal_id: String,
    pub meeting_id: String,
    pub package_sha256: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingPacket {
    pub id: String,
    pub meeting_id: String,
    pub stage: String,
    pub slot_id: String,
    pub payload: String,
    pub payload_sha256: String,
    #[serde(default)]
    pub byte_length: usize,
    #[serde(default)]
    pub source_snapshot_ids: Vec<String>,
    pub preview_revision: u32,
    pub destination_fingerprint: String,
    pub status: String,
    pub created_at: String,
    /// Packet template version; packets from another template are never dispatched.
    #[serde(default)]
    pub template_version: u32,
    /// The previous revision for this slot/stage may already have reached the provider.
    #[serde(default)]
    pub prior_outcome_unknown: bool,
    #[serde(default)]
    pub dispatch: Option<PacketDispatch>,
    #[serde(default)]
    pub plan_digest: String,
    #[serde(default)]
    pub repair_of: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransmissionApproval {
    pub id: String,
    pub meeting_id: String,
    pub packet_id: String,
    pub preview_revision: u32,
    pub payload_sha256: String,
    pub status: String,
    pub decided_at: Option<String>,
    /// Dispatch contract the decision was made under; 0 marks offline-slice decisions that never dispatch.
    #[serde(default)]
    pub dispatch_contract: u32,
    /// `exact` (CEO decided this packet) or `scoped` (covered by a meeting grant).
    #[serde(default)]
    pub mode: String,
    /// Read-only profile of the destination when decided; `ineligible` can never dispatch.
    #[serde(default)]
    pub eligibility: String,
    #[serde(default)]
    pub plan_digest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalTurn {
    pub id: String,
    pub goal_id: String,
    pub question: String,
    pub answer: String,
    pub status: String,
    pub asked_at: String,
    pub answered_at: Option<String>,
    pub agent_id: String,
    #[serde(default)]
    pub agent_name: String,
    pub connection_name: String,
    pub model_id: String,
    #[serde(default)]
    pub guidance_id: Option<String>,
    #[serde(default)]
    pub route: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalGuidance {
    pub id: String,
    pub goal_id: String,
    pub text: String,
    pub status: String,
    pub created_at: String,
    pub checkpoint: Option<String>,
    pub replacement_goal_id: Option<String>,
    #[serde(default)]
    pub addressed_agent_id: Option<String>,
    #[serde(default)]
    pub worker_handoff: Option<String>,
    #[serde(default)]
    pub continuation: Option<ContinuationDecision>,
    #[serde(default)]
    pub auto_continue: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDigest {
    pub path: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskIntegrationProof {
    pub task_id: String,
    pub goal_id: String,
    pub files: Vec<FileDigest>,
    pub integrated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinuationDecision {
    pub source_goal_id: String,
    pub reused_task_ids: Vec<String>,
    pub new_task_count: usize,
    pub reviewed_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsageRecord {
    pub id: String,
    pub project_id: String,
    pub goal_id: String,
    pub agent_id: String,
    pub task_id: Option<String>,
    #[serde(default)]
    pub connection_id: String,
    #[serde(default)]
    pub connection_name: String,
    #[serde(default)]
    pub model_id: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub source: String,
    pub status: String,
    pub updated_at: String,
    /// Set for meeting calls; the record's agent_id is then the real Office member.
    #[serde(default)]
    pub meeting_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub goal_id: String,
    pub title: String,
    pub instructions: String,
    #[serde(default)]
    pub files: Vec<String>,
    pub assignee_id: String,
    pub status: String,
    pub depends_on: Vec<String>,
    pub worktree: Option<String>,
    pub result: String,
    pub session_id: Option<String>,
    /// Deliverable contract (ADR-0038). Empty on tasks created before the contract.
    #[serde(default)]
    pub deliverable: String,
    #[serde(default)]
    pub acceptance: Vec<String>,
    /// Read-only input files inside the project the deliverable is based on.
    #[serde(default)]
    pub evidence: Vec<String>,
    /// `document` or `code`.
    #[serde(default)]
    pub capability: String,
    /// Why the engine chose this assignee.
    #[serde(default)]
    pub assignment_reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CriterionResult {
    pub criterion: String,
    pub met: bool,
    #[serde(default)]
    pub evidence: String,
}

/// Independent QA verdict for one deliverable task in one QA round (ADR-0038).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliverableReview {
    pub id: String,
    pub goal_id: String,
    pub task_id: String,
    pub round: u32,
    pub verdict: String,
    pub criteria: Vec<CriterionResult>,
    #[serde(default)]
    pub unverified: Vec<String>,
    pub reviewer_id: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: i64,
    pub at: String,
    pub kind: String,
    pub goal_id: Option<String>,
    pub task_id: Option<String>,
    pub agent_id: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Approval {
    pub id: String,
    pub goal_id: Option<String>,
    pub operation: String,
    pub reason: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub project: Option<Project>,
    pub projects: Vec<Project>,
    pub agents: Vec<Agent>,
    pub connections: Vec<AiConnection>,
    pub goals: Vec<Goal>,
    pub meetings: Vec<Meeting>,
    pub meeting_packets: Vec<MeetingPacket>,
    pub transmission_approvals: Vec<TransmissionApproval>,
    #[serde(default)]
    pub meeting_contributions: Vec<MeetingContribution>,
    #[serde(default)]
    pub goal_lineage: Vec<GoalLineage>,
    pub goal_turns: Vec<GoalTurn>,
    pub goal_guidance: Vec<GoalGuidance>,
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub deliverable_reviews: Vec<DeliverableReview>,
    #[serde(default)]
    pub squads: Vec<Squad>,
    #[serde(default)]
    pub goal_summaries: Vec<GoalSummary>,
    #[serde(default)]
    pub archived_goals: Vec<crate::archive::ArchivedGoal>,
    pub events: Vec<Event>,
    pub approvals: Vec<Approval>,
    pub usage: Vec<TokenUsageRecord>,
    pub spec_profiles: Vec<crate::spec_import::SpecProfile>,
    pub spec_drafts: Vec<crate::spec_import::SpecDraft>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredAi {
    pub connection: AiConnection,
    pub source: String,
    pub version: Option<String>,
    pub detail: String,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RolePreset {
    pub id: String,
    pub name: String,
    pub role: String,
    pub summary: String,
    pub instructions: String,
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
