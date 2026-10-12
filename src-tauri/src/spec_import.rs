use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

const MAX_IMPORT_BYTES: usize = 512 * 1024;
pub const PORTABLE_PATH: &str = ".ucompany/spec.json";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableStatus {
    pub state: String,
    pub path: String,
    pub detail: String,
}

fn project_root(project: &crate::model::Project) -> Result<PathBuf, String> {
    let root = Path::new(&project.path).canonicalize().map_err(|e| format!("프로젝트 폴더를 열 수 없습니다: {e}"))?;
    if !root.is_dir() || !root.to_string_lossy().eq_ignore_ascii_case(&project.path) {
        return Err("등록된 프로젝트 경로가 변경되었습니다. 프로젝트를 다시 확인해 주세요".into());
    }
    Ok(root)
}

fn portable_location(project: &crate::model::Project) -> Result<PathBuf, String> {
    let root = project_root(project)?;
    let parent = root.join(".ucompany");
    if let Ok(metadata) = std::fs::symlink_metadata(&parent) {
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(".ucompany는 일반 폴더여야 합니다".into());
        }
        if !parent.canonicalize().map_err(|e| e.to_string())?.starts_with(&root) {
            return Err("명세 폴더가 프로젝트 밖을 가리킵니다".into());
        }
    }
    let file = parent.join("spec.json");
    if let Ok(metadata) = std::fs::symlink_metadata(&file) {
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("spec.json은 일반 파일이어야 합니다".into());
        }
    }
    Ok(file)
}

fn read_portable(project: &crate::model::Project) -> Result<Option<String>, String> {
    let file = portable_location(project)?;
    if !file.exists() { return Ok(None); }
    let metadata = std::fs::metadata(&file).map_err(|e| e.to_string())?;
    if metadata.len() > MAX_IMPORT_BYTES as u64 { return Err("프로젝트 명세 파일은 512 KiB 이하여야 합니다".into()); }
    let content = std::fs::read_to_string(file).map_err(|e| format!("프로젝트 명세 파일을 읽지 못했습니다: {e}"))?;
    if content.len() > MAX_IMPORT_BYTES { return Err("프로젝트 명세 파일은 512 KiB 이하여야 합니다".into()); }
    Ok(Some(content))
}

fn portable_manifest(profile: &SpecProfile) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": 1,
        "project": { "summary": profile.project_summary },
        "agents": profile.agents.iter().map(|agent| serde_json::json!({ "key": agent.key, "role": agent.role, "instructions": agent.instructions })).collect::<Vec<_>>(),
        "workflow": profile.workflow.iter().map(|step| serde_json::json!({ "title": step.title, "description": step.description })).collect::<Vec<_>>(),
        "exceptions": profile.exceptions.iter().map(|item| serde_json::json!({ "title": item.title, "content": item.content, "scope": item.scope })).collect::<Vec<_>>(),
    })
}

pub fn portable_status(project: &crate::model::Project, active: Option<&SpecProfile>) -> PortableStatus {
    let (state, detail) = match read_portable(project) {
        Ok(None) => ("absent", "공통 명세 파일이 없습니다".to_string()),
        Ok(Some(content)) => match preview(&project.id, active.map_or(0, |item| item.revision), vec![SpecSource { name: PORTABLE_PATH.into(), content }]) {
            Ok(draft) if active.is_some_and(|item| portable_manifest(item) == portable_manifest(&draft.profile)) => ("current", "프로젝트 파일과 적용된 명세가 같습니다".to_string()),
            Ok(_) => ("changed", "프로젝트 파일과 앱의 적용된 명세가 다릅니다".to_string()),
            Err(error) => ("invalid", error),
        },
        Err(error) => ("invalid", error),
    };
    PortableStatus { state: state.into(), path: PORTABLE_PATH.into(), detail }
}

pub fn preview_portable(project: &crate::model::Project, revision: u32) -> Result<SpecDraft, String> {
    let content = read_portable(project)?.ok_or("프로젝트에 공통 명세 파일이 없습니다")?;
    let mut draft = preview(&project.id, revision, vec![SpecSource { name: PORTABLE_PATH.into(), content }])?;
    draft.portable_source = true;
    Ok(draft)
}

pub fn verify_portable_draft(project: &crate::model::Project, draft: &SpecDraft) -> Result<(), String> {
    if !draft.portable_source || draft.project_id != project.id {
        return Err("프로젝트 파일을 미리본 초안만 적용할 수 있습니다. 가져온 명세를 프로젝트 파일에 저장하거나 병합한 뒤 다시 미리보세요".into());
    }
    let current = preview_portable(project, draft.base_revision)?;
    if portable_manifest(&current.profile) != portable_manifest(&draft.profile) {
        return Err("프로젝트 명세 파일이 미리보기 이후 변경되었습니다. 다시 미리보세요".into());
    }
    Ok(())
}

pub fn create_portable(project: &crate::model::Project, profile: &SpecProfile) -> Result<PortableStatus, String> {
    if profile.project_id != project.id { return Err("선택한 프로젝트의 명세가 아닙니다".into()); }
    validate_profile(profile)?;
    let file = portable_location(project)?;
    let parent = file.parent().ok_or("명세 폴더를 확인할 수 없습니다")?;
    if !parent.exists() { std::fs::create_dir(parent).map_err(|e| format!("명세 폴더를 만들지 못했습니다: {e}"))?; }
    let file = portable_location(project)?;
    let content = serde_json::to_vec_pretty(&portable_manifest(profile)).map_err(|e| e.to_string())?;
    if content.len() > MAX_IMPORT_BYTES { return Err("내보낼 명세가 512 KiB를 넘습니다".into()); }
    let mut handle = std::fs::OpenOptions::new().write(true).create_new(true).open(&file)
        .map_err(|e| format!("명세 파일이 이미 있거나 만들 수 없습니다: {e}. 기존 파일은 덮어쓰지 않습니다"))?;
    if let Err(error) = handle.write_all(&content).and_then(|_| handle.sync_all()) {
        drop(handle);
        let _ = std::fs::remove_file(&file);
        return Err(format!("명세 파일 저장 실패: {error}"));
    }
    Ok(portable_status(project, Some(profile)))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecSource {
    pub name: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSpec {
    pub key: String,
    pub role: String,
    pub instructions: String,
    #[serde(default)]
    pub bound_agent_id: Option<String>,
    #[serde(default)]
    pub source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowSpec {
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionSpec {
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecProfile {
    pub project_id: String,
    pub revision: u32,
    pub imported_at: String,
    pub project_summary: String,
    pub agents: Vec<AgentSpec>,
    pub workflow: Vec<WorkflowSpec>,
    pub exceptions: Vec<ExceptionSpec>,
    pub sources: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecDraft {
    pub id: String,
    pub project_id: String,
    pub base_revision: u32,
    pub created_at: String,
    pub profile: SpecProfile,
    pub warnings: Vec<String>,
    #[serde(default)]
    pub portable_source: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    schema_version: u32,
    #[serde(default)]
    project: ManifestProject,
    #[serde(default)]
    agents: Vec<AgentSpec>,
    #[serde(default)]
    workflow: Vec<WorkflowSpec>,
    #[serde(default)]
    exceptions: Vec<ExceptionSpec>,
}

#[derive(Default, Deserialize)]
struct ManifestProject {
    #[serde(default)]
    summary: String,
}

fn check_text(label: &str, value: &str, max: usize) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > max || value.contains('\0') {
        return Err(format!("{label}은(는) 비어 있지 않은 {max}바이트 이하 텍스트여야 합니다"));
    }
    Ok(())
}

fn contains_obvious_secret(content: &str) -> bool {
    let lowered = content.to_ascii_lowercase();
    if lowered.contains("-----begin private key-----") || lowered.contains("authorization: bearer ") { return true; }
    content.split(|ch: char| ch.is_whitespace() || ['\'', '"', '=', ':', ',', ';'].contains(&ch)).any(|part| {
        let token = part.trim();
        ["sk-ant-", "sk-proj-", "ghp_", "github_pat_", "xoxb-", "xoxp-"].iter().any(|prefix| token.starts_with(prefix) && token.len() > prefix.len() + 8)
    })
}

fn role_key(heading: &str) -> Option<String> {
    let lowered = heading.trim().to_ascii_lowercase();
    let key = lowered.strip_prefix("agent:").or_else(|| lowered.strip_prefix("role:"))
        .or_else(|| lowered.strip_prefix("팀원:")).unwrap_or(&lowered).trim();
    match key {
        "cto" | "pm" | "qa" | "developer" | "developer-a" | "developer-b" | "specialist" => Some(key.into()),
        _ if lowered.starts_with("agent:") || lowered.starts_with("role:") || lowered.starts_with("팀원:") => Some(key.into()),
        _ => None,
    }
}

fn add_markdown_section(profile: &mut SpecProfile, source: &str, title: &str, body: &str) -> Result<(), String> {
    let content = body.trim();
    if content.is_empty() { return Ok(()); }
    check_text(title, content, 64 * 1024)?;
    let heading = title.to_ascii_lowercase();
    if let Some(key) = role_key(title) {
        check_text("팀원 지침", content, 8 * 1024)?;
        profile.agents.push(AgentSpec { key, role: title.into(), instructions: content.into(), bound_agent_id: None, source: source.into() });
    } else if ["project", "프로젝트", "overview", "개요", "requirements", "요구사항"].iter().any(|label| heading.contains(label)) {
        if !profile.project_summary.is_empty() { profile.project_summary.push_str("\n\n"); }
        profile.project_summary.push_str(&format!("[{source} · {title}]\n{content}"));
    } else if ["workflow", "워크플로우", "process", "업무 흐름"].iter().any(|label| heading.contains(label)) {
        profile.workflow.push(WorkflowSpec { title: title.into(), description: content.into(), source: source.into() });
    } else {
        profile.exceptions.push(ExceptionSpec { title: title.into(), content: content.into(), scope: "project_reference".into(), source: source.into() });
    }
    Ok(())
}

fn parse_markdown(profile: &mut SpecProfile, source: &SpecSource) -> Result<(), String> {
    let name = source.name.rsplit(['/', '\\']).next().unwrap_or(&source.name);
    if ["agents.md", "claude.md"].contains(&name.to_ascii_lowercase().as_str()) {
        profile.exceptions.push(ExceptionSpec { title: name.into(), content: source.content.clone(), scope: "harness_reference".into(), source: source.name.clone() });
        return Ok(());
    }
    let mut title = "문서 머리말".to_string();
    let mut body = String::new();
    let mut fence: Option<&str> = None;
    for line in source.content.lines() {
        let trimmed = line.trim_start();
        if let Some(marker) = fence {
            body.push_str(line);
            body.push('\n');
            if trimmed.starts_with(marker) { fence = None; }
            continue;
        }
        if trimmed.starts_with("```") { fence = Some("```"); }
        else if trimmed.starts_with("~~~") { fence = Some("~~~"); }
        if fence.is_none() {
        if let Some(heading) = line.strip_prefix('#').map(|part| part.trim_start_matches('#').trim()).filter(|part| !part.is_empty()) {
            add_markdown_section(profile, &source.name, &title, &body)?;
            title = heading.to_string();
            body.clear();
            continue;
        }
        }
        body.push_str(line);
        body.push('\n');
    }
    add_markdown_section(profile, &source.name, &title, &body)
}

fn parse_manifest(profile: &mut SpecProfile, source: &SpecSource) -> Result<(), String> {
    let value: serde_json::Value = serde_json::from_str(&source.content).map_err(|e| format!("{} JSON 파싱 실패: {e}", source.name))?;
    let manifest: Manifest = serde_json::from_value(value.clone()).map_err(|e| format!("{} 명세 형식 오류: {e}", source.name))?;
    if manifest.schema_version != 1 { return Err("지원하지 않는 명세 버전입니다. schemaVersion: 1을 사용하세요".into()); }
    if !manifest.project.summary.trim().is_empty() {
        if !profile.project_summary.is_empty() { profile.project_summary.push_str("\n\n"); }
        profile.project_summary.push_str(&manifest.project.summary);
    }
    for mut agent in manifest.agents { agent.source = source.name.clone(); agent.bound_agent_id = None; profile.agents.push(agent); }
    for mut step in manifest.workflow { step.source = source.name.clone(); profile.workflow.push(step); }
    for mut exception in manifest.exceptions { exception.source = source.name.clone(); profile.exceptions.push(exception); }
    if let Some(object) = value.as_object() {
        for (key, value) in object {
            if ["schemaVersion", "project", "agents", "workflow", "exceptions"].contains(&key.as_str()) { continue; }
            profile.exceptions.push(ExceptionSpec { title: format!("미분류 JSON 필드: {key}"), content: value.to_string(), scope: "project_reference".into(), source: source.name.clone() });
        }
    }
    let nested = [
        ("project", vec!["summary"]),
        ("agents", vec!["key", "role", "instructions", "boundAgentId", "source"]),
        ("workflow", vec!["title", "description", "source"]),
        ("exceptions", vec!["title", "content", "scope", "source"]),
    ];
    for (section, known) in nested {
        let rows: Vec<&serde_json::Value> = match value.get(section) {
            Some(serde_json::Value::Array(items)) => items.iter().collect(),
            Some(item) => vec![item],
            None => Vec::new(),
        };
        for (index, row) in rows.into_iter().enumerate() {
            if let Some(object) = row.as_object() {
                for (key, value) in object {
                    if known.contains(&key.as_str()) { continue; }
                    profile.exceptions.push(ExceptionSpec { title: format!("미분류 JSON 필드: {section}[{index}].{key}"), content: value.to_string(), scope: "project_reference".into(), source: source.name.clone() });
                }
            }
        }
    }
    Ok(())
}

pub fn preview(project_id: &str, base_revision: u32, sources: Vec<SpecSource>) -> Result<SpecDraft, String> {
    if sources.is_empty() || sources.len() > 20 { return Err("명세 파일을 1~20개 선택해 주세요".into()); }
    if sources.iter().map(|source| source.content.len()).sum::<usize>() > MAX_IMPORT_BYTES { return Err("명세 전체 크기는 512 KiB 이하여야 합니다".into()); }
    let mut profile = SpecProfile { project_id: project_id.into(), revision: 0, imported_at: String::new(), project_summary: String::new(), agents: Vec::new(), workflow: Vec::new(), exceptions: Vec::new(), sources: Vec::new() };
    for source in sources {
        check_text("파일 이름", &source.name, 240)?;
        check_text("명세 내용", &source.content, MAX_IMPORT_BYTES)?;
        if contains_obvious_secret(&source.content) { return Err(format!("{}에 자격 증명으로 보이는 값이 있습니다. 비밀 값을 제거한 사본을 가져와 주세요", source.name)); }
        profile.sources.push(source.name.clone());
        if source.name.to_ascii_lowercase().ends_with(".json") || source.content.trim_start().starts_with('{') {
            parse_manifest(&mut profile, &source)?;
        } else { parse_markdown(&mut profile, &source)?; }
    }
    validate_profile(&profile)?;
    let mut warnings: Vec<String> = profile.agents.iter().map(|agent| format!("{} 역할은 기존 팀원 연결 전까지 placeholder입니다", agent.key)).collect();
    if !profile.exceptions.is_empty() { warnings.push(format!("자동 분류하지 못한 내용 {}개를 출처와 함께 예외 명세로 보존했습니다", profile.exceptions.len())); }
    Ok(SpecDraft { id: crate::model::id(), project_id: project_id.into(), base_revision, created_at: crate::model::now(), profile, warnings, portable_source: false })
}

pub fn validate_profile(profile: &SpecProfile) -> Result<(), String> {
    if profile.agents.len() > 50 || profile.workflow.len() > 50 || profile.exceptions.len() > 100 { return Err("명세 항목이 한도를 넘었습니다 (팀원 50, 흐름 50, 예외 100)".into()); }
    if profile.project_summary.len() > 64 * 1024 { return Err("프로젝트 명세는 64 KiB 이하여야 합니다".into()); }
    for agent in &profile.agents {
        check_text("팀원 키", &agent.key, 80)?;
        check_text("팀원 역할", &agent.role, 120)?;
        check_text("팀원 지침", &agent.instructions, 8 * 1024)?;
    }
    let mut keys = std::collections::HashSet::new();
    for agent in &profile.agents {
        if !keys.insert(agent.key.to_ascii_lowercase()) { return Err(format!("팀원 키가 중복되었습니다: {}", agent.key)); }
    }
    for step in &profile.workflow { check_text("업무 단계", &step.title, 160)?; check_text("업무 설명", &step.description, 64 * 1024)?; }
    for exception in &profile.exceptions { check_text("예외 제목", &exception.title, 160)?; check_text("예외 내용", &exception.content, 64 * 1024)?; }
    if profile.project_summary.is_empty() && profile.agents.is_empty() && profile.workflow.is_empty() && profile.exceptions.is_empty() { return Err("가져올 명세 내용이 없습니다".into()); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_file_is_created_once_and_external_edit_is_detected() {
        let root = std::env::temp_dir().join(format!("utopia-portable-spec-{}", crate::model::id()));
        std::fs::create_dir(&root).unwrap();
        let canonical = root.canonicalize().unwrap();
        let project = crate::model::Project {
            id: "portable-project".into(), path: canonical.to_string_lossy().into_owned(),
            name: "Portable".into(), kind: "folder".into(), review_loop_limit: 2,
        };
        let profile = preview(&project.id, 0, vec![SpecSource {
            name: "team.json".into(),
            content: r#"{"schemaVersion":1,"project":{"summary":"Initial"},"agents":[{"key":"qa","role":"QA","instructions":"Review"}]}"#.into(),
        }]).unwrap().profile;
        assert_eq!(portable_status(&project, Some(&profile)).state, "absent");
        assert_eq!(create_portable(&project, &profile).unwrap().state, "current");
        assert!(create_portable(&project, &profile).is_err());
        let before_edit = preview_portable(&project, 0).unwrap();
        assert!(verify_portable_draft(&project, &before_edit).is_ok());
        let file = canonical.join(PORTABLE_PATH);
        let changed = std::fs::read_to_string(&file).unwrap().replace("Initial", "Externally edited");
        std::fs::write(&file, changed).unwrap();
        assert_eq!(portable_status(&project, Some(&profile)).state, "changed");
        assert!(verify_portable_draft(&project, &before_edit).is_err());
        assert_eq!(preview_portable(&project, 1).unwrap().profile.project_summary, "Externally edited");
        let resolved = root.canonicalize().unwrap();
        assert!(resolved.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn markdown_preserves_unmapped_and_harness_sections() {
        let draft = preview("p", 0, vec![
            SpecSource { name: "SDD.md".into(), content: "## Project\nLocal app\n## Agent: developer\nImplement safely\n## Workflow\nReview then build\n## Legacy rule\nKeep this".into() },
            SpecSource { name: "AGENTS.md".into(), content: "Existing harness instructions".into() },
        ]).unwrap();
        assert_eq!(draft.profile.agents.len(), 1);
        assert_eq!(draft.profile.workflow.len(), 1);
        assert_eq!(draft.profile.exceptions.len(), 2);
        assert_eq!(draft.profile.exceptions[1].scope, "harness_reference");
    }

    #[test]
    fn json_unknown_fields_are_exceptions_and_oversize_rejected() {
        let draft = preview("p", 0, vec![SpecSource { name: "team.json".into(), content: r#"{"schemaVersion":1,"project":{"summary":"P"},"agents":[{"key":"qa","role":"review","instructions":"Check"}],"workflow":[],"legacyHarness":{"tool":"custom"}}"#.into() }]).unwrap();
        assert_eq!(draft.profile.exceptions[0].title, "미분류 JSON 필드: legacyHarness");
        assert!(preview("p", 0, vec![SpecSource { name: "x.md".into(), content: "x".repeat(MAX_IMPORT_BYTES + 1) }]).is_err());
        assert!(preview("p", 0, vec![SpecSource { name: "secret.md".into(), content: "## Project\nsk-ant-thislookssecret123456".into() }]).is_err());
    }

    #[test]
    fn markdown_code_fence_does_not_create_fake_roles() {
        let draft = preview("p", 0, vec![SpecSource { name: "SDD.md".into(), content: "## Workflow\nExample:\n```md\n## Agent: cto\nNot a real role\n```".into() }]).unwrap();
        assert!(draft.profile.agents.is_empty());
        assert!(draft.profile.workflow[0].description.contains("## Agent: cto"));
    }

    #[test]
    fn staging_replaces_only_same_project_draft() {
        use crate::storage::Store;
        let dir = std::env::temp_dir().join(format!("utopia-spec-stage-{}", crate::model::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("state.sqlite")).unwrap();
        let source = || vec![SpecSource { name: "x.md".into(), content: "## Project\nExample".into() }];
        let first = preview("a", 0, source()).unwrap();
        let second = preview("a", 0, source()).unwrap();
        let other = preview("b", 0, source()).unwrap();
        store.stage_spec_import(&first).unwrap();
        store.stage_spec_import(&other).unwrap();
        store.stage_spec_import(&second).unwrap();
        assert!(store.get::<SpecDraft>("spec_draft", &first.id).unwrap().is_none());
        assert!(store.get::<SpecDraft>("spec_draft", &second.id).unwrap().is_some());
        assert!(store.get::<SpecDraft>("spec_draft", &other.id).unwrap().is_some());
        drop(store);
        let resolved = dir.canonicalize().unwrap();
        assert!(resolved.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        std::fs::remove_dir_all(resolved).unwrap();
    }

    #[test]
    fn project_profiles_apply_atomically_without_changing_global_agents() {
        use crate::model::{Agent, Project};
        use crate::storage::{SpecAgentBinding, Store};
        let dir = std::env::temp_dir().join(format!("utopia-spec-import-{}", crate::model::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir(dir.join("a")).unwrap();
        std::fs::create_dir(dir.join("b")).unwrap();
        let path = dir.join("state.sqlite");
        {
            let store = Store::open(&path).unwrap();
            let project = Project { id: "project-a".into(), path: dir.join("a").canonicalize().unwrap().to_string_lossy().into_owned(), name: "A".into(), kind: "folder".into(), review_loop_limit: 2 };
            store.select_project(&project).unwrap();
            let other = Project { id: "project-b".into(), path: dir.join("b").canonicalize().unwrap().to_string_lossy().into_owned(), name: "B".into(), ..project.clone() };
            store.put("project_entry", &other.id, &other, None).unwrap();
            let agents_before = serde_json::to_value(store.list::<Agent>("agent").unwrap()).unwrap();
            let source = || vec![SpecSource { name: "team.json".into(), content: r#"{"schemaVersion":1,"agents":[{"key":"cto","role":"architect","instructions":"Project-specific review"}]}"#.into() }];
            let imported = preview(&project.id, 0, source()).unwrap();
            store.stage_spec_import(&imported).unwrap();
            assert!(store.apply_spec_import(&imported.id, &[]).is_err());
            create_portable(&project, &imported.profile).unwrap();
            let first = preview_portable(&project, 0).unwrap();
            let stale = preview_portable(&project, 0).unwrap();
            store.put("spec_draft", &first.id, &first, None).unwrap();
            store.put("spec_draft", &stale.id, &stale, None).unwrap();
            let applied = store.apply_spec_import(&first.id, &[SpecAgentBinding { key: "cto".into(), agent_id: Some("cto".into()) }]).unwrap();
            assert_eq!(applied.revision, 1);
            assert_eq!(applied.agents[0].bound_agent_id.as_deref(), Some("cto"));
            assert!(store.apply_spec_import(&stale.id, &[]).is_err());
            let other_import = preview(&other.id, 0, vec![SpecSource { name: "OTHER.md".into(), content: "## Project\nOther project's rules".into() }]).unwrap();
            create_portable(&other, &other_import.profile).unwrap();
            let second_project = preview_portable(&other, 0).unwrap();
            store.stage_spec_import(&second_project).unwrap();
            store.apply_spec_import(&second_project.id, &[]).unwrap();
            assert_eq!(serde_json::to_value(store.list::<Agent>("agent").unwrap()).unwrap(), agents_before);
            assert_eq!(store.list::<SpecProfile>("spec_profile_version").unwrap().len(), 2);
            let selected = store.snapshot().unwrap();
            assert_eq!(selected.spec_profiles.len(), 1);
            assert_eq!(selected.spec_profiles[0].project_id, "project-a");
        }
        let reopened = Store::open(&path).unwrap();
        assert_eq!(reopened.get::<SpecProfile>("spec_profile", "project-a").unwrap().unwrap().revision, 1);
        drop(reopened);
        let resolved = dir.canonicalize().unwrap();
        assert!(resolved.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        std::fs::remove_dir_all(resolved).unwrap();
    }
}
