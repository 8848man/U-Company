use crate::model::{Goal, GoalGuidance, GoalTurn, Task};
use std::collections::HashSet;

const COMPACT_AT_ESTIMATED_TOKENS: usize = 8_000;
const MAX_INTENT_CHARS: usize = 12_000;

fn excerpt(value: &str, limit: usize) -> String {
    let mut chars = value.chars();
    let head: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() { format!("{head}… [일부 생략; 원본 기록 보존]") } else { head }
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
