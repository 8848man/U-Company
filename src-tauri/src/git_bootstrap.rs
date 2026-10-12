use crate::model::Project;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateFile {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub project_id: String,
    pub root: String,
    pub files: Vec<CandidateFile>,
    pub fingerprint: String,
    pub initialized: bool,
    pub suggested_author_name: String,
    pub suggested_author_email: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultInfo {
    pub commit_id: String,
    pub branch: String,
    pub committed_files: Vec<String>,
    pub remaining_changes: usize,
    pub qa_started: bool,
}

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git").arg("-C").arg(root).args(args).output().map_err(|e| format!("Git 실행 실패: {e}"))?;
    if !output.status.success() {
        return Err(format!("Git 작업 실패: {}", String::from_utf8_lossy(&output.stderr).chars().take(400).collect::<String>()));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(if args.contains(&"-z") { text.into_owned() } else { text.trim().into() })
}

fn valid_root(project: &Project) -> Result<(PathBuf, bool), String> {
    let root = Path::new(&project.path).canonicalize().map_err(|e| format!("프로젝트 폴더를 열 수 없습니다: {e}"))?;
    if !root.is_dir() || !root.to_string_lossy().eq_ignore_ascii_case(&project.path) { return Err("등록된 프로젝트 경로가 바뀌었습니다. 프로젝트를 다시 선택해 주세요".into()); }
    let dotgit = root.join(".git");
    let meta = std::fs::symlink_metadata(&dotgit);
    let initialized = match meta {
        Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => true,
        Ok(_) => return Err(".git 경로가 일반 폴더가 아닙니다. 기존 저장소 상태를 직접 확인해 주세요".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => return Err(e.to_string()),
    };
    if initialized {
        let info_attributes = dotgit.join("info").join("attributes");
        if info_attributes.exists() && std::fs::read_to_string(info_attributes).map_err(|e| e.to_string())?.contains("filter=") { return Err("저장소 정보에 외부 Git 필터가 있어 앱 내 첫 커밋을 진행할 수 없습니다".into()); }
        let top = git(&root, &["rev-parse", "--show-toplevel"])?;
        let top = Path::new(&top).canonicalize().map_err(|e| e.to_string())?;
        if top != root { return Err("선택한 폴더가 Git 저장소 루트가 아닙니다".into()); }
        if git(&root, &["rev-parse", "--verify", "HEAD"]).is_ok() { return Err("이미 첫 커밋이 있는 Git 저장소입니다".into()); }
        if git(&root, &["symbolic-ref", "--short", "HEAD"])? != "main" { return Err("초기화된 저장소의 브랜치가 main이 아닙니다. 상태를 확인해 주세요".into()); }
    } else if let Ok(top) = git(&root, &["rev-parse", "--show-toplevel"]) {
        let top = Path::new(&top).canonicalize().map_err(|e| e.to_string())?;
        if top != root { return Err("이 폴더는 상위 Git 저장소 안에 있습니다. 중첩 저장소 초기화는 별도로 검토해 주세요".into()); }
    }
    Ok((root, initialized))
}

pub fn preview(project: &Project) -> Result<Preview, String> {
    let (root, initialized) = valid_root(project)?;
    let mut pending = vec![PathBuf::new()];
    let mut files = Vec::new();
    let mut total_bytes = 0_u64;
    while let Some(relative) = pending.pop() {
        for entry in std::fs::read_dir(root.join(&relative)).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.file_name() == ".git" { continue; }
            let path = relative.join(entry.file_name());
            let meta = std::fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
            if meta.file_type().is_symlink() { return Err(format!("심볼릭 링크는 초기 커밋 범위에 포함할 수 없습니다: {}", path.display())); }
            if meta.is_dir() { pending.push(path); continue; }
            if !meta.is_file() { return Err(format!("지원하지 않는 파일 형식: {}", path.display())); }
            total_bytes = total_bytes.saturating_add(meta.len());
            if files.len() >= 1000 || total_bytes > 64 * 1024 * 1024 { return Err("초기 커밋 미리보기는 1000개 파일·64 MiB까지 지원합니다. 더 작은 폴더를 선택해 주세요".into()); }
            let content = std::fs::read(entry.path()).map_err(|e| e.to_string())?;
            if entry.file_name() == ".gitattributes" && String::from_utf8_lossy(&content).contains("filter=") { return Err(".gitattributes의 외부 필터 설정이 있어 앱 내 첫 커밋을 진행할 수 없습니다".into()); }
            files.push(CandidateFile { path: path.to_string_lossy().replace('\\', "/"), bytes: meta.len(), sha256: format!("{:x}", Sha256::digest(&content)) });
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let mut digest = Sha256::new();
    digest.update(root.to_string_lossy().as_bytes());
    for file in &files { digest.update(file.path.as_bytes()); digest.update(file.sha256.as_bytes()); }
    Ok(Preview { project_id: project.id.clone(), root: root.to_string_lossy().into_owned(), files, fingerprint: format!("{:x}", digest.finalize()), initialized, suggested_author_name: git(&root, &["config", "--get", "user.name"]).unwrap_or_default(), suggested_author_email: git(&root, &["config", "--get", "user.email"]).unwrap_or_default() })
}

fn clean_text(label: &str, value: &str, max: usize) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max || value.chars().any(char::is_control) { return Err(format!("{label}을(를) 1~{max}자의 한 줄로 입력해 주세요")); }
    Ok(value.into())
}

struct EmptyGitHooks(PathBuf);

impl Drop for EmptyGitHooks {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.0.join("attributes"));
        let _ = std::fs::remove_dir(&self.0);
    }
}

pub fn validate_request(project: &Project, fingerprint: &str, selected: &[String], author_name: &str, author_email: &str, message: &str) -> Result<Preview, String> {
    let current = preview(project)?;
    if current.fingerprint != fingerprint { return Err("미리보기 이후 프로젝트 파일이 바뀌었습니다. 범위를 다시 확인해 주세요".into()); }
    clean_text("작성자 이름", author_name, 120)?;
    let email = clean_text("작성자 이메일", author_email, 180)?;
    if !email.contains('@') || email.contains(char::is_whitespace) { return Err("작성자 이메일을 확인해 주세요".into()); }
    clean_text("커밋 메시지", message, 200)?;
    let chosen: BTreeSet<_> = selected.iter().cloned().collect();
    if chosen.is_empty() || chosen.len() != selected.len() { return Err("커밋할 파일을 중복 없이 하나 이상 선택해 주세요".into()); }
    let candidates: BTreeSet<_> = current.files.iter().map(|file| file.path.clone()).collect();
    if !chosen.is_subset(&candidates) { return Err("미리보기에 없는 파일이 선택됐습니다. 다시 확인해 주세요".into()); }
    Ok(current)
}

pub fn bootstrap(project: &Project, fingerprint: &str, selected: &[String], author_name: &str, author_email: &str, message: &str) -> Result<ResultInfo, String> {
    let current = validate_request(project, fingerprint, selected, author_name, author_email, message)?;
    let name = author_name.trim();
    let email = author_email.trim();
    let message = message.trim();
    let chosen: BTreeSet<_> = selected.iter().cloned().collect();
    let root = Path::new(&current.root);
    if !current.initialized { git(root, &["init", "-b", "main"])?; }
    let hooks = EmptyGitHooks(std::env::temp_dir().join(format!("utopia-empty-hooks-{}", crate::model::id())));
    std::fs::create_dir(&hooks.0).map_err(|e| e.to_string())?;
    let attributes = hooks.0.join("attributes");
    std::fs::write(&attributes, []).map_err(|e| e.to_string())?;
    let already_staged = git(root, &["ls-files", "-z"])?;
    if !already_staged.is_empty() && !already_staged.split('\0').filter(|s| !s.is_empty()).all(|path| chosen.contains(path)) {
        return Err("이전 Git 작업에서 선택 범위 밖 파일이 스테이징됐습니다. 저장소 상태를 확인해 주세요".into());
    }
    let mut add = Command::new("git");
    add.arg("-C").arg(root).arg("-c").arg("core.fsmonitor=false").arg("-c").arg(format!("core.attributesFile={}", attributes.display())).args(["add", "--"]);
    for path in &chosen { add.arg(path); }
    let output = add.output().map_err(|e| e.to_string())?;
    if !output.status.success() { return Err(format!("선택한 파일 스테이징 실패: {}", String::from_utf8_lossy(&output.stderr).chars().take(400).collect::<String>())); }
    let mut normalize = Command::new("git");
    normalize.arg("-C").arg(root).arg("-c").arg("core.fsmonitor=false").arg("-c").arg(format!("core.attributesFile={}", attributes.display())).args(["add", "--renormalize", "--"]);
    for path in &chosen { normalize.arg(path); }
    let output = normalize.output().map_err(|e| e.to_string())?;
    if !output.status.success() { return Err(format!("선택한 파일의 Git 인덱스 줄바꿈 정리 실패: {}", String::from_utf8_lossy(&output.stderr).chars().take(400).collect::<String>())); }
    let staged = git(root, &["ls-files", "-z"])?;
    let staged: BTreeSet<_> = staged.split('\0').filter(|s| !s.is_empty()).map(str::to_string).collect();
    if staged != chosen { return Err("Git 스테이징 결과가 승인한 파일 목록과 다릅니다. 커밋하지 않았습니다".into()); }
    if preview(project)?.fingerprint != fingerprint { return Err("스테이징 중 프로젝트 파일이 바뀌었습니다. 커밋하지 않았습니다".into()); }
    let mut commit = Command::new("git");
    commit.arg("-C").arg(root).arg("-c").arg(format!("user.name={name}")).arg("-c").arg(format!("user.email={email}"))
        .arg("-c").arg("core.fsmonitor=false").arg("-c").arg("commit.gpgSign=false")
        .arg("-c").arg(format!("core.attributesFile={}", attributes.display()))
        .arg("-c").arg(format!("core.hooksPath={}", hooks.0.display())).args(["commit", "-m"]).arg(message);
    let result = commit.output().map_err(|e| e.to_string());
    let result = result?;
    if !result.status.success() { return Err(format!("첫 커밋 실패: {}", String::from_utf8_lossy(&result.stderr).chars().take(400).collect::<String>())); }
    let branch = git(root, &["branch", "--show-current"])?;
    let commit_id = git(root, &["rev-parse", "HEAD"])?;
    let count = git(root, &["rev-list", "--count", "HEAD"])?;
    if branch != "main" || count != "1" { return Err("커밋 후 브랜치 또는 커밋 수가 예상과 다릅니다. 저장소를 확인해 주세요".into()); }
    let remaining_changes = git(root, &["status", "--porcelain"])?.lines().count();
    Ok(ResultInfo { commit_id, branch, committed_files: chosen.into_iter().collect(), remaining_changes, qa_started: false })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (PathBuf, Project) {
        let root = std::env::temp_dir().join(format!("utopia-bootstrap-test-{}", crate::model::id()));
        std::fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let project = Project { id: "test-project".into(), path: root.to_string_lossy().into_owned(), name: "Test".into(), kind: "folder".into(), review_loop_limit: 2 };
        (root, project)
    }

    fn clean(root: PathBuf) {
        let temp = std::env::temp_dir().canonicalize().unwrap();
        let root = root.canonicalize().unwrap();
        assert!(root.starts_with(temp) && root.file_name().unwrap().to_string_lossy().starts_with("utopia-bootstrap-test-"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bootstraps_only_approved_files_on_main() {
        let (root, project) = fixture();
        std::fs::write(root.join(".gitattributes"), "* text=auto\n").unwrap();
        std::fs::write(root.join("README.md"), "Hello\n").unwrap();
        std::fs::write(root.join("secret.txt"), "not selected\n").unwrap();
        let scope = preview(&project).unwrap();
        let selected = vec![".gitattributes".into(), "README.md".into()];
        let result = bootstrap(&project, &scope.fingerprint, &selected, "Test User", "test@example.invalid", "chore: bootstrap").unwrap();
        assert_eq!(result.branch, "main");
        assert_eq!(result.committed_files, selected);
        assert_eq!(git(&root, &["rev-list", "--count", "HEAD"]).unwrap(), "1");
        assert_eq!(git(&root, &["ls-files"]).unwrap().lines().collect::<Vec<_>>(), vec![".gitattributes", "README.md"]);
        assert!(result.remaining_changes > 0);
        clean(root);
    }

    #[test]
    fn changed_preview_rejects_before_creating_git_metadata() {
        let (root, project) = fixture();
        std::fs::write(root.join("README.md"), "first\n").unwrap();
        let scope = preview(&project).unwrap();
        std::fs::write(root.join("README.md"), "changed\n").unwrap();
        let error = bootstrap(&project, &scope.fingerprint, &["README.md".into()], "Test User", "test@example.invalid", "chore: bootstrap").unwrap_err();
        assert!(error.contains("미리보기 이후"));
        assert!(!root.join(".git").exists());
        clean(root);
    }

    #[test]
    fn retries_an_unborn_main_repository() {
        let (root, project) = fixture();
        std::fs::write(root.join("README.md"), "Hello\n").unwrap();
        git(&root, &["init", "-b", "main"]).unwrap();
        let scope = preview(&project).unwrap();
        assert!(scope.initialized);
        let result = bootstrap(&project, &scope.fingerprint, &["README.md".into()], "Test User", "test@example.invalid", "chore: bootstrap").unwrap();
        assert_eq!(result.branch, "main");
        clean(root);
    }
}
