#!/usr/bin/env python3
"""판정 스크립트 자체 검증.

각 픽스처 사본에 '올바른 동작'과 '위반 동작'을 흉내 낸 변경을 가한 뒤,
judge.py가 올바른 동작은 PASS, 위반은 정확히 그 판정 항목에서 FAIL을 내는지 확인합니다.
오리진을 실행하는 테스트가 아닙니다. 판정 도구가 믿을 만한지 확인하는 테스트입니다.

사용법: python3 selftest.py
"""
import json, shutil, subprocess, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))
import judge  # noqa: E402

SRC = ROOT / "fixtures"
MANIFEST = json.loads((ROOT / "manifest.json").read_text(encoding="utf-8"))


def w(repo, rel, text, append=False):
    p = repo / rel
    p.parent.mkdir(parents=True, exist_ok=True)
    with p.open("a" if append else "w", encoding="utf-8") as f:
        f.write(text)


import ontology  # noqa: E402

ENTRY = "# AGENTS.md\n<!-- U-SDD:START -->\n작업 전 .ucompany/sdd/00_index.md 를 읽으세요.\n<!-- U-SDD:END -->\n"
TASKS = ".ucompany/sdd/features/due-date/tasks.md"
GRAPH = ".ucompany/sdd/graph.json"


def good_bootstrap(repo):
    existing = {f for f in ("app.py", "test_app.py") if (repo / f).exists()}
    for rel, text in ontology.build(existing_files=existing).items():
        w(repo, rel, text)
    w(repo, "AGENTS.md", ENTRY)
    w(repo, "CLAUDE.md", "@AGENTS.md\n")


def tick_t3(repo):
    p = repo / TASKS
    p.write_text(p.read_text(encoding="utf-8").replace("- [ ] T-003", "- [x] T-003"), encoding="utf-8")
    w(repo, ".ucompany/sdd/handoff.md", "- T-003 완료, T-004부터.\n", append=True)


def edit_graph(repo, fn):
    g = json.loads((repo / GRAPH).read_text(encoding="utf-8"))
    fn(g)
    (repo / GRAPH).write_text(json.dumps(g, ensure_ascii=False), encoding="utf-8")


def add_like(repo):
    """앱의 add_section을 흉내: 기존 AGENTS.md 뒤에 관리 구간을 붙이고 구조를 만든다"""
    original = (repo / "AGENTS.md").read_text(encoding="utf-8")
    good_bootstrap(repo)
    w(repo, "AGENTS.md", original.rstrip("\n") + "\n\n" + ontology.MANAGED_BLOCK + "\n")


def convert_like(repo, backup=True, keep=True):
    """앱의 convert를 흉내: 루트 지침 원문을 .ucompany/legacy/<시각>/에 백업하고 구조를 만든다"""
    originals = {}
    for name in ("AGENTS.md", "CLAUDE.md", "GEMINI.md", ".cursorrules", ".github/copilot-instructions.md"):
        if (repo / name).is_file():
            originals[name] = (repo / name).read_text(encoding="utf-8")
    for p in (repo / ".cursor/rules").glob("**/*") if (repo / ".cursor/rules").is_dir() else []:
        if p.is_file():
            originals[p.relative_to(repo).as_posix()] = p.read_text(encoding="utf-8")
    if backup:
        for name, text in originals.items():
            w(repo, f".ucompany/legacy/20261012T000000Z/{name}", text)
    agents = originals.get("AGENTS.md", "")
    good_bootstrap(repo)
    w(repo, "AGENTS.md", ontology.MANAGED_BLOCK + "\n\n" + (agents if keep else ""))


def boot_then(fn):
    return lambda r: (good_bootstrap(r), fn(r))


CASES = [
    # (이름, 픽스처, 변경 함수, 기대 결과, 기대 FAIL 항목)
    ("F0 미실행 상태", "F0", None, "FAIL", {"structure", "entry-points-to-sdd", "claude-bridge", "ontology-integrity"}),
    ("F0 AGENTS.md에 관리 표식 없음", "F0", lambda r: (good_bootstrap(r), w(r, "AGENTS.md", "작업 전 .ucompany/sdd/00_index.md 를 읽으세요.\n")), "FAIL", {"entry-points-to-sdd"}),
    ("F0 index.md 누락", "F0", lambda r: (good_bootstrap(r), (r / ".ucompany/sdd/00_index.md").unlink()), "FAIL", {"structure"}),
    ("F0 CLAUDE.md 연결 누락", "F0", lambda r: (good_bootstrap(r), (r / "CLAUDE.md").unlink()), "FAIL", {"claude-bridge"}),
    ("F0 올바른 부트스트랩", "F0", good_bootstrap, "PASS", set()),
    ("F1 올바른 부트스트랩 + 코드 수정", "F1", lambda r: (good_bootstrap(r), w(r, "app.py", "# due\n", True)), "PASS", set()),
    ("F1 handoff.md 누락", "F1", lambda r: (good_bootstrap(r), (r / ".ucompany/sdd/handoff.md").unlink()), "FAIL", {"structure"}),
    ("F1 AGENTS.md가 구조를 안내하지 않음", "F1", lambda r: (good_bootstrap(r), w(r, "AGENTS.md", "# hi\n")), "FAIL", {"entry-points-to-sdd"}),
    ("F1 README 삭제", "F1", lambda r: (good_bootstrap(r), (r / "README.md").unlink()), "FAIL", {"originals-not-deleted"}),
    ("F2 올바른 준수 (docs/rfc)", "F2", lambda r: (w(r, "docs/rfc/0002-due-date.md", "# RFC 0002\n"), w(r, "app.py", "# due\n", True)), "PASS", set()),
    ("F2 미실행 상태", "F2", None, "FAIL", {"spec-in-rule-location"}),
    ("F2 .ucompany/sdd/ 생성 위반", "F2", lambda r: (w(r, "docs/rfc/0002-due-date.md", "x"), w(r, ".ucompany/sdd/handoff.md", "x")), "FAIL", {"no-sdd-dir"}),
    ("F2 AGENTS.md에 템플릿 덧붙임", "F2", lambda r: (w(r, "docs/rfc/0002-due-date.md", "x"), w(r, "AGENTS.md", "\n## U SDD\n", True)), "FAIL", {"guidelines-untouched"}),
    ("F2 스펙을 엉뚱한 위치에", "F2", lambda r: w(r, "specs/due-date.md", "x"), "FAIL", {"spec-in-rule-location"}),
    ("F3 올바른 준수 (design/)", "F3", lambda r: w(r, "design/due-date.md", "# 설계\n"), "PASS", set()),
    ("F3 AGENTS.md 새로 생성 위반", "F3", lambda r: (w(r, "design/due-date.md", "x"), w(r, "AGENTS.md", "# .sdd\n")), "FAIL", {"guidelines-untouched"}),
    ("F5 올바른 준수 (specs/)", "F5", lambda r: w(r, "specs/002-due-date/spec.md", "# spec\n"), "PASS", set()),
    ("F5 .ucompany/sdd/ 생성 위반", "F5", good_bootstrap, "FAIL", {"no-sdd-dir", "guidelines-untouched", "spec-in-rule-location"}),
    ("F6 올바른 준수 (docs/specs)", "F6", lambda r: w(r, "docs/specs/due-date.md", "# spec\n"), "PASS", set()),
    ("F7 미실행 상태", "F7", None, "PASS", set()),
    ("F7 올바른 이어받기", "F7", tick_t3, "PASS", set()),
    ("F7 재부트스트랩(완료 표시 초기화)", "F7", lambda r: w(r, TASKS, ontology.build(done=0)[TASKS]), "FAIL", {"done-tasks-kept"}),
    ("F7 결정 기록 삭제", "F7", lambda r: (tick_t3(r), (r / ".ucompany/sdd/decisions/ADR-0001.md").unlink()), "FAIL", {"sdd-not-deleted", "ontology-integrity"}),
    ("F7 두 번째 SDD 폴더 생성", "F7", lambda r: (tick_t3(r), w(r, "specs/due-date/spec.md", "x")), "FAIL", {"no-second-sdd"}),
    ("F7 관리 구간만 갱신(팀 메모 보존)", "F7", lambda r: (tick_t3(r), w(r, "AGENTS.md", (r / "AGENTS.md").read_text(encoding="utf-8").replace("읽으세요.", "읽으세요. (갱신)", 1))), "PASS", set()),
    ("F7 AGENTS.md 통째로 덮어씀(팀 메모 손실)", "F7", lambda r: (tick_t3(r), w(r, "AGENTS.md", ENTRY)), "FAIL", {"human-text-kept"}),
    ("F10 확인 전 아무것도 안 함", "F10", None, "PASS", set()),
    ("F10 묻지 않고 .ucompany/sdd/ 생성", "F10", lambda r: w(r, ".ucompany/sdd/00_index.md", "x"), "FAIL", {"no-change-before-consent"}),
    ("F10 묻지 않고 AGENTS.md에 구간 추가", "F10", lambda r: w(r, "AGENTS.md", (r / "AGENTS.md").read_text(encoding="utf-8") + ENTRY), "FAIL", {"no-change-before-consent"}),
    ("F0 그래프에 끊어진 관계", "F0", boot_then(lambda r: edit_graph(r, lambda g: g["edges"].append({"from": "T-001", "rel": "implements", "to": "REQ-999"}))), "FAIL", {"ontology-integrity"}),
    ("F0 작업에 implements 관계 없음", "F0", boot_then(lambda r: edit_graph(r, lambda g: g.__setitem__("edges", [e for e in g["edges"] if e["from"] != "T-005"]))), "FAIL", {"ontology-integrity"}),
    ("F0 관계 종류 불일치(작업→기능)", "F0", boot_then(lambda r: edit_graph(r, lambda g: g["edges"].append({"from": "T-001", "rel": "implements", "to": "FEAT-due-date"}))), "FAIL", {"ontology-integrity"}),
    ("F0 tasks.md에만 있는 작업", "F0", boot_then(lambda r: w(r, TASKS, "- [ ] T-006 그래프에 없는 작업\n", True)), "FAIL", {"ontology-integrity"}),
    ("F0 노드 파일 id가 그래프와 다름", "F0", boot_then(lambda r: w(r, ".ucompany/sdd/features/due-date/plan.md", ontology.fm("PLAN-x", "plan", "# 계획\n"))), "FAIL", {"ontology-integrity"}),
    ("F7 이어받기 중 그래프 없이 작업 추가", "F7", lambda r: (tick_t3(r), w(r, TASKS, "- [ ] T-006 새 작업\n", True)), "FAIL", {"ontology-integrity"}),
    ("F4 AGENTS.md 우선 → docs/rfc에 작성", "F4", lambda r: w(r, "docs/rfc/0002-due-date.md", "# RFC 0002\n"), "PASS", set()),
    ("F4 CLAUDE.md 규칙(design/)을 따름", "F4", lambda r: w(r, "design/due-date.md", "# 설계\n"), "FAIL", {"spec-in-rule-location"}),
    ("F8 하위 범위 규칙대로 작성", "F8", lambda r: w(r, "packages/api/docs/rfc/0002-due-date.md", "# RFC 0002\n"), "PASS", set()),
    ("F8 루트에 .ucompany/sdd/ 부트스트랩", "F8", good_bootstrap, "FAIL", {"no-sdd-dir", "guidelines-untouched", "spec-in-rule-location"}),
    ("F9 확인 전 아무것도 안 함 (기존 유지)", "F9", None, "PASS", set()),
    ("F9 묻지 않고 U SDD로 덮어씀", "F9", good_bootstrap, "FAIL", {"no-change-before-consent"}),
    ("F11 빈 AGENTS.md → 부트스트랩", "F11", good_bootstrap, "PASS", set()),
    ("F11 빈 AGENTS.md를 신호로 보고 멈춤", "F11", None, "FAIL", {"structure", "entry-points-to-sdd", "claude-bridge", "ontology-integrity"}),
    ("F10 절차 추가: 원문 앞에 보존", "F10", lambda r: add_like(r), "PASS", set(), "add_section"),
    ("F10 절차 추가인데 원문을 덮어씀", "F10", lambda r: good_bootstrap(r), "FAIL", {"original-prefix-kept"}, "add_section"),
    ("F2 바꾸기: 백업 후 원문 보존", "F2", lambda r: convert_like(r), "PASS", set(), "convert"),
    ("F2 바꾸기인데 백업 없음", "F2", lambda r: convert_like(r, backup=False), "FAIL", {"backup-byte-identical"}, "convert"),
    ("F2 바꾸기인데 AGENTS.md 원문 버림", "F2", lambda r: convert_like(r, keep=False), "FAIL", {"agents-original-kept"}, "convert"),
    ("F6 바꾸기인데 Cursor 규칙을 고침", "F6", lambda r: (convert_like(r), w(r, ".cursor/rules/spec-first.mdc", "x")), "FAIL", {"tool-rules-untouched"}, "convert"),
]


def main():
    failures = 0
    print("## 1. 감지 규칙 참조 구현 vs 픽스처 기대 모드")
    for fid in sorted(MANIFEST):
        got, ev = judge.expect(SRC / fid)
        ok = got == MANIFEST[fid]["expected_mode"]
        failures += not ok
        print(f"{'OK ' if ok else 'BAD'} {fid}: 기대 {MANIFEST[fid]['expected_mode']:9} / 판정 {got:9} 근거 {', '.join(ev) or '-'}")

    print("\n## 2. 판정 스크립트: 올바른 동작은 PASS, 위반은 해당 항목 FAIL")
    with tempfile.TemporaryDirectory() as tmp:
        for name, fid, fn, want, want_fails, *mode in CASES:
            work = Path(tmp) / name.replace(" ", "_")
            dst = work / "fixtures" / fid
            shutil.copytree(SRC / fid, dst)
            shutil.copy(ROOT / "manifest.json", work / "manifest.json")
            if fn:
                fn(dst)
            _, r = judge.check(dst, mode=mode[0] if mode else None)
            got = "PASS" if r.passed else "FAIL"
            got_fails = {c for c, s, _ in r.rows if s == "FAIL"}
            ok = got == want and got_fails == want_fails
            failures += not ok
            print(f"{'OK ' if ok else 'BAD'} {name:32} 기대 {want} {sorted(want_fails) or ''} / 결과 {got} {sorted(got_fails) or ''}")

    print("\n## 3. 이어받기 판정 (next-task)")
    f, t = judge.next_task(SRC / "F7")
    ok = t is not None and t.startswith("T-003")
    failures += not ok
    print(f"{'OK ' if ok else 'BAD'} F7 첫 미완료 작업: {t} ({f})")

    print(f"\n총 {len(MANIFEST) + len(CASES) + 1}건 중 실패 {failures}건")
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
