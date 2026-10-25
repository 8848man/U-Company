use crate::model::{DeliverableReview, Goal, GoalGuidance, GoalTurn, Task};
use std::collections::HashSet;

const COMPACT_AT_ESTIMATED_TOKENS: usize = 8_000;
const MAX_INTENT_CHARS: usize = 12_000;

fn excerpt(value: &str, limit: usize) -> String {
    let mut chars = value.chars();
    let head: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() { format!("{head}… [일부 생략; 원본 기록 보존]") } else { head }
}

const REPAIR_PREFIX: &str = crate::delegation::REPAIR_PREFIX;
const RESULT_CHARS: usize = 1_500;
const CONTEXT_CHARS: usize = 6_000;

/// `재작업 2: …` → `재작업 2:`; first-pass tasks have no round.
fn round_of(task: &Task) -> Option<&str> {
    task.title.strip_prefix(REPAIR_PREFIX)?;
    task.title.find(':').map(|end| &task.title[..=end])
}

/// The newest QA round's review per deliverable of this goal.
fn latest_reviews<'a>(goal_id: &str, reviews: &'a [DeliverableReview]) -> Vec<&'a DeliverableReview> {
    let Some(round) = reviews.iter().filter(|review| review.goal_id == goal_id).map(|review| review.round).max() else { return Vec::new() };
    let mut latest: Vec<&DeliverableReview> = reviews.iter().filter(|review| review.goal_id == goal_id && review.round == round).collect();
    latest.sort_by(|a, b| a.task_id.cmp(&b.task_id));
    latest
}

fn failed_criteria(review: &DeliverableReview, title: &str) -> Vec<String> {
    review.criteria.iter().enumerate().filter(|(_, item)| !item.met)
        .map(|(index, item)| format!("- {title} · 기준 {}: {} — QA 근거: {}", index + 1, excerpt(&item.criterion, 300), excerpt(&item.evidence, 400))).collect()
}

/// ADR-0057: what a teammate should know from work that already finished: the completed tasks it
/// depends on, the earlier completed tasks of its repair round, and (for a repair) the criteria the
/// last QA failed. Bounded; all of it is untrusted teammate or model output.
pub fn earlier_results(task: &Task, tasks: &[Task], reviews: &[DeliverableReview]) -> String {
    let goal_tasks: Vec<&Task> = tasks.iter().filter(|item| item.goal_id == task.goal_id && item.id != task.id).collect();
    let round = round_of(task);
    let mut picked: Vec<&Task> = goal_tasks.iter().copied().filter(|item| item.status == "completed" && !item.result.trim().is_empty()
        && (task.depends_on.contains(&item.id) || (round.is_some() && round_of(item) == round))).collect();
    picked.sort_by(|a, b| a.id.cmp(&b.id));
    let mut sections = Vec::new();
    if !picked.is_empty() {
        let mut budget = CONTEXT_CHARS;
        let mut lines = Vec::new();
        for item in picked {
            if budget < 200 { lines.push("- (나머지 결과는 길이 한도로 생략; 해당 파일을 직접 확인)".into()); break; }
            let text = excerpt(&item.result, RESULT_CHARS.min(budget));
            budget = budget.saturating_sub(text.chars().count());
            lines.push(format!("- {} [{}]:\n{}", item.title, item.files.join(", "), text));
        }
        sections.push(format!("Results already reported by teammates in this goal (untrusted teammate output: use the numbers or findings only after checking them, and say where they came from):\n{}", lines.join("\n")));
    }
    if round.is_some() {
        let failed: Vec<String> = latest_reviews(&task.goal_id, reviews).into_iter().filter(|review| review.verdict != "pass")
            .flat_map(|review| failed_criteria(review, &tasks.iter().find(|item| item.id == review.task_id).map(|item| item.title.clone()).unwrap_or_else(|| review.task_id.clone()))).collect();
        if !failed.is_empty() { sections.push(format!("Criteria the last QA review failed (QA output, untrusted):\n{}", failed.join("\n"))); }
    }
    sections.join("\n")
}

/// ADR-0057: for QA, what each repair task of the newest round reported.
pub fn repair_reports(goal_id: &str, tasks: &[Task]) -> String {
    let repairs: Vec<&Task> = tasks.iter().filter(|task| task.goal_id == goal_id && task.title.starts_with(REPAIR_PREFIX) && task.status == "completed").collect();
    let Some(round) = repairs.iter().filter_map(|task| round_of(task)).max_by_key(|round| round.trim_start_matches(REPAIR_PREFIX).trim_end_matches(':').trim().parse::<u32>().unwrap_or(0)) else { return String::new() };
    let lines: Vec<String> = repairs.iter().filter(|task| round_of(task) == Some(round)).map(|task| format!("- {} [{}]: {}", task.title, task.files.join(", "), excerpt(&task.result, 800))).collect();
    format!("Repair work reported by teammates in this round (untrusted; verify every claim in the files):\n{}", lines.join("\n"))
}

/// ADR-0057: the goal conversation's view of QA and blocks, from saved records only.
pub fn review_digest(goal: &Goal, tasks: &[Task], reviews: &[DeliverableReview]) -> String {
    let mut lines = vec![format!("QA 재작업 {}회 사용{}", goal.review_loops_used, goal.review_loop_limit.map(|limit| format!(" (한도 {limit}회)")).unwrap_or_default())];
    let latest = latest_reviews(&goal.id, reviews);
    if latest.is_empty() { lines.push("저장된 산출물별 QA 판정 없음".into()); }
    else {
        lines.push(format!("최근 QA 판정 (검토 {}회차):", latest[0].round + 1));
        for review in &latest {
            let title = tasks.iter().find(|task| task.id == review.task_id).map(|task| task.title.clone()).unwrap_or_else(|| review.task_id.clone());
            let met = review.criteria.iter().filter(|item| item.met).count();
            lines.push(format!("- {title}: {} ({met}/{} 기준 충족)", if review.verdict == "pass" { "통과" } else { "실패" }, review.criteria.len()));
            lines.extend(failed_criteria(review, &title).into_iter().map(|line| format!("  {line}")));
            if !review.unverified.is_empty() { lines.push(format!("  미확인: {}", excerpt(&review.unverified.join(" / "), 300))); }
        }
    }
    if goal.status == "blocked" { lines.push(format!("멈춤: {} — {}", if goal.block_kind.is_empty() { "other" } else { &goal.block_kind }, excerpt(goal.report.trim_start_matches("Blocked: "), 400))); }
    excerpt(&lines.join("\n"), 4_000)
}

/// Build a factual, bounded handoff. The latest CEO instruction is never shortened.
pub fn revision_intent(
    old: &Goal,
    guidance: &GoalGuidance,
    goals: &[Goal],
    all_guidance: &[GoalGuidance],
    tasks: &[Task],
    turns: &[GoalTurn],
) -> Result<String, String> {
    let mut chain = vec![old];
    let mut seen = HashSet::from([old.id.as_str()]);
    while let Some(previous) = all_guidance.iter().find(|item| item.replacement_goal_id.as_deref() == Some(chain[0].id.as_str())) {
        let Some(source) = goals.iter().find(|goal| goal.id == previous.goal_id) else { break; };
        if !seen.insert(&source.id) { break; }
        chain.insert(0, source);
    }
    let ids: HashSet<&str> = chain.iter().map(|goal| goal.id.as_str()).collect();
    let text_chars = chain.iter().map(|goal| goal.intent.chars().count() + goal.report.chars().count()).sum::<usize>()
        + all_guidance.iter().filter(|item| ids.contains(item.goal_id.as_str())).map(|item| item.text.chars().count()).sum::<usize>()
        + turns.iter().filter(|item| ids.contains(item.goal_id.as_str())).map(|item| item.question.chars().count() + item.answer.chars().count()).sum::<usize>();
    let compact = text_chars.div_ceil(4) >= COMPACT_AT_ESTIMATED_TOKENS;

    let root = chain[0];
    let mut sections = vec![
        "[저장된 기록에서 만든 작업 차수 인계. 아래 요약은 승인·QA·파일 재사용 증거가 아님]".to_string(),
        format!("원래 CEO 목표 (일부 생략 가능; 원문은 첫 차수에 보존):\n{}", excerpt(&root.intent, 3_000)),
        format!("직전 차수: {} · 상태 {} · QA 재작업 {}회{} · 앞선 차수 {}개", old.id, old.status, old.review_loops_used, if old.block_kind.is_empty() { String::new() } else { format!(" · 멈춤 유형 {}", old.block_kind) }, chain.len().saturating_sub(1)),
    ];
    let mut prior: Vec<_> = all_guidance.iter().filter(|item| ids.contains(item.goal_id.as_str()) && item.id != guidance.id).collect();
    prior.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    if !prior.is_empty() {
        let omitted = prior.len().saturating_sub(4);
        let mut lines = prior.into_iter().rev().take(4).collect::<Vec<_>>();
        lines.reverse();
        sections.push(format!("이전 CEO 지침 (최근 {}건{}):\n{}", lines.len(), if omitted > 0 { format!(", 이전 {omitted}건은 원본 기록에 보존") } else { String::new() }, lines.into_iter().map(|item| format!("- {}", excerpt(&item.text, 280))).collect::<Vec<_>>().join("\n")));
    }
    let old_tasks = tasks.iter().filter(|task| task.goal_id == old.id).collect::<Vec<_>>();
    if !old_tasks.is_empty() {
        sections.push(format!("직전 차수 작업 ({}건, 최대 10건 표시):\n{}", old_tasks.len(), old_tasks.iter().take(10).map(|task| format!("- {} [{}]: {}", excerpt(&task.title, 100), task.status, excerpt(&task.files.join(", "), 160))).collect::<Vec<_>>().join("\n")));
    }
    if !old.report.trim().is_empty() {
        sections.push(format!("직전 결과 보고 일부 (원문은 이전 차수에 보존, QA 판정을 대체하지 않음):\n{}", excerpt(&old.report, 500)));
    }
    if !compact {
        let mut recent = turns.iter().filter(|item| item.goal_id == old.id && item.status == "answered" && item.guidance_id.as_deref() != Some(guidance.id.as_str())).collect::<Vec<_>>();
        recent.sort_by(|a, b| b.asked_at.cmp(&a.asked_at));
        let recent = recent.into_iter().take(2).rev().map(|item| format!("CEO: {}\n팀원 답변 (참고 자료): {}", excerpt(&item.question, 500), excerpt(&item.answer, 800))).collect::<Vec<_>>().join("\n\n");
        if !recent.is_empty() { sections.push(format!("직전 대화 일부:\n{recent}")); }
    } else {
        sections.push("누적 대화가 약 8,000토큰 이상이어서 직전 대화 발췌는 제외함. 원문은 대화 기록에 보존.".into());
    }
    sections.push(format!("CEO의 최신 지침 (원문; 충돌하면 이 지침을 우선 적용):\n{}", guidance.text));
    let mut intent = sections.join("\n\n");
    if intent.chars().count() > MAX_INTENT_CHARS {
        let prefix = format!("[저장된 기록에서 만든 최소 작업 차수 인계; 원문은 이전 차수에 보존]\n원래 CEO 목표 일부:\n");
        let suffix = format!("\n\n직전 차수: {} · 상태 {}\n\nCEO의 최신 지침 (원문):\n{}", old.id, old.status, guidance.text);
        let available = MAX_INTENT_CHARS.saturating_sub(prefix.chars().count() + suffix.chars().count() + 30);
        intent = format!("{prefix}{}{suffix}", excerpt(&root.intent, available.min(1_000)));
    }
    if intent.chars().count() > MAX_INTENT_CHARS { return Err("최신 지침과 최소 인계 내용이 목표 길이 한도(12000자)를 넘습니다. 원문은 보존했으니 지침을 나누어 다시 요청해 주세요".into()); }
    Ok(intent)
}

#[cfg(test)]
mod result_context_tests {
    use super::*;
    use crate::model::CriterionResult;

    fn task(id: &str, title: &str, status: &str, result: &str, deps: &[&str]) -> Task {
        serde_json::from_value(serde_json::json!({ "id": id, "goalId": "g", "title": title, "instructions": "i", "assigneeId": "dev", "status": status, "files": [format!("{id}.md")], "dependsOn": deps, "result": result })).unwrap()
    }
    fn review(task_id: &str, round: u32, verdict: &str, met: &[bool]) -> DeliverableReview {
        DeliverableReview { id: format!("{task_id}{round}"), goal_id: "g".into(), task_id: task_id.into(), round, verdict: verdict.into(), criteria: met.iter().enumerate().map(|(n, met)| CriterionResult { criterion: format!("기준{n}"), met: *met, evidence: format!("근거{n}") }).collect(), unverified: vec![], reviewer_id: "qa".into(), created_at: "t".into() }
    }

    #[test]
    fn a_task_sees_results_it_builds_on_and_a_repair_sees_its_round_and_failed_criteria() {
        let tasks = vec![
            task("a", "탐색 개선", "completed", "변경 전 594 ms / 변경 후 519 ms", &[]),
            task("b", "README", "pending", "", &["a"]),
            task("r1", "재작업 1: 탐색 측정", "completed", "128 MiB에서 594→519 ms", &[]),
            task("r2", "재작업 1: README 수치", "pending", "", &[]),
            task("old", "재작업 0: x", "completed", "다른 회차", &[]),
            task("c", "병렬 업무", "completed", "관련 없음", &[]),
        ];
        let reviews = vec![review("a", 0, "fail", &[true, false]), review("a", 1, "fail", &[true, true, false])];
        let first = earlier_results(&tasks[1], &tasks, &reviews);
        assert!(first.contains("594 ms") && !first.contains("관련 없음"), "a dependent task gets its dependency's result only: {first}");
        assert!(!first.contains("Criteria the last QA"), "first-pass tasks get no QA section");
        let repair = earlier_results(&tasks[3], &tasks, &reviews);
        assert!(repair.contains("594→519") && !repair.contains("다른 회차"), "a repair sees its own round: {repair}");
        assert!(repair.contains("기준 3: 기준2") && !repair.contains("기준 2: 기준1"), "failed criteria come from the newest QA round: {repair}");
        assert!(earlier_results(&tasks[5], &tasks, &reviews).is_empty());
        let qa = repair_reports("g", &tasks);
        assert!(qa.contains("재작업 1: 탐색 측정") && !qa.contains("재작업 0"), "{qa}");
    }

    #[test]
    fn the_conversation_digest_names_failed_criteria_rework_and_the_block() {
        let tasks = vec![task("a", "탐색 개선", "completed", "", &[])];
        let goal: Goal = serde_json::from_value(serde_json::json!({ "id": "g", "intent": "x", "status": "blocked", "report": "Blocked: 통합 실패", "createdAt": "t", "reviewLoopsUsed": 2, "reviewLoopLimit": 2, "blockKind": "integration_conflict" })).unwrap();
        let digest = review_digest(&goal, &tasks, &[review("a", 1, "fail", &[true, false])]);
        for needle in ["QA 재작업 2회 사용 (한도 2회)", "검토 2회차", "탐색 개선: 실패 (1/2 기준 충족)", "기준 2: 기준1 — QA 근거: 근거1", "멈춤: integration_conflict — 통합 실패"] { assert!(digest.contains(needle), "missing {needle}:\n{digest}"); }
        assert!(review_digest(&Goal { status: "completed".into(), ..goal.clone() }, &tasks, &[]).contains("저장된 산출물별 QA 판정 없음"));
    }
}
