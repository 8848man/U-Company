#!/usr/bin/env python3
"""SDD 오리진 테스트용 픽스처 저장소 F0~F11 생성.

사용법: python3 make_fixtures.py [출력폴더]   (기본: ./fixtures)
각 저장소는 'fixture-init' 태그로 초기 상태가 고정됩니다.
초기화: git -C fixtures/F2 reset --hard fixture-init && git -C fixtures/F2 clean -fdx
픽스처 메타데이터(manifest.json)는 저장소 밖에 둡니다. 오리진이 읽지 못하게 하기 위함입니다.
"""
import json, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
OUT = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "fixtures"
TAG = json.loads((ROOT / "sdd_config.json").read_text(encoding="utf-8"))["init_tag"]

APP = {
    "app.py": '''"""Minimal TODO API (sample app for SDD tests)."""
from dataclasses import dataclass, field
from itertools import count

_ids = count(1)


@dataclass
class Todo:
    title: str
    done: bool = False
    id: int = field(default_factory=lambda: next(_ids))


TODOS: dict[int, Todo] = {}


def create(title: str) -> Todo:
    todo = Todo(title)
    TODOS[todo.id] = todo
    return todo


def complete(todo_id: int) -> Todo:
    TODOS[todo_id].done = True
    return TODOS[todo_id]


def list_all() -> list[Todo]:
    return list(TODOS.values())
''',
    "test_app.py": '''import app


def test_create_and_complete():
    t = app.create("write spec")
    assert not t.done
    assert app.complete(t.id).done
''',
}
README = "# todo-api\n\n할 일을 만들고 완료 처리하는 작은 API입니다.\n\n```\npython -m pytest\n```\n"

AGENTS_RFC = """# AGENTS.md

이 저장소는 RFC 기반 스펙 주도 개발을 따릅니다. 모든 AI 에이전트는 아래 규칙을 지켜야 합니다.

## 워크플로우
1. 기능 작업 전 `docs/rfc/NNNN-<기능명>.md` 에 RFC를 먼저 작성한다. 번호는 기존 최대 번호 + 1.
2. RFC에는 배경, 요구사항, 수용 기준, 테스트 계획 섹션을 둔다.
3. RFC 작성 후 구현하고, 테스트를 추가한다.
4. 다른 스펙 폴더(.ucompany/sdd/, specs/ 등)를 만들지 않는다.

## 커밋 규칙
- 커밋 메시지는 `feat(rfc-NNNN): 요약` 형식.
"""

CLAUDE_DESIGN = """# CLAUDE.md

이 프로젝트의 개발 규칙입니다.

## 스펙 작성
- 새 기능은 `design/<기능명>.md` 에 설계 문서를 먼저 쓴다.
- 설계 문서에는 목표, 비목표, API 변경, 수용 기준을 적는다.

## 브랜치와 커밋
- 브랜치 이름은 `feature/<기능명>`.
- 커밋 메시지는 Conventional Commits 형식(`feat: ...`, `fix: ...`).
- 기존 지침 파일은 사람의 승인 없이 수정하지 않는다.
"""

CLAUDE_CONFLICT = """# CLAUDE.md

## 스펙 작성
- 새 기능은 `design/<기능명>.md` 에 설계 문서를 먼저 쓴다. (AGENTS.md의 RFC 위치와 다름)

## 커밋
- 커밋 메시지는 Conventional Commits 형식(`feat: ...`).
"""

AGENTS_STYLE = """# AGENTS.md

이 저장소에서 코드를 작성하는 모든 AI 에이전트를 위한 규칙입니다.

## 코드 스타일
- Python 3.11, 들여쓰기 4칸, 한 줄 최대 100자.
- 함수와 변수 이름은 snake_case, 클래스는 PascalCase.
- 공개 함수에는 docstring을 단다.

## 테스트
- 새 코드에는 pytest 테스트를 함께 추가한다.
- `python -m pytest`가 통과해야 커밋한다.
"""

CURSOR_RULE = """---
description: 스펙 우선 개발 규칙
alwaysApply: true
---

- 기능 작업 전 `docs/specs/<기능명>.md` 에 스펙(요구사항, 수용 기준)을 작성한다.
- 스펙이 승인되기 전에는 구현 코드를 쓰지 않는다.
- 작업 목록은 스펙 파일 하단의 체크박스로 관리한다.
"""

SPECKIT = {
    ".specify/memory/constitution.md": "# Constitution\n\n- 모든 기능은 specs/NNN-<name>/ 아래 spec.md, plan.md, tasks.md를 갖는다.\n- 테스트 우선.\n",
    "specs/001-todo-basics/spec.md": "# Feature: TODO basics\n\n## User stories\n- 사용자는 할 일을 만들 수 있다.\n- 사용자는 할 일을 완료 처리할 수 있다.\n\n## Acceptance\n- create/complete 테스트 통과\n",
    "specs/001-todo-basics/plan.md": "# Plan\n\n- dataclass 기반 인메모리 저장소\n",
    "specs/001-todo-basics/tasks.md": "# Tasks\n\n- [x] T001 Todo 모델\n- [x] T002 create/complete\n",
}

import ontology  # noqa: E402  (결정 3: 관계 기반 인덱싱 온톨로지 v0)

ENTRY_BLOCK = ontology.MANAGED_BLOCK
SDD_IN_PROGRESS = {
    **ontology.build(existing_files={"app.py", "test_app.py"}, done=2),   # T-001, T-002 완료 상태
    "AGENTS.md": "# AGENTS.md\n\n" + ENTRY_BLOCK + "\n\n## 팀 메모 (사람이 추가)\n- 배포는 금요일에 하지 않는다.\n",
    "CLAUDE.md": "@AGENTS.md\n",
}

FIXTURES = {
    "F0": {"desc": "빈 저장소", "files": {}, "expected_mode": "bootstrap"},
    "F1": {"desc": "일반 프로젝트, SDD 신호 없음", "files": {**APP, "README.md": README}, "expected_mode": "bootstrap"},
    "F2": {"desc": "AGENTS.md만 (docs/rfc 규칙)", "files": {**APP, "README.md": README, "AGENTS.md": AGENTS_RFC, "docs/rfc/0001-todo-basics.md": "# RFC 0001: TODO basics\n\n## 배경\n최초 기능.\n"},
           "expected_mode": "existing", "spec_dir": "docs/rfc"},
    "F3": {"desc": "CLAUDE.md만 (design/ 규칙)", "files": {**APP, "README.md": README, "CLAUDE.md": CLAUDE_DESIGN},
           "expected_mode": "existing", "spec_dir": "design"},
    "F4": {"desc": "AGENTS.md + CLAUDE.md 충돌 (결정 4: AGENTS.md 우선)", "files": {**APP, "README.md": README, "AGENTS.md": AGENTS_RFC, "CLAUDE.md": CLAUDE_CONFLICT},
           "expected_mode": "existing", "spec_dir": "docs/rfc", "manual": "AGENTS.md 우선이므로 docs/rfc에 작성. 충돌 사실을 사용자에게 알렸는지는 대화로 판정(B4)"},
    "F5": {"desc": "외부 SDD 도구(spec-kit 형) specs/ + .specify/", "files": {**APP, "README.md": README, **SPECKIT},
           "expected_mode": "existing", "spec_dir": "specs"},
    "F6": {"desc": "다른 툴 지침만 (.cursor/rules)", "files": {**APP, "README.md": README, ".cursor/rules/spec-first.mdc": CURSOR_RULE},
           "expected_mode": "existing", "spec_dir": "docs/specs"},
    "F7": {"desc": "U 컴퍼니 SDD 진행 중", "files": {**APP, "README.md": README, **SDD_IN_PROGRESS}, "expected_mode": "continue"},
    "F8": {"desc": "하위 폴더에만 신호 (모노레포)", "files": {"packages/api/" + k: v for k, v in APP.items()} | {"README.md": README, "packages/api/AGENTS.md": AGENTS_RFC.replace("docs/rfc", "packages/api/docs/rfc")},
           "expected_mode": "existing", "spec_dir": "packages/api/docs/rfc", "manual": "결정 2: 하위 폴더 신호는 그 범위의 기존 SDD. 루트에 .ucompany/sdd/를 만들지 않음(A5)"},
    "F9": {"desc": "내용이 조금 있는 AGENTS.md", "files": {**APP, "README.md": README, "AGENTS.md": "# AGENTS\n"},
           "expected_mode": "ask", "manual": "결정: 내용이 조금이라도 있으면 기존 파일 유지, 덮어쓰기 여부를 물었는지 대화로 판정(A6)"},
    "F10": {"desc": "코드 스타일만 있는 AGENTS.md (스펙 워크플로우 없음)", "files": {**APP, "README.md": README, "AGENTS.md": AGENTS_STYLE},
            "expected_mode": "ask", "manual": "SDD 구간을 추가할지 사용자에게 확인했는지 대화로 판정(A7)"},
    "F11": {"desc": "빈 AGENTS.md (0바이트)", "files": {**APP, "README.md": README, "AGENTS.md": ""},
            "expected_mode": "bootstrap", "manual": "결정: 빈 지침은 없는 것으로 간주 → 부트스트랩(A8)"},
}


def git(repo, *args):
    subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True)


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    manifest = {}
    for fid, spec in FIXTURES.items():
        repo = OUT / fid
        repo.mkdir(parents=True)
        for rel, content in spec["files"].items():
            p = repo / rel
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(content, encoding="utf-8")
        git(repo, "init", "-q")
        git(repo, "add", "-A")
        msg = (f"fixture {fid}: {spec['desc']}\n\n"
               "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>\n"
               "Claude-Session: https://claude.ai/code/session_01XnaV3vNZfKwYVhDtmtD5T7")
        git(repo, "commit", "-q", "--allow-empty", "-m", msg)
        git(repo, "tag", TAG)
        manifest[fid] = {k: v for k, v in spec.items() if k != "files"}
        print(f"{fid}  {spec['desc']}  ({len(spec['files'])} files)")
    (OUT.parent / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
