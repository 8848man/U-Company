//! ADR-0048: the recovery card's one-click `해결 요청`. The app, not an AI, decides where a blocked goal
//! goes and writes the fix request from saved records, so a click can never widen the approved scope.

use crate::model::{Approval, DeliverableReview, Goal, GoalGuidance, Task};

/// Where a one-click fix sends a blocked goal.
#[derive(Debug, Clone, PartialEq)]
pub enum FixRoute {
    /// The planning lead plans a QA rework on the same goal, inside the approved files; no new approval.
    Repair,
    /// A new goal version is created and planned automatically; its plan needs the CEO's approval.
    Revision,
    /// Not something a teammate can fix (connections, a CEO decision); the reason says what to do.
    Refused(String),
}

const REPAIR_KINDS: [&str; 4] = ["plan_form", "qa_failed", "qa_form", "other"];

/// The route for this goal, from saved state only.
pub fn route(goal: &Goal, tasks: &[Task], approvals: &[Approval], guidance: &[GoalGuidance]) -> FixRoute {
    if goal.kind != "execution" { return FixRoute::Refused("회의 목표는 회의 화면에서 이어 가 주세요".into()); }
    if goal.status != "blocked" { return FixRoute::Refused("멈춘 목표가 아닙니다. 화면을 새로고침해 주세요".into()); }
    if goal.project_id.is_none() { return FixRoute::Refused("프로젝트가 연결되지 않은 목표입니다".into()); }
    let own_guidance: Vec<&GoalGuidance> = guidance.iter().filter(|item| item.goal_id == goal.id).collect();
    if own_guidance.iter().any(|item| ["pending", "review_required"].contains(&item.status.as_str())) { return FixRoute::Refused("검토 중인 방향 변경이 있습니다. 대화에서 먼저 확인해 주세요".into()); }
    if own_guidance.iter().any(|item| item.status == "revised" && item.replacement_goal_id.is_some()) { return FixRoute::Refused("이 목표는 이미 새 버전으로 이어졌습니다. 새 버전을 열어 주세요".into()); }
    match goal.block_kind.as_str() {
        "provider" | "precall" => return FixRoute::Refused("AI 연결이나 팀원 상태 문제라 팀원이 고칠 수 없습니다. 연결을 확인한 뒤 다시 실행해 주세요".into()),
        "approval_denied" => return FixRoute::Refused("CEO가 거절한 범위입니다. 무엇을 바꿀지 대화에 직접 적어 주세요".into()),
        _ => {}
    }
    let owned: Vec<&Task> = tasks.iter().filter(|task| task.goal_id == goal.id).collect();
    let approved = approvals.iter().any(|item| item.goal_id.as_deref() == Some(goal.id.as_str()) && item.operation == "project_write" && item.status == "approved");
    let after_review = goal.resumed_from.as_deref() == Some("validating");
    if REPAIR_KINDS.contains(&goal.block_kind.as_str()) && after_review && approved && own_guidance.is_empty() && !owned.is_empty() && owned.iter().all(|task| task.status == "completed") {
        return FixRoute::Repair;
    }
    FixRoute::Revision
}

/// The QA rework limit the repair will run under: a click past the limit adds exactly one round.
pub fn limit_after_click(used: u32, limit: u32) -> Option<u32> {
    if used >= limit { Some(used + 1) } else { None }
}

/// The fix request, written by the app from the block message and the latest QA verdicts.
/// QA text is model output; it is quoted as data and never changes what the request may touch.
pub fn brief(goal: &Goal, reviews: &[DeliverableReview], tasks: &[Task]) -> String {
    let reason = goal.report.strip_prefix("Blocked: ").unwrap_or(&goal.report).trim();
    let mut lines = vec![format!("멈춘 이유: {}", reason.chars().take(600).collect::<String>())];
    let own: Vec<&DeliverableReview> = reviews.iter().filter(|item| item.goal_id == goal.id).collect();
    if let Some(round) = own.iter().map(|item| item.round).max() {
        let mut failed = Vec::new();
        for review in own.iter().filter(|item| item.round == round) {
            let title = tasks.iter().find(|task| task.id == review.task_id).map(|task| if task.deliverable.trim().is_empty() { task.title.clone() } else { task.deliverable.clone() }).unwrap_or_else(|| review.task_id.clone());
            for criterion in review.criteria.iter().filter(|item| !item.met) {
                failed.push(format!("- {}: {}", title.chars().take(80).collect::<String>(), criterion.criterion.chars().take(200).collect::<String>()));
            }
        }
        if !failed.is_empty() { lines.push(format!("QA가 충족하지 않았다고 본 기준:\n{}", failed.into_iter().take(12).collect::<Vec<_>>().join("\n"))); }
    }
    lines.push("승인된 파일 안에서만 고치고, 계획 형식(업무별 수용 기준 1~8개, 하나당 300자 이하)을 지켜 주세요.".into());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CriterionResult;

    fn goal(block_kind: &str, resumed_from: &str) -> Goal {
        serde_json::from_value(serde_json::json!({ "id": "g", "kind": "execution", "projectId": "p", "intent": "README 링크 확인", "status": "blocked", "report": "Blocked: 산출물 'README'의 수용 기준은 8개 이하여야 합니다", "createdAt": "t", "resumedFrom": resumed_from, "blockKind": block_kind })).unwrap()
    }
    fn task(id: &str, status: &str) -> Task {
        serde_json::from_value(serde_json::json!({ "id": id, "goalId": "g", "title": format!("업무 {id}"), "instructions": "", "assigneeId": "dev", "status": status, "dependsOn": [], "files": ["README.md"], "worktree": null, "result": "", "createdAt": "t", "deliverable": "README §4 링크 확인" })).unwrap()
    }
    fn approved() -> Vec<Approval> {
        vec![Approval { id: "a".into(), goal_id: Some("g".into()), operation: "project_write".into(), reason: String::new(), status: "approved".into(), created_at: "t".into() }]
    }

    #[test]
    fn a_review_stage_format_block_is_repaired_on_the_same_goal() {
        // The CEO's 2026-10-14 case: QA failed, the PM's repair plan had too many criteria, kind `other`.
        assert_eq!(route(&goal("other", "validating"), &[task("t1", "completed")], &approved(), &[]), FixRoute::Repair);
        for kind in ["plan_form", "qa_failed", "qa_form"] {
            assert_eq!(route(&goal(kind, "validating"), &[task("t1", "completed")], &approved(), &[]), FixRoute::Repair, "{kind}");
        }
    }

    #[test]
    fn without_approved_scope_or_with_unfinished_work_it_becomes_a_new_version() {
        assert_eq!(route(&goal("other", "validating"), &[task("t1", "completed")], &[], &[]), FixRoute::Revision, "no approved scope");
        assert_eq!(route(&goal("other", "validating"), &[task("t1", "running")], &approved(), &[]), FixRoute::Revision, "a task may still hold changes");
        assert_eq!(route(&goal("integration_conflict", "executing"), &[task("t1", "awaiting_integration")], &approved(), &[]), FixRoute::Revision);
        assert_eq!(route(&goal("interrupted", "executing"), &[task("t1", "running")], &approved(), &[]), FixRoute::Revision);
        assert_eq!(route(&goal("other", "executing"), &[task("t1", "completed")], &approved(), &[]), FixRoute::Revision, "not stopped at review");
    }

    #[test]
    fn connection_problems_ceo_denials_and_open_guidance_are_refused() {
        assert!(matches!(route(&goal("provider", "executing"), &[], &[], &[]), FixRoute::Refused(_)));
        assert!(matches!(route(&goal("approval_denied", "awaiting_approval"), &[], &[], &[]), FixRoute::Refused(_)));
        let open: GoalGuidance = serde_json::from_value(serde_json::json!({ "id": "gd", "goalId": "g", "text": "방향 변경", "status": "review_required", "createdAt": "t" })).unwrap();
        assert!(matches!(route(&goal("other", "validating"), &[task("t1", "completed")], &approved(), &[open]), FixRoute::Refused(_)));
        let mut running = goal("other", "validating");
        running.status = "planning".into();
        assert!(matches!(route(&running, &[task("t1", "completed")], &approved(), &[]), FixRoute::Refused(_)), "a second click after the first started nothing new");
    }

    #[test]
    fn a_click_past_the_limit_adds_exactly_one_round() {
        assert_eq!(limit_after_click(2, 2), Some(3));
        assert_eq!(limit_after_click(1, 2), None);
    }

    #[test]
    fn the_brief_quotes_the_block_and_only_failed_criteria_of_the_latest_round() {
        let review = |round: u32, met: bool, text: &str| DeliverableReview { id: format!("r{round}{text}"), goal_id: "g".into(), task_id: "t1".into(), round, verdict: "fail".into(), criteria: vec![CriterionResult { criterion: text.into(), met, evidence: String::new() }], unverified: vec![], reviewer_id: "qa".into(), created_at: "t".into() };
        let text = brief(&goal("other", "validating"), &[review(0, false, "옛 기준"), review(1, false, "확인일 기록"), review(1, true, "링크 형식")], &[task("t1", "completed")]);
        assert!(text.starts_with("멈춘 이유: 산출물 'README'의 수용 기준은 8개 이하여야 합니다"));
        assert!(text.contains("README §4 링크 확인: 확인일 기록"));
        assert!(!text.contains("옛 기준") && !text.contains("링크 형식"));
        assert!(text.contains("승인된 파일 안에서만"));
    }
}
