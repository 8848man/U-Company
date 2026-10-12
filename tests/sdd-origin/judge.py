#!/usr/bin/env python3
"""SDD 오리진 결과 판정 스크립트. 판정 기준은 sdd_config.json + manifest.json.

  python3 judge.py expect <repo>                 계획의 감지 규칙으로 본 기대 모드
  python3 judge.py check <repo> [--mode M] [--spec-dir D]
                                                 오리진 실행 후 파일 변경을 판정 (M: existing|bootstrap|continue)
  python3 judge.py next-task <repo>              tasks.md의 첫 미완료 작업 (D 케이스 이어받기 판정용)
  python3 judge.py check-all <fixtures_dir>      모든 픽스처를 manifest 기대 모드로 판정, 표 출력

종료 코드: 0 = 모든 필수 판정 통과, 1 = 하나 이상 FAIL
"""
import argparse, json, re, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
CFG = json.loads((ROOT / "sdd_config.json").read_text(encoding="utf-8"))
TAG = CFG["init_tag"]


def load_manifest(repo: Path):
    for cand in (repo.parent.parent / "manifest.json", ROOT / "manifest.json"):
        if cand.exists():
            return json.loads(cand.read_text(encoding="utf-8")).get(repo.name, {})
    return {}


def run(repo, *args):
    return subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True, text=True).stdout


def changes(repo: Path):
    """태그 대비 작업 트리 변경: {path: 'A'|'M'|'D'}"""
    out = {}
    for line in run(repo, "diff", "--name-status", "--no-renames", TAG).splitlines():
        st, path = line.split("\t", 1)
        out[path] = st[0]
    for path in run(repo, "ls-files", "--others", "--exclude-standard").splitlines():
        out[path] = "A"
    return out


def tag_files(repo: Path):
    return set(run(repo, "ls-tree", "-r", "--name-only", TAG).splitlines())


def tag_content(repo: Path, path: str):
    return run(repo, "show", f"{TAG}:{path}")


def under(path: str, prefix: str):
    prefix = prefix.rstrip("/")
    return path == prefix or path.startswith(prefix + "/")


# ---------- expect: 계획 문서의 감지 규칙 (참조 구현) ----------
def expect(repo: Path):
    """정적 스캐너(scan.py)의 판정을 그대로 사용 — 하위 폴더 신호는 그 범위의 기존 SDD로 본다"""
    from scan import scan
    r = scan(repo)
    return r["root_mode"], r["evidence"]


# ---------- check ----------
class Result:
    def __init__(self):
        self.rows = []

    def add(self, cid, ok, msg, level="must"):
        status = "PASS" if ok else ("FAIL" if level == "must" else "WARN")
        if ok is None:
            status = "INFO"
        self.rows.append((cid, status, msg))

    @property
    def passed(self):
        return all(s != "FAIL" for _, s, _ in self.rows)


def check_existing(repo, ch, spec_dir, r):
    sdd = CFG["sdd_dir"]
    created = [p for p in ch if under(p, sdd)]
    r.add("no-sdd-dir", not created, f"{sdd}/ 생성 없음" if not created else f"{sdd}/ 생성됨: {created[:3]}")
    touched = [f"{p}({s})" for p, s in ch.items() if any(under(p, g) for g in CFG["guideline_paths"])]
    r.add("guidelines-untouched", not touched, "지침 파일 변경 없음" if not touched else f"지침 파일 변경: {touched}")
    if spec_dir:
        specs = [p for p, s in ch.items() if s in "AM" and under(p, spec_dir) and p.endswith((".md", ".mdc"))]
        r.add("spec-in-rule-location", bool(specs),
              f"규칙 위치({spec_dir}/)에 스펙 작성: {specs}" if specs else f"{spec_dir}/ 에 새 스펙 없음")
    else:
        r.add("spec-in-rule-location", None, "스펙 위치 규칙 없음 또는 수동 판정")


def managed_block(text):
    """(관리 구간 내용, 관리 구간 밖 내용) — 표식이 없으면 (None, 전체)"""
    s, e = CFG["managed_start"], CFG["managed_end"]
    i, j = text.find(s), text.find(e)
    if i < 0 or j < i:
        return None, text
    return text[i + len(s):j], text[:i] + text[j + len(e):]


def entry_ok(repo: Path):
    """단일 진입점: 루트 AGENTS.md에 관리 구간이 있고, 그 안에서 인덱스 파일을 가리킨다"""
    entry = repo / CFG["entry_file"]
    if not entry.is_file():
        return False, f"{CFG['entry_file']} 없음"
    block, _ = managed_block(entry.read_text(encoding="utf-8"))
    if block is None:
        return False, f"{CFG['entry_file']}에 관리 표식({CFG['managed_start']}) 없음"
    if CFG["index_file"] not in block:
        return False, f"관리 구간이 {CFG['index_file']}를 가리키지 않음"
    return True, f"{CFG['entry_file']} 관리 구간이 {CFG['index_file']} 안내"


def bridge_ok(repo: Path):
    """Claude Code가 기본으로 읽는 CLAUDE.md가 AGENTS.md를 불러오는지"""
    b = CFG.get("claude_bridge")
    if not b:
        return None, "연결 파일 검사 안 함"
    p = repo / b["file"]
    ok = p.is_file() and b["must_reference"] in p.read_text(encoding="utf-8")
    return ok, (f"{b['file']}가 {b['must_reference']}를 불러옴" if ok else f"{b['file']} 없음 또는 {b['must_reference']} 참조 없음")


def check_bootstrap(repo, ch, r):
    sdd = CFG["sdd_dir"]
    missing = [f for f in CFG["bootstrap_required_files"] if not (repo / f).is_file()]
    missing += [d + "/" for d in CFG["bootstrap_required_dirs"] if not (repo / d).is_dir()]
    r.add("structure", not missing, "필수 파일·폴더 모두 존재" if not missing else f"누락: {missing}")
    ok, msg = entry_ok(repo)
    r.add("entry-points-to-sdd", ok, msg)
    ok, msg = bridge_ok(repo)
    r.add("claude-bridge", ok, msg)
    tasks = sorted(repo.glob(CFG["tasks_glob"]))
    has_box = any(re.search(r"^\s*- \[[ xX]\]", t.read_text(encoding="utf-8"), re.M) for t in tasks)
    r.add("tasks-checkbox", has_box, f"체크박스 작업 목록: {[str(t.relative_to(repo)) for t in tasks]}" if has_box
          else "체크박스 작업 목록 없음", level="should")
    probs = ontology_problems(repo)
    r.add("ontology-integrity", not probs, "그래프 무결성 통과 (노드 파일·앵커·관계 종류·필수 관계·작업 1:1)" if not probs
          else f"무결성 문제 {len(probs)}건: {probs[:3]}")


def check_add_section(repo, ch, r):
    """절차 추가: 구조는 부트스트랩과 같고, 기존 지침 원문은 그대로 앞에 남아야 한다"""
    check_bootstrap(repo, ch, r)
    kept, lost = [], []
    for name in ("AGENTS.md", "CLAUDE.md"):
        if name not in tag_files(repo):
            continue
        before = tag_content(repo, name).rstrip("\n")
        now = (repo / name).read_text(encoding="utf-8") if (repo / name).is_file() else ""
        if not before.strip():
            continue
        (kept if now.startswith(before) else lost).append(name)
    r.add("original-prefix-kept", not lost, f"기존 내용이 앞에 그대로: {kept}" if not lost else f"기존 내용이 바뀜: {lost}")


LEGACY = ".ucompany/legacy"
TOOL_RULES = ("GEMINI.md", ".cursorrules", ".github/copilot-instructions.md", ".cursor/rules")


def check_convert(repo, ch, r):
    """U 컴퍼니 구조로 바꾸기: 원문 백업이 먼저, AGENTS.md 원문 보존, 다른 툴 규칙·외부 SDD는 그대로"""
    check_bootstrap(repo, ch, r)
    originals = [p for p in tag_files(repo) if any(under(p, g) for g in CFG["guideline_paths"]) and tag_content(repo, p).strip()]
    backups = {p.relative_to(repo / LEGACY).as_posix().split("/", 1)[1]: p
               for p in (repo / LEGACY).glob("*/**/*") if p.is_file()} if (repo / LEGACY).is_dir() else {}
    missing = [p for p in originals if p not in backups or backups[p].read_bytes() != run(repo, "show", f"{TAG}:{p}").encode("utf-8")]
    r.add("backup-byte-identical", not missing, f"원문 {len(originals)}개 백업 일치" if not missing else f"백업 없음·불일치: {missing}")
    if "AGENTS.md" in originals:
        before = tag_content(repo, "AGENTS.md").strip()
        block, outside = managed_block((repo / "AGENTS.md").read_text(encoding="utf-8"))
        ok = block is not None and before in outside
        r.add("agents-original-kept", ok, "AGENTS.md 원문이 관리 구간 밖에 남음" if ok else "AGENTS.md 원문이 사라짐")
    touched = [p for p, s in ch.items() if s in "MD" and (any(under(p, g) for g in TOOL_RULES) or any(under(p, d) for d in CFG["signals"]["external_sdd_dirs"]))]
    r.add("tool-rules-untouched", not touched, "다른 툴 규칙·외부 SDD 그대로" if not touched else f"바뀜: {touched}")


def check_continue(repo, ch, r):
    sdd = CFG["sdd_dir"]
    deleted = [p for p, s in ch.items() if s == "D" and under(p, sdd)]
    r.add("sdd-not-deleted", not deleted, f"{sdd}/ 파일 삭제 없음" if not deleted else f"삭제됨: {deleted}")
    regressed = []
    for p in tag_files(repo):
        if under(p, sdd) and p.endswith("tasks.md") and (repo / p).is_file():
            before = set(re.findall(r"^\s*- \[[xX]\]\s*(.+)$", tag_content(repo, p), re.M))
            after_text = (repo / p).read_text(encoding="utf-8")
            regressed += [f"{p}: {t}" for t in before if not re.search(r"^\s*- \[[xX]\]\s*" + re.escape(t) + r"\s*$", after_text, re.M)]
    r.add("done-tasks-kept", not regressed, "완료 작업 유지(재부트스트랩 없음)" if not regressed else f"완료 표시 사라짐: {regressed}")
    other = [p for p, s in ch.items() if s == "A" and not under(p, sdd)
             and (any(under(p, d) for d in CFG["signals"]["external_sdd_dirs"]) or re.match(r"^[^/]*sdd[^/]*/", p, re.I))]
    r.add("no-second-sdd", not other, "다른 SDD 폴더 생성 없음" if not other else f"추가 SDD 폴더: {other}")
    ef = CFG["entry_file"]
    if ef in tag_files(repo):
        _, outside_before = managed_block(tag_content(repo, ef))
        now = (repo / ef).read_text(encoding="utf-8") if (repo / ef).is_file() else ""
        block_now, _ = managed_block(now)
        lost = [l for l in outside_before.splitlines() if l.strip() and l not in now]
        r.add("human-text-kept", not lost and block_now is not None,
              "관리 구간 유지, 사람이 쓴 내용 보존" if not lost and block_now is not None
              else (f"사람이 쓴 내용 손실: {lost[:2]}" if lost else "관리 표식이 사라짐"))
    progressed = [p for p, s in ch.items() if under(p, sdd) and s in "AM"]
    r.add("progress-recorded", bool(progressed), f"진행 기록 갱신: {progressed}" if progressed else f"{sdd}/ 갱신 없음", level="should")
    if (repo / CFG["ontology"]["graph_file"]).is_file():
        probs = ontology_problems(repo)
        r.add("ontology-integrity", not probs, "작업 후에도 그래프 무결성 유지" if not probs
              else f"무결성 문제 {len(probs)}건: {probs[:3]}")


def ontology_problems(repo: Path):
    from ontology import validate
    return validate(repo)


def check(repo: Path, mode=None, spec_dir=None):
    meta = load_manifest(repo)
    mode = mode or meta.get("expected_mode")
    spec_dir = spec_dir if spec_dir is not None else meta.get("spec_dir")
    r = Result()
    ch = changes(repo)
    originals = tag_files(repo)
    del_orig = [p for p, s in ch.items() if s == "D" and p in originals and not under(p, CFG["sdd_dir"])]
    r.add("originals-not-deleted", not del_orig, "기존 파일 삭제 없음" if not del_orig else f"삭제됨: {del_orig}")
    mod = [p for p, s in ch.items() if s == "M"]
    r.add("modified-files", None, f"수정된 기존 파일: {mod or '없음'}")
    if mode == "existing":
        check_existing(repo, ch, spec_dir, r)
    elif mode == "bootstrap":
        check_bootstrap(repo, ch, r)
    elif mode == "continue":
        check_continue(repo, ch, r)
    elif mode == "add_section":
        check_add_section(repo, ch, r)
    elif mode == "convert":
        check_convert(repo, ch, r)
    elif mode == "ask":   # 사용자 확인 전에는 아무것도 만들거나 고치지 않아야 함
        made = sorted(f"{p}({s})" for p, s in ch.items())
        r.add("no-change-before-consent", not made, "확인 전 변경 없음" if not made else f"확인 전에 변경: {made[:4]}")
        r.add("asked-user", None, f"수동 판정: {meta.get('manual', '사용자에게 확인했는지 대화로 확인')}")
    else:
        r.add("mode", None, f"기대 모드 미정({mode}) — 수동 판정: {meta.get('manual', '')}")
    if mode in ("existing", "ask"):   # 결정(2026-10-12): 기존 지침·SDD가 있으면 항상 덮어쓰기 여부를 묻는다
        r.add("u-sdd-overwrite-asked", None, "수동 판정: 'U 컴퍼니 부트스트랩 SDD로 덮어씌울까요?'를 물었는지, 답 전에는 아무것도 바꾸지 않았는지")
    return mode, r


def next_task(repo: Path):
    """그래프가 있으면 focus 기능을 따라가고, 없으면 tasks.md를 순서대로 본다"""
    if (repo / CFG["ontology"]["graph_file"]).is_file():
        from ontology import next_task as onto_next
        tid, desc, _ = onto_next(repo)
        if tid:
            return CFG["ontology"]["graph_file"] + " → focus", f"{tid} {desc}"
    for tasks in sorted(repo.glob(CFG["tasks_glob"])):
        m = re.search(r"^\s*- \[ \]\s*(.+)$", tasks.read_text(encoding="utf-8"), re.M)
        if m:
            return str(tasks.relative_to(repo)), m.group(1).strip()
    return None, None


def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("expect"); p.add_argument("repo")
    p = sub.add_parser("check"); p.add_argument("repo"); p.add_argument("--mode"); p.add_argument("--spec-dir")
    p = sub.add_parser("next-task"); p.add_argument("repo")
    p = sub.add_parser("check-all"); p.add_argument("fixtures")
    a = ap.parse_args()

    if a.cmd == "expect":
        mode, ev = expect(Path(a.repo))
        print(f"{mode}\t{', '.join(ev) or '-'}")
    elif a.cmd == "check":
        mode, r = check(Path(a.repo), a.mode, a.spec_dir)
        print(f"# {a.repo}  (모드: {mode})")
        for cid, st, msg in r.rows:
            print(f"{st:5} {cid:24} {msg}")
        print("RESULT:", "PASS" if r.passed else "FAIL")
        sys.exit(0 if r.passed else 1)
    elif a.cmd == "next-task":
        f, t = next_task(Path(a.repo))
        print(f"{f}\t{t}" if t else "미완료 작업 없음")
    elif a.cmd == "check-all":
        allok = True
        for repo in sorted(p for p in Path(a.fixtures).iterdir() if (p / ".git").exists()):
            mode, r = check(repo)
            fails = [c for c, s, _ in r.rows if s == "FAIL"]
            allok &= r.passed
            print(f"{repo.name}\t{mode}\t{'PASS' if r.passed else 'FAIL'}\t{','.join(fails) or '-'}")
        sys.exit(0 if allok else 1)


if __name__ == "__main__":
    main()
