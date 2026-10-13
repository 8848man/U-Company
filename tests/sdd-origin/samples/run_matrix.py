#!/usr/bin/env python3
"""SDD 표본 3종(+ 참조 오리진) × 픽스처 테스트 실행 및 채점.

실험 1 (표본 단독): 표본의 절차를 감지 없이 그대로 실행 — 표본이 '오리진 그 자체'일 때
실험 2 (오리진 = 감지 계층 + 표본): 계획서의 감지 규칙으로 먼저 분기하고,
          신호가 없을 때만 표본으로 부트스트랩 — 표본을 '오리진의 부트스트랩 내용물'로 쓸 때
참조 오리진(uref)은 감지를 스스로 포함하므로 실험 2로만 실행한다.
추가: C5(재실행 시 진행 상태·사람이 쓴 내용 보존), D(파일만으로 다음 작업 찾기)

사용법: python3 samples/run_matrix.py [출력폴더]   → results.json, results.csv, summary.txt
"""
import csv, json, re, shutil, subprocess, sys, tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(HERE))
import judge                                    # noqa: E402
from origins import SAMPLES, FILLED, tick, w    # noqa: E402

FIX = ROOT / "fixtures"
MANIFEST = json.loads((ROOT / "manifest.json").read_text(encoding="utf-8"))
OUT = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "sample_results"
FIXTURES = ["F0", "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11"]
GUIDE = judge.CFG["guideline_paths"]
FOREIGN_SDD = [".ucompany/sdd", "specs", ".specify", "openspec"]
HUMAN_MEMO = "## 팀 메모 (사람이 추가)\n- 배포는 금요일에 하지 않는다.\n"


def git(repo, *a):
    return subprocess.run(["git", "-C", str(repo), *a], capture_output=True, text=True, check=True).stdout


def snapshot(repo, tag):
    git(repo, "add", "-A"); git(repo, "commit", "-q", "--allow-empty", "-m", f"snapshot {tag}"); git(repo, "tag", "-f", tag)


def changes(repo, tag):
    out = {}
    for line in git(repo, "diff", "--name-status", "--no-renames", tag).splitlines():
        st, p = line.split("\t", 1); out[p] = st[0]
    for p in git(repo, "ls-files", "--others", "--exclude-standard").splitlines():
        out[p] = "A"
    return out


def tag_files(repo, tag):
    return set(git(repo, "ls-tree", "-r", "--name-only", tag).splitlines())


def under(p, prefix):
    return p == prefix or p.startswith(prefix.rstrip("/") + "/")


def done_items(text):
    return {t.strip() for t in re.findall(r"^\s*- \[[xX]\]\s*(.+)$", text, re.M)}


def expected_mode(sample, fid):
    if fid in ("F0", "F1", "F11"):
        return "bootstrap"    # F11: 빈 AGENTS.md는 없는 것으로 간주
    if fid in ("F9", "F10"):
        return "ask"          # 내용이 있는 지침은 유지하고, 덮어쓰기·SDD 추가 여부를 먼저 묻는다          # 코드 스타일만 있는 AGENTS.md → SDD 구간 추가 여부를 사용자에게 확인
    if any((FIX / fid / m).exists() for m in sample.own_markers):
        return "own"          # 이미 그 표본 형식인 프로젝트 (F5×Spec Kit, F7×참조 오리진)
    return "existing"


def detect(repo, sample):
    """감지 계층: 자기 형식 표식 → own, 그 외는 계획서 규칙(judge.expect)"""
    if any((repo / m).exists() for m in sample.own_markers):
        return "own"
    mode, _ = judge.expect(repo)
    return {"continue": "existing"}.get(mode, mode)   # U 컴퍼니 .ucompany/sdd/는 표본 입장에선 외부 SDD


def follow_existing(repo, fid):
    """기존 워크플로우를 따르는 동작 (표본은 호출하지 않음)"""
    spec_dir = MANIFEST[fid].get("spec_dir")
    if fid == "F7":                                         # U 컴퍼니 SDD 지침대로 다음 작업 진행
        tick(repo / ".ucompany/sdd/features/due-date/tasks.md", 1)
        (repo / ".ucompany/sdd/handoff.md").open("a", encoding="utf-8").write("- T3 완료, T4부터.\n")
    elif fid == "F5":
        w(repo, "specs/002-due-date/spec.md", "# spec\n")
    elif spec_dir:
        name = "0002-due-date.md" if spec_dir.endswith("rfc") else "due-date.md"   # F4는 결정 4로 AGENTS.md(docs/rfc) 우선
        w(repo, f"{spec_dir}/{name}", "# 마감일\n")


def continue_own(repo, sample):
    """이미 그 형식인 프로젝트 → 재초기화 없이 그 규칙대로 이어서 작업"""
    if sample.key == "uref":
        tick(next(repo.glob(sample.tasks_glob)), 1)
        (repo / ".ucompany/sdd/handoff.md").open("a", encoding="utf-8").write("- T3 완료, T4부터.\n")
        sample.write_entry(repo)                            # 진입점 관리 구간만 갱신
    else:
        w(repo, "specs/002-due-date/spec.md", "# spec\n")   # Spec Kit 규칙: specs/NNN-name/


def act(repo, sample, mode):
    if mode == "bootstrap":
        sample.bootstrap(repo)
    elif mode == "own":
        continue_own(repo, sample)
    elif mode == "existing":
        follow_existing(repo, repo.name)
    # ask / undecided → 사용자 확인 전이므로 아무것도 하지 않음


# ------------------------------------------------------------------ 판정
def R(rows, check, ok, msg, level="must"):
    rows.append({"check": check, "status": ("PASS" if ok else ("FAIL" if level == "must" else "WARN")), "msg": msg})


def entry_check(repo, sample):
    """파일 기반 단일 진입점: 루트 AGENTS.md 관리 구간이 인덱스 파일을 가리키는가"""
    a = repo / "AGENTS.md"
    if not a.is_file():
        return False, "루트 AGENTS.md 없음 → 명령 없이 들어온 툴이 SDD를 찾을 곳 없음"
    block, _ = judge.managed_block(a.read_text(encoding="utf-8"))
    if block is None:
        return False, "AGENTS.md에 관리 표식 없음"
    if not sample.index_file or sample.index_file not in block:
        return False, "관리 구간이 인덱스 파일을 가리키지 않음"
    return True, f"AGENTS.md 관리 구간 → {sample.index_file}"


def judge_existing(repo, sample, fid, tag="fixture-init"):
    rows, ch, orig = [], changes(repo, tag), tag_files(repo, tag)
    made = sorted({p.split("/")[0] for p, s in ch.items() if s == "A" and any(under(p, o) for o in sample.origin_paths)
                   and not (fid == "F5" and under(p, "specs/002-due-date") and sample.key != "speckit")})
    R(rows, "no-origin-artifacts", not made, "표본 산출물 생성 없음" if not made else f"기존 프로젝트에 표본 산출물 생성: {made}")
    touched = [f"{p}({s})" for p, s in ch.items() if any(under(p, g) for g in GUIDE)]
    R(rows, "guidelines-untouched", not touched, "지침 파일 변경 없음" if not touched else f"지침 파일 변경: {touched}")
    lost = [p for p, s in ch.items() if s == "D" and p in orig]
    regressed = []
    for p in orig:
        if p.endswith("tasks.md") and any(under(p, f) for f in FOREIGN_SDD) and (repo / p).is_file():
            before = done_items(git(repo, "show", f"{tag}:{p}"))
            regressed += [f"{p}:{t}" for t in before - done_items((repo / p).read_text(encoding="utf-8"))]
    R(rows, "existing-intact", not lost and not regressed,
      "기존 파일·진행 상태 보존" if not (lost or regressed) else f"삭제 {lost} / 완료 표시 손실 {regressed}")
    return rows


def judge_ask(repo, tag="fixture-init"):
    made = sorted(f"{p}({s})" for p, s in changes(repo, tag).items())
    rows = []
    R(rows, "no-change-before-consent", not made, "확인 전 변경 없음" if not made else f"확인 없이 변경: {made[:4]}")
    return rows


def judge_own(repo, sample, tag="fixture-init"):
    """자기 형식 프로젝트: 기존 원문을 지우지 않고 이어서 작업"""
    if sample.key == "uref":   # U 컴퍼니 형식은 기존 판정 도구의 continue 판정을 그대로 사용
        _, r = judge.check(repo, mode="continue")
        return [{"check": c, "status": s, "msg": m} for c, s, m in r.rows if s != "INFO"]
    rows, ch = [], changes(repo, tag)
    over = []
    for p in tag_files(repo, tag):
        if under(p, ".specify/memory") or under(p, "specs/001-todo-basics"):
            before = [l for l in git(repo, "show", f"{tag}:{p}").splitlines() if l.strip()]
            now = (repo / p).read_text(encoding="utf-8") if (repo / p).is_file() else ""
            if any(l not in now for l in before):          # 덧붙이기는 허용, 원문이 사라지면 실패
                over.append(p)
    R(rows, "own-content-kept", not over, "기존 헌법·스펙 원문 보존" if not over else f"기존 원문 손실: {over}")
    touched = [p for p, s in ch.items() if any(under(p, g) for g in GUIDE)]
    R(rows, "guidelines-untouched", not touched, "지침 파일 변경 없음" if not touched else f"지침 파일 변경: {touched}")
    return rows


def judge_bootstrap(repo, sample, tag="fixture-init"):
    rows, ch, orig = [], changes(repo, tag), tag_files(repo, tag)
    miss = [f for f in sample.required if not (repo / f).exists()]
    R(rows, "C1-structure", not miss, "필수 구조 생성" if not miss else f"누락: {miss}")
    if sample.feature_glob:
        full = [d.name for d in repo.glob(sample.feature_glob) if d.is_dir() and all((d / f).is_file() for f in sample.feature_files)]
    else:
        full = ["(루트)"] if all((repo / f).is_file() for f in sample.required) else []
    R(rows, "C4-spec-plan-tasks", bool(full), f"스펙·계획·작업 문서 완비: {full}" if full else "스펙·계획·작업 문서 미완비")
    iso = bool(sample.feature_glob and full)
    R(rows, "C4-feature-isolation", iso, "기능별 폴더로 분리" if iso else "기능 구분 없이 루트 단일 파일 → 기능이 둘 이상이면 섞임", level="should")
    ok, msg = entry_check(repo, sample)
    R(rows, "C3-single-entry", ok, msg)
    ok, msg = judge.bridge_ok(repo)
    R(rows, "C6-claude-bridge", ok, msg)
    mod = [f"{p}({s})" for p, s in ch.items() if p in orig and s in "MD"]
    R(rows, "C2-originals-untouched", not mod, "기존 파일 변경 없음" if not mod else f"기존 파일 변경: {mod}")
    return rows


def judge_rerun(repo, sample, tag, had_entry):
    rows, ch = [], changes(repo, tag)
    regressed = []
    for p in repo.glob(sample.tasks_glob):
        rel = str(p.relative_to(repo))
        before = done_items(git(repo, "show", f"{tag}:{rel}"))
        regressed += sorted(before - done_items(p.read_text(encoding="utf-8")))
    R(rows, "C5-done-tasks-kept", not regressed, "완료 표시 유지" if not regressed else f"완료 표시 사라짐: {regressed}")
    reset = [str(p.relative_to(repo)) for g in sample.filled_files for p in repo.glob(g) if FILLED not in p.read_text(encoding="utf-8")]
    R(rows, "C5-filled-content-kept", not reset, "작성한 내용 유지" if not reset else f"템플릿으로 되돌아감: {reset}")
    dele = [p for p, s in ch.items() if s == "D"]
    R(rows, "C5-nothing-deleted", not dele, "삭제 없음" if not dele else f"삭제: {dele}")
    if had_entry:   # 진입점이 있는 오리진만: 관리 구간 밖 사람이 쓴 내용 보존
        now = (repo / "AGENTS.md").read_text(encoding="utf-8")
        ok = all(l in now for l in HUMAN_MEMO.splitlines() if l.strip())
        R(rows, "C5-human-text-kept", ok, "관리 구간 밖 팀 메모 보존" if ok else "팀 메모가 사라짐")
    return rows


def next_task(repo, sample):
    for p in sorted(repo.glob(sample.tasks_glob)):
        m = re.search(r"^\s*- \[ \]\s*(.+)$", p.read_text(encoding="utf-8"), re.M)
        if m:
            return str(p.relative_to(repo)), m.group(1).strip()
    return None, None


# ------------------------------------------------------------------ 실행
def fresh(tmp, sample, label, fid):
    dst = Path(tmp) / sample.key / label / fid
    shutil.copytree(FIX / fid, dst)
    return dst


def judge_for(repo, s, fid, exp_mode):
    if exp_mode == "bootstrap":
        return "C-bootstrap", judge_bootstrap(repo, s)
    if exp_mode == "own":
        return "B-existing", judge_own(repo, s)
    if exp_mode == "ask":
        return "B-existing", judge_ask(repo)
    return "B-existing", judge_existing(repo, s, fid)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    results = []

    def add(exp, sample, case, fid, rows, extra=None):
        for r in rows:
            results.append({"exp": exp, "sample": sample.key, "grade": sample.grade, "case": case, "fixture": fid, **r, **(extra or {})})

    with tempfile.TemporaryDirectory() as tmp:
        for s in SAMPLES:
            for fid in FIXTURES:
                exp_mode = expected_mode(s, fid)
                if not s.has_detection:
                    # 실험 1: 표본 단독 — 감지 없이 항상 표본 절차 실행
                    repo = fresh(tmp, s, "exp1", fid)
                    s.bootstrap(repo)
                    add("exp1", s, "A-routing", fid, [{"check": "A-mode", "status": "PASS" if exp_mode == "bootstrap" else "FAIL",
                        "msg": f"기대 {exp_mode} / 실제 bootstrap (감지 절차 없음)"}])
                    case, rows = judge_for(repo, s, fid, exp_mode)
                    add("exp1", s, case, fid, rows)
                # 실험 2: 감지 계층 + 표본 (참조 오리진은 자체 감지)
                repo = fresh(tmp, s, "exp2", fid)
                got = detect(repo, s)
                add("exp2", s, "A-routing", fid, [{"check": "A-mode", "status": "PASS" if got == exp_mode else "FAIL",
                    "msg": f"기대 {exp_mode} / 감지 {got}"}])
                act(repo, s, got)
                case, rows = judge_for(repo, s, fid, exp_mode)
                add("exp2", s, case, fid, rows)

            # C5: 자기 형식으로 진행 중인 프로젝트에 재실행 (사람이 AGENTS.md에 메모를 더한 상태)
            repo = fresh(tmp, s, "rerun", "F0")
            s.bootstrap(repo)
            tick(next(repo.glob(s.tasks_glob)), 2)
            had_entry = (repo / "AGENTS.md").exists()
            if had_entry:
                (repo / "AGENTS.md").open("a", encoding="utf-8").write("\n" + HUMAN_MEMO)
            snapshot(repo, "before-rerun")
            s.rerun(repo)
            add("rerun", s, "C5-rerun", "F0", judge_rerun(repo, s, "before-rerun", had_entry),
                {"note": "" if s.rerun_defined else "재실행 절차가 문서에 없음 → 계획 단계를 문서대로 다시 수행"})

            # D: 파일만으로 다음 작업 찾기 + 단일 진입점 + 여러 툴
            repo = fresh(tmp, s, "handoff", "F0")
            s.bootstrap(repo)
            tick(next(repo.glob(s.tasks_glob)), 2)
            f, t = next_task(repo, s)
            ok_e, msg_e = entry_check(repo, s)
            ok_b, _ = judge.bridge_ok(repo)
            multi = (s.tools_supported or 0) > 1 or (ok_e and ok_b)
            add("handoff", s, "D-continuity", "F0", [
                {"check": "D1-next-task-from-files", "status": "PASS" if t and (t.startswith("T3") or t.startswith("T-003")) else "FAIL", "msg": f"{f}: {t}"},
                {"check": "D2-single-entry", "status": "PASS" if ok_e else "FAIL", "msg": msg_e},
                {"check": "D3-multi-tool", "status": "PASS" if multi else "FAIL",
                 "msg": (f"공식 연동 툴 {s.tools_supported}개" if s.tools_supported else
                         "툴 중립 진입점(AGENTS.md) + CLAUDE.md 연결" if multi else "툴 연동도 툴 중립 진입점도 없음")}])

    (OUT / "results.json").write_text(json.dumps(results, ensure_ascii=False, indent=1), encoding="utf-8")
    with (OUT / "results.csv").open("w", newline="", encoding="utf-8-sig") as fp:
        wr = csv.DictWriter(fp, fieldnames=["exp", "sample", "grade", "case", "fixture", "check", "status", "msg", "note"])
        wr.writeheader(); [wr.writerow({k: r.get(k, "") for k in wr.fieldnames}) for r in results]
    lines = []
    for exp in ("exp1", "exp2", "rerun", "handoff"):
        for s in SAMPLES:
            rs = [r for r in results if r["exp"] == exp and r["sample"] == s.key and r["status"] != "WARN"]
            if rs:
                lines.append(f"{exp:8} {s.key:9} {sum(r['status'] == 'PASS' for r in rs):3}/{len(rs):<3} PASS")
    (OUT / "summary.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print("\n".join(lines))


if __name__ == "__main__":
    main()
