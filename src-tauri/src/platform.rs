use crate::model::Project;
use std::path::Path;
#[cfg(target_os = "linux")]
use std::path::PathBuf;
use std::process::Command;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf as LocalPathBuf;

const SNAPSHOT_MAX_FILES: usize = 20_000;
const SNAPSHOT_MAX_BYTES: u64 = 512 * 1024 * 1024;

fn folder_files(root: &Path) -> Result<BTreeMap<LocalPathBuf, u64>, String> {
    let mut files = BTreeMap::new();
    let mut pending = vec![LocalPathBuf::new()];
    let mut total = 0_u64;
    while let Some(relative) = pending.pop() {
        for entry in std::fs::read_dir(root.join(&relative)).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            // A parent project may contain nested repositories. Their working files belong
            // to the selected folder; Git metadata never belongs in an isolated task copy.
            if entry.file_name() == ".git" { continue; }
            // ADR-0049: reference material is copied per task (only what the task names), never wholesale.
            if relative.as_os_str().is_empty() && entry.file_name().to_string_lossy().eq_ignore_ascii_case(crate::references::FOLDER) { continue; }
            let path = relative.join(entry.file_name());
            let metadata = std::fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
            if metadata.file_type().is_symlink() { return Err(format!("심볼릭 링크는 일반 폴더 작업에 포함할 수 없습니다: {}", path.display())); }
            if metadata.is_dir() { pending.push(path); }
            else if metadata.is_file() {
                total = total.saturating_add(metadata.len());
                files.insert(path, metadata.len());
                if files.len() > SNAPSHOT_MAX_FILES || total > SNAPSHOT_MAX_BYTES {
                    return Err("프로젝트 복사 범위가 20,000개 파일 또는 512 MiB를 넘습니다. 작은 폴더를 선택하거나 Git 프로젝트로 관리해 주세요".into());
                }
            }
        }
    }
    Ok(files)
}

fn copy_folder_files(source: &Path, destination: &Path, files: &BTreeMap<LocalPathBuf, u64>) -> Result<(), String> {
    std::fs::create_dir_all(destination).map_err(|e| e.to_string())?;
    for relative in files.keys() {
        let target = destination.join(relative);
        if let Some(parent) = target.parent() { std::fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
        std::fs::copy(source.join(relative), target).map_err(|e| format!("{} 복사 실패: {e}", relative.display()))?;
    }
    Ok(())
}

fn check_target_parent(root: &Path, relative: &Path) -> Result<(), String> {
    let canonical_root = root.canonicalize().map_err(|e| e.to_string())?;
    let mut current = root.to_path_buf();
    if let Some(parent) = relative.parent() {
        for component in parent.components() {
            current.push(component.as_os_str());
            match std::fs::symlink_metadata(&current) {
                Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => return Err(format!("대상 상위 경로가 안전한 폴더가 아닙니다: {}", current.display())),
                Ok(_) => {
                    if !current.canonicalize().map_err(|e| e.to_string())?.starts_with(&canonical_root) { return Err("대상 경로가 프로젝트 밖입니다".into()); }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
                Err(e) => return Err(e.to_string()),
            }
        }
    }
    Ok(())
}

pub fn prepare_folder_copy(root: &Path, workspace: &Path) -> Result<(), String> {
    if workspace.exists() { return Err("작업 복사본이 이미 존재합니다. 이전 작업을 확인해 주세요".into()); }
    let baseline = workspace.with_extension("baseline");
    if baseline.exists() { return Err("기준 복사본이 이미 존재합니다. 이전 작업을 확인해 주세요".into()); }
    let files = folder_files(root)?;
    copy_folder_files(root, &baseline, &files)?;
    copy_folder_files(&baseline, workspace, &files)
}

/// Project-relative paths that differ between a folder-copy workspace and its baseline.
pub fn folder_changes(workspace: &Path) -> Result<Vec<String>, String> {
    let baseline = workspace.with_extension("baseline");
    let before = folder_files(&baseline)?;
    let after = folder_files(workspace)?;
    let names: BTreeSet<_> = before.keys().chain(after.keys()).cloned().collect();
    let mut changes = Vec::new();
    for relative in names {
        let old = if before.contains_key(&relative) { Some(std::fs::read(baseline.join(&relative)).map_err(|e| e.to_string())?) } else { None };
        let new = if after.contains_key(&relative) { Some(std::fs::read(workspace.join(&relative)).map_err(|e| e.to_string())?) } else { None };
        if old != new { changes.push(relative.to_string_lossy().replace('\\', "/")); }
    }
    Ok(changes)
}

pub fn integrate_folder_copy(root: &Path, workspace: &Path, allowed_files: &[String]) -> Result<Vec<String>, String> {
    let baseline = workspace.with_extension("baseline");
    let before = folder_files(&baseline)?;
    let after = folder_files(workspace)?;
    let names: BTreeSet<_> = before.keys().chain(after.keys()).cloned().collect();
    let mut changes = Vec::new();
    for relative in names {
        let old = if before.contains_key(&relative) { Some(std::fs::read(baseline.join(&relative)).map_err(|e| e.to_string())?) } else { None };
        let new = if after.contains_key(&relative) { Some(std::fs::read(workspace.join(&relative)).map_err(|e| e.to_string())?) } else { None };
        if old == new { continue; }
        let label = relative.to_string_lossy().replace('\\', "/");
        if !allowed_files.iter().any(|allowed| allowed.eq_ignore_ascii_case(&label)) { return Err(format!("Agent changed undeclared file: {label}")); }
        check_target_parent(root, &relative)?;
        let destination = root.join(&relative);
        let current = match std::fs::symlink_metadata(&destination) {
            Ok(meta) if meta.file_type().is_symlink() || !meta.is_file() => return Err(format!("대상 파일 형식이 바뀌었습니다: {label}")),
            Ok(_) => Some(std::fs::read(&destination).map_err(|e| e.to_string())?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.to_string()),
        };
        if current != old { return Err(format!("프로젝트 파일이 작업 중 변경되어 통합을 중단했습니다: {label}")); }
        changes.push((relative, new));
    }
    if changes.is_empty() { return Err("Task produced no file changes".into()); }
    let labels = changes.iter().map(|(path, _)| path.to_string_lossy().replace('\\', "/")).collect();
    for (relative, content) in changes {
        check_target_parent(root, &relative)?;
        let destination = root.join(relative);
        match content {
            Some(bytes) => {
                if let Some(parent) = destination.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                    if !parent.canonicalize().map_err(|e| e.to_string())?.starts_with(root.canonicalize().map_err(|e| e.to_string())?) { return Err("대상 경로가 프로젝트 밖입니다".into()); }
                }
                std::fs::write(destination, bytes).map_err(|e| e.to_string())?;
            }
            None => std::fs::remove_file(destination).map_err(|e| e.to_string())?,
        }
    }
    Ok(labels)
}

pub fn git_project(path: &str) -> Result<Project, String> {
    let canonical = Path::new(path).canonicalize().map_err(|e| format!("Project path: {e}"))?;
    if !canonical.is_dir() { return Err("Select a directory".into()); }
    let output = Command::new("git").arg("-C").arg(&canonical).args(["rev-parse", "--show-toplevel"]).output().map_err(|e| e.to_string())?;
    if !output.status.success() { return Err("Writing workflows need a Git project".into()); }
    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let root = Path::new(&root).canonicalize().map_err(|e| e.to_string())?;
    if root != canonical { return Err("Select the Git repository root".into()); }
    let name = root.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Project".into());
    Ok(Project { id: crate::model::id(), path: root.to_string_lossy().into_owned(), name, kind: "git".into(), review_loop_limit: crate::model::default_review_loop_limit() })
}

pub fn folder_project(path: &str) -> Result<Project, String> {
    let root = Path::new(path).canonicalize().map_err(|e| format!("Project path: {e}"))?;
    if !root.is_dir() { return Err("폴더를 선택해 주세요".into()); }
    let name = root.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Project".into());
    Ok(Project { id: crate::model::id(), path: root.to_string_lossy().into_owned(), name, kind: "folder".into(), review_loop_limit: crate::model::default_review_loop_limit() })
}

pub fn adopt_project(path: &str) -> Result<Project, String> {
    let folder = Path::new(path).canonicalize().map_err(|e| format!("프로젝트 폴더를 열 수 없습니다: {e}"))?;
    if !folder.is_dir() { return Err("폴더를 선택해 주세요".into()); }
    if let Ok(project) = git_project(&folder.to_string_lossy()) {
        let head = Command::new("git").arg("-C").arg(&folder).args(["rev-parse", "--verify", "HEAD"]).output();
        if head.is_ok_and(|result| result.status.success()) { return Ok(project); }
    }
    folder_project(&folder.to_string_lossy())
}

pub fn refresh_registered_project(mut saved: Project, detected: Project) -> Project {
    saved.path = detected.path;
    saved.kind = detected.kind;
    saved
}

pub fn create_project(parent: &str, name: &str) -> Result<Project, String> {
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\', ':']) || name.ends_with(['.', ' ']) {
        return Err("Enter one folder name without slashes or special path segments".into());
    }
    let parent = Path::new(parent).canonicalize().map_err(|e| format!("Parent folder: {e}"))?;
    if !parent.is_dir() { return Err("Choose an existing parent folder".into()); }
    let target = parent.join(name);
    if target.exists() { return Err("That folder already exists; choose another name or add it as an existing project".into()); }
    std::fs::create_dir(&target).map_err(|e| format!("Create project folder: {e}"))?;
    folder_project(&target.to_string_lossy())
}

pub fn pick_folder() -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        return std::thread::spawn(pick_windows_folder)
            .join().map_err(|_| "Windows 폴더 선택 창이 예기치 않게 종료되었습니다".to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("osascript").args(["-e", "POSIX path of (choose folder with prompt \"Choose a project folder\")"]).output().map_err(|e| format!("Open macOS folder picker: {e}"))?;
        if !output.status.success() { return Ok(None); }
        return Ok(nonempty_output(output.stdout));
    }
    #[cfg(target_os = "linux")]
    {
        let (program, args): (&str, Vec<&str>) = if find_on_path("zenity").is_some() { ("zenity", vec!["--file-selection", "--directory", "--title=Choose a project folder"]) }
            else if find_on_path("kdialog").is_some() { ("kdialog", vec!["--getexistingdirectory", ".", "Choose a project folder"]) }
            else { return Err("No native folder picker is available. Install zenity or kdialog, or enter the folder path manually".into()); };
        let output = Command::new(program).args(args).output().map_err(|e| format!("Open Linux folder picker: {e}"))?;
        if !output.status.success() { return Ok(None); }
        return Ok(nonempty_output(output.stdout));
    }
}

#[cfg(target_os = "windows")]
fn pick_windows_folder() -> Result<Option<String>, String> {
    use windows::core::{w, PWSTR};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{FileOpenDialog, IFileOpenDialog, FOS_FORCEFILESYSTEM, FOS_PATHMUSTEXIST, FOS_PICKFOLDERS, SIGDN_FILESYSPATH};

    // The common dialog requires STA. Keep COM initialization and release on this one thread.
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    initialized.ok().map_err(|error| format!("Windows 폴더 선택 창을 준비하지 못했습니다: {error}"))?;
    struct ComApartment;
    impl Drop for ComApartment { fn drop(&mut self) { unsafe { CoUninitialize(); } } }
    let _apartment = ComApartment;
    let dialog: IFileOpenDialog = unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER) }
        .map_err(|error| format!("Windows 폴더 선택 창을 열지 못했습니다: {error}"))?;
    unsafe {
        let options = dialog.GetOptions().map_err(|error| error.to_string())?;
        dialog.SetOptions(options | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST)
            .map_err(|error| format!("폴더 선택 모드를 설정하지 못했습니다: {error}"))?;
        dialog.SetTitle(w!("프로젝트 폴더 선택")).map_err(|error| error.to_string())?;
        dialog.SetOkButtonLabel(w!("이 폴더 선택")).map_err(|error| error.to_string())?;
        match dialog.Show(None) {
            Ok(()) => {},
            Err(error) if error.code().0 == 0x8007_04C7_u32 as i32 => return Ok(None),
            Err(error) => return Err(format!("폴더 선택 창에서 오류가 발생했습니다: {error}")),
        }
        let item = dialog.GetResult().map_err(|error| error.to_string())?;
        let path: PWSTR = item.GetDisplayName(SIGDN_FILESYSPATH).map_err(|error| error.to_string())?;
        let result = path.to_string().map_err(|error| error.to_string());
        CoTaskMemFree(Some(path.0.cast()));
        result.map(Some)
    }
}

#[cfg(not(target_os = "windows"))]
fn nonempty_output(bytes: Vec<u8>) -> Option<String> {
    let path = String::from_utf8_lossy(&bytes).trim().to_string();
    if path.is_empty() { None } else { Some(path) }
}

#[cfg(target_os = "linux")]
fn find_on_path(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?).map(|dir| dir.join(name)).find(|path| path.is_file())
}

pub fn create_worktree(root: &Path, destination: &Path) -> Result<(), String> {
    let output = Command::new("git").arg("-C").arg(root).args(["worktree", "add", "--detach"]).arg(destination).arg("HEAD").output().map_err(|e| e.to_string())?;
    if !output.status.success() { return Err(String::from_utf8_lossy(&output.stderr).into_owned()); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_copy_skips_reference_material_and_a_copied_reference_is_no_change() {
        let base = std::env::temp_dir().join(format!("utopia-folder-refs-{}", crate::model::id()));
        let root = base.join("project");
        std::fs::create_dir_all(root.join("work-references/g/text")).unwrap();
        std::fs::create_dir_all(root.join("docs/work-references")).unwrap();
        std::fs::write(root.join("work-references/g/text/R1.md"), "자료").unwrap();
        std::fs::write(root.join("docs/work-references/keep.md"), "nested folder with the same name is ordinary").unwrap();
        std::fs::write(root.join("a.md"), "a").unwrap();
        let workspace = base.join("ws");
        prepare_folder_copy(&root, &workspace).unwrap();
        assert!(!workspace.join("work-references").exists());
        assert!(workspace.join("docs/work-references/keep.md").is_file());
        // The engine copies a named reference into both the workspace and its baseline.
        for dir in [workspace.clone(), workspace.with_extension("baseline")] {
            std::fs::create_dir_all(dir.join("work-references/g/text")).unwrap();
            std::fs::write(dir.join("work-references/g/text/R1.md"), "자료").unwrap();
        }
        std::fs::write(workspace.join("a.md"), "changed").unwrap();
        assert_eq!(integrate_folder_copy(&root, &workspace, &["a.md".into()]).unwrap(), vec!["a.md".to_string()]);
    }

    #[test]
    fn creates_new_folder_project_without_overwriting_existing_folder() {
        let parent = std::env::temp_dir().join(format!("utopia-new-project-{}", crate::model::id()));
        std::fs::create_dir_all(&parent).unwrap();
        let project = create_project(&parent.to_string_lossy(), "Team One").unwrap();
        assert_eq!(project.name, "Team One");
        assert!(!project.id.is_empty());
        assert_eq!(project.kind, "folder");
        assert!(!Path::new(&project.path).join("README.md").exists());
        assert!(!Path::new(&project.path).join(".git").exists());
        assert!(create_project(&parent.to_string_lossy(), "Team One").unwrap_err().contains("already exists"));
        assert!(create_project(&parent.to_string_lossy(), "../outside").is_err());
        let resolved = parent.canonicalize().unwrap();
        assert!(resolved.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(resolved.file_name().unwrap().to_string_lossy().starts_with("utopia-new-project-"));
        std::fs::remove_dir_all(resolved).unwrap();
    }

    #[test]
    fn unified_folder_flow_adopts_git_and_plain_folders_unchanged() {
        let parent = std::env::temp_dir().join(format!("utopia-folder-flow-{}", crate::model::id()));
        std::fs::create_dir_all(&parent).unwrap();
        let empty = parent.join("empty");
        std::fs::create_dir(&empty).unwrap();
        let created = adopt_project(&empty.to_string_lossy()).unwrap();
        assert_eq!(created.kind, "folder");
        let adopted = adopt_project(&empty.to_string_lossy()).unwrap();
        assert_eq!(created.path, adopted.path);
        assert!(!empty.join(".git").exists());
        let nested = empty.join("nested");
        std::fs::create_dir(&nested).unwrap();
        assert_eq!(adopt_project(&nested.to_string_lossy()).unwrap().kind, "folder");
        let nonempty = parent.join("nonempty");
        std::fs::create_dir(&nonempty).unwrap();
        std::fs::write(nonempty.join("keep.txt"), "keep").unwrap();
        assert_eq!(adopt_project(&nonempty.to_string_lossy()).unwrap().kind, "folder");
        assert_eq!(std::fs::read_to_string(nonempty.join("keep.txt")).unwrap(), "keep");
        assert!(!nonempty.join(".git").exists());
        let git_root = parent.join("git-root");
        std::fs::create_dir(&git_root).unwrap();
        std::fs::write(git_root.join("keep.txt"), "keep").unwrap();
        for args in [vec!["init"], vec!["add", "keep.txt"], vec!["-c", "user.name=QA", "-c", "user.email=qa@local.invalid", "commit", "-m", "baseline"]] {
            assert!(Command::new("git").arg("-C").arg(&git_root).args(args).status().unwrap().success());
        }
        assert_eq!(adopt_project(&git_root.to_string_lossy()).unwrap().kind, "git");
        let saved_as_folder = Project { id: "stable-project-id".into(), review_loop_limit: 4, ..folder_project(&git_root.to_string_lossy()).unwrap() };
        let refreshed = refresh_registered_project(saved_as_folder, adopt_project(&git_root.to_string_lossy()).unwrap());
        assert_eq!(refreshed.kind, "git");
        assert_eq!(refreshed.id, "stable-project-id");
        assert_eq!(refreshed.review_loop_limit, 4);
        assert_eq!(std::fs::read_to_string(git_root.join("keep.txt")).unwrap(), "keep");
        let parent_project = adopt_project(&parent.to_string_lossy()).unwrap();
        assert_eq!(parent_project.kind, "folder");
        let copy = parent.with_extension("task-copy");
        prepare_folder_copy(&parent, &copy).unwrap();
        assert_eq!(std::fs::read_to_string(copy.join("git-root/keep.txt")).unwrap(), "keep");
        assert!(!copy.join("git-root/.git").exists());
        std::fs::remove_dir_all(&copy).unwrap();
        std::fs::remove_dir_all(copy.with_extension("baseline")).unwrap();
        let resolved = parent.canonicalize().unwrap();
        assert!(resolved.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(resolved.file_name().unwrap().to_string_lossy().starts_with("utopia-folder-flow-"));
        std::fs::remove_dir_all(resolved).unwrap();
    }

    #[test]
    fn plain_folder_copy_rejects_undeclared_and_conflicting_changes() {
        let parent = std::env::temp_dir().join(format!("utopia-plain-copy-{}", crate::model::id()));
        let root = parent.join("project");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("keep.txt"), "original").unwrap();
        let a = parent.join("task-a");
        let b = parent.join("task-b");
        prepare_folder_copy(&root, &a).unwrap();
        prepare_folder_copy(&root, &b).unwrap();
        std::fs::write(a.join("wrong.txt"), "bad").unwrap();
        assert!(integrate_folder_copy(&root, &a, &["keep.txt".into()]).unwrap_err().contains("undeclared"));
        assert!(!root.join("wrong.txt").exists());
        std::fs::remove_file(a.join("wrong.txt")).unwrap();
        std::fs::write(a.join("keep.txt"), "task a").unwrap();
        std::fs::write(root.join("keep.txt"), "user edit").unwrap();
        assert!(integrate_folder_copy(&root, &a, &["keep.txt".into()]).unwrap_err().contains("변경"));
        assert_eq!(std::fs::read_to_string(root.join("keep.txt")).unwrap(), "user edit");
        std::fs::write(b.join("new.txt"), "task b").unwrap();
        assert_eq!(integrate_folder_copy(&root, &b, &["new.txt".into()]).unwrap(), vec!["new.txt"]);
        assert_eq!(std::fs::read_to_string(root.join("new.txt")).unwrap(), "task b");
        std::fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn old_project_records_default_to_git_mode() {
        let old = serde_json::json!({"id":"legacy","path":"C:/old","name":"Old","reviewLoopLimit":2});
        let project: Project = serde_json::from_value(old).unwrap();
        assert_eq!(project.kind, "git");
    }
}
