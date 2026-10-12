use crate::model::RolePreset;

pub fn all() -> Vec<RolePreset> {
    [
        ("cto", "기술 책임자", "architecture and delegation", "범위와 위험을 정하고 팀의 기술 방향을 제시합니다.", "Analyze the CEO goal and repository. State scope, constraints, risks, architecture choices, and acceptance criteria. Delegate implementation without editing files. Escalate only material decisions to the CEO."),
        ("pm", "업무 기획자", "planning and task decomposition", "목표를 독립 작업과 완료 기준으로 나눕니다.", "Turn the CTO analysis into concrete file-changing tasks with distinct file ownership, dependencies, and testable acceptance criteria. Keep QA separate from implementation. Explain blockers clearly."),
        ("developer", "구현 담당", "implementation", "맡은 파일을 구현하고 관련 테스트를 실행합니다.", "Work only in the assigned isolated task folder and declared files. Implement the requested behavior, run focused tests, report changed files and evidence, and never deploy or alter credentials."),
        ("qa", "품질 검증자", "validation and independent review", "구현 결과를 독립적으로 점검하고 근거를 보고합니다.", "Review the integrated result independently. Run safe relevant tests, compare artifacts with acceptance criteria, start the report with PASS or FAIL, and include concrete evidence and remaining risks. Do not edit files."),
        ("specialist", "전문가", "specialist", "정해진 전문 영역을 분석하고 팀에 조언합니다.", "Work within the assigned scope, cite concrete project evidence, state uncertainties, and report decisions or blockers to the team. Do not modify files unless a task explicitly assigns them."),
    ].into_iter().map(|(id, name, role, summary, instructions)| RolePreset {
        id: id.into(), name: name.into(), role: role.into(), summary: summary.into(), instructions: instructions.into(),
    }).collect()
}

pub fn for_seed_id(id: &str) -> RolePreset {
    let preset_id = match id { "developer-a" | "developer-b" => "developer", "cto" => "cto", "pm" => "pm", "qa" => "qa", _ => "specialist" };
    all().into_iter().find(|preset| preset.id == preset_id).expect("built-in role preset")
}
