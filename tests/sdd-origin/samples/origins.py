"""SDD 표본 3종을 '테스트 오리진'으로 실행하는 실행기.

각 표본의 실행 절차는 그 표본이 문서화한 순서를 그대로 따른다.
- CLI가 있는 단계(초기화, 기능 폴더 생성)는 실제 CLI를 실행한다 (실측).
- AI 에이전트가 문서를 채우는 단계는, 표본의 명령/스킬이 지시한 파일 위치에
  동일한 내용(마감일 기능, 작업 5개)을 써서 모사한다 (모사).
모든 표본에 같은 기능·같은 작업 5개를 쓰므로 결과 차이는 '구조와 절차'에서만 나온다.
"""
import json, subprocess, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import ontology  # noqa: E402

FILLED = "U-TEST-FILLED"  # 에이전트가 채운 내용인지 확인하는 표식
TASKS = ["T1 Todo 모델에 due 필드 추가", "T2 create()에 due 인자 추가", "T3 overdue(today) 구현",
         "T4 overdue 테스트 추가", "T5 README에 사용법 추가"]
SPEC = f"# 마감일 기능 ({FILLED})\n\n## 요구사항\n- 할 일에 선택적 마감일을 둔다.\n- 마감일이 지난 미완료 항목을 조회한다.\n\n## 수용 기준\n- create(title, due=None)\n- overdue(today)는 지난 미완료 항목만 반환\n"
PLAN = f"# 구현 계획 ({FILLED})\n\n- Todo dataclass에 `due: date | None` 추가\n- `overdue(today)` 함수 추가\n"


def task_md(title="Tasks"):
    return f"# {title} ({FILLED})\n\n" + "".join(f"- [ ] {t}\n" for t in TASKS)


def sh(cwd, *cmd, check=True):
    r = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    if check and r.returncode != 0:
        raise RuntimeError(f"{' '.join(cmd)} 실패: {r.stderr[-500:]}")
    return r


def w(repo, rel, text):
    p = Path(repo) / rel
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text, encoding="utf-8")


def tick(path: Path, n: int):
    """앞에서부터 n개 작업 완료 처리"""
    lines, done = path.read_text(encoding="utf-8").splitlines(keepends=True), 0
    for i, l in enumerate(lines):
        if done < n and l.lstrip().startswith("- [ ]"):
            lines[i] = l.replace("- [ ]", "- [x]", 1); done += 1
    path.write_text("".join(lines), encoding="utf-8")


# ---------------------------------------------------------------- 상: GitHub Spec Kit
class SpecKit:
    key, name, grade = "speckit", "GitHub Spec Kit", "상"
    origin_paths = [".specify", "specs", ".claude"]          # 이 표본이 만드는 경로
    own_markers = [".specify"]                                # 이미 이 표본을 쓰는 프로젝트의 표식
    required = [".specify/memory/constitution.md", ".specify/templates"]
    feature_glob, feature_files = "specs/*", ["spec.md", "plan.md", "tasks.md"]
    tasks_glob = "specs/*/tasks.md"
    filled_files = [".specify/memory/constitution.md", "specs/*/spec.md"]
    tools_supported = 42   # `specify integration list` 실측
    rerun_defined = True

    def init(self, repo):  # 실측: 기존 프로젝트 가이드가 권장하는 명령
        sh(repo, "specify", "init", "--here", "--force", "--non-interactive", "--ignore-agent-tools",
           "--integration", "claude", "--script", "sh")

    def bootstrap(self, repo):
        self.init(repo)
        # 모사: /speckit-constitution — 스킬 설명이 "Create or update"이므로
        # 초기화가 깐 템플릿(자리표시자 포함)이면 채우고, 이미 작성된 헌법이면 원문을 두고 덧붙인다
        con = Path(repo) / ".specify/memory/constitution.md"
        old = con.read_text(encoding="utf-8") if con.exists() else ""
        if not old or "[PROJECT_NAME]" in old or "[PRINCIPLE" in old:
            w(repo, ".specify/memory/constitution.md", f"# Constitution ({FILLED})\n\n- 테스트 우선\n- 외부 의존성 추가 금지\n")
        else:
            w(repo, ".specify/memory/constitution.md", old + f"\n## Amendment ({FILLED})\n- 외부 의존성 추가 금지\n")
        sh(repo, "bash", ".specify/scripts/bash/create-new-feature.sh", "--json", "--short-name", "due-date", "할 일에 마감일 추가")  # 실측
        sh(repo, "bash", ".specify/scripts/bash/setup-plan.sh", "--json")  # 실측
        feat = next(Path(repo).glob("specs/*-due-date"))
        w(feat, "spec.md", SPEC); w(feat, "plan.md", PLAN); w(feat, "tasks.md", task_md())  # 모사: specify/plan/tasks 스킬

    def rerun(self, repo):
        self.init(repo)  # 실측: 같은 초기화 명령 재실행


# ---------------------------------------------------------------- 중: OpenSpec
class OpenSpec:
    key, name, grade = "openspec", "OpenSpec", "중"
    origin_paths = ["openspec", ".claude"]
    own_markers = ["openspec"]
    required = ["openspec/config.yaml", "openspec/specs"]
    feature_glob, feature_files = "openspec/changes/*", ["proposal.md", "design.md", "tasks.md"]
    tasks_glob = "openspec/changes/*/tasks.md"
    filled_files = ["openspec/changes/*/proposal.md"]
    tools_supported = 50   # `openspec init --help`의 --tools 목록 실측
    rerun_defined = True

    def init(self, repo):  # 실측
        sh(repo, "openspec", "init", "--tools", "claude", "--no-animation", "--no-copilot-cloud", ".")

    def bootstrap(self, repo):
        self.init(repo)
        sh(repo, "openspec", "new", "change", "add-due-date")  # 실측
        ch = Path(repo) / "openspec/changes/add-due-date"
        w(ch, "proposal.md", f"# Proposal: 마감일 ({FILLED})\n\n## Why\n마감 관리가 필요하다.\n\n## What Changes\n- due 필드, overdue 조회\n")  # 모사: /opsx:propose
        w(ch, "design.md", PLAN)
        w(ch, "specs/todo/spec.md", "## ADDED Requirements\n\n### Requirement: 마감일\n#### Scenario: 지난 항목 조회\n- WHEN overdue(today)\n- THEN 지난 미완료 항목만 반환\n")
        w(ch, "tasks.md", task_md())

    def rerun(self, repo):
        self.init(repo)  # 실측


# ---------------------------------------------------------------- 하: Harper Reed 3파일 워크플로우
class Harper:
    key, name, grade = "harper", "Harper Reed LLM codegen workflow", "하"
    origin_paths = ["spec.md", "prompt_plan.md", "todo.md"]
    own_markers = ["todo.md", "prompt_plan.md"]
    required = ["spec.md", "prompt_plan.md", "todo.md"]
    feature_glob, feature_files = None, []          # 기능 단위 폴더 개념 없음 (저장소 루트에 3파일)
    tasks_glob = "todo.md"
    filled_files = ["spec.md"]
    tools_supported = None  # 도구 연동 없음 (평문 파일)
    rerun_defined = False   # 이미 파일이 있을 때의 절차가 문서에 없음

    def bootstrap(self, repo):  # 전부 모사: 문서의 3단계(아이디어 → 계획 → todo)를 파일로
        w(repo, "spec.md", SPEC)
        w(repo, "prompt_plan.md", f"# Prompt plan ({FILLED})\n\n" + "".join(f"## Prompt {i+1}\n{t}\n\n" for i, t in enumerate(TASKS)))
        w(repo, "todo.md", task_md("TODO"))

    def rerun(self, repo):
        # 문서에 재실행/기존 파일 확인 절차가 없음 → 문서의 계획 단계를 그대로 다시 수행하면 파일을 새로 쓴다
        self.bootstrap(repo)


for _s in (SpecKit, OpenSpec, Harper):
    _s.index_file, _s.has_detection = None, False


# ---------------------------------------------------------------- 참조: 합의한 규칙을 구현한 U 컴퍼니 참조 오리진
ENTRY_BLOCK = ontology.MANAGED_BLOCK


class URef:
    """감지 계층(정적 스캐너) + 온톨로지 부트스트랩(단일 진입점, 관리 표식, CLAUDE.md 연결, graph.json) + 재실행 규칙.
    테스트 규칙을 그대로 구현했으므로 '기준이 만족 가능한지'를 보여주는 대조군이지, 표본과 겨루는 후보가 아니다."""
    key, name, grade = "uref", "U 컴퍼니 참조 오리진", "참조"
    origin_paths = [".ucompany/sdd", "AGENTS.md", "CLAUDE.md"]
    own_markers = [".ucompany/sdd"]
    required = [".ucompany/sdd/00_index.md", ".ucompany/sdd/graph.json", ".ucompany/sdd/constitution.md", ".ucompany/sdd/handoff.md", ".ucompany/sdd/features", ".ucompany/sdd/decisions"]
    feature_glob, feature_files = ".ucompany/sdd/features/*", ["feature.md", "requirements.md", "plan.md", "tasks.md"]
    tasks_glob = ".ucompany/sdd/features/*/tasks.md"
    filled_files = [".ucompany/sdd/constitution.md", ".ucompany/sdd/features/*/feature.md"]
    tools_supported = None
    rerun_defined = True
    index_file = ".ucompany/sdd/00_index.md"
    has_detection = True

    def write_entry(self, repo):
        """관리 구간만 쓰거나 갱신 — 구간 밖의 사람이 쓴 내용은 그대로 둔다"""
        p = Path(repo) / "AGENTS.md"
        text = p.read_text(encoding="utf-8") if p.exists() else "# AGENTS.md\n\n"
        s, e = "<!-- U-SDD:START -->", "<!-- U-SDD:END -->"
        i, j = text.find(s), text.find(e)
        text = (text[:i] + ENTRY_BLOCK + text[j + len(e):]) if 0 <= i < j else text.rstrip("\n") + "\n\n" + ENTRY_BLOCK + "\n"
        p.write_text(text, encoding="utf-8")
        if not (Path(repo) / "CLAUDE.md").exists():
            w(repo, "CLAUDE.md", "@AGENTS.md\n")

    def bootstrap(self, repo):
        import sys as _s; _s.path.insert(0, str(Path(__file__).resolve().parent.parent))
        import ontology
        existing = {f for f in ("app.py", "test_app.py") if (Path(repo) / f).exists()}
        for rel, text in ontology.build(existing_files=existing).items():
            w(repo, rel, text)
        self.write_entry(repo)

    def rerun(self, repo):
        """이미 .ucompany/sdd/가 있으면 재부트스트랩하지 않고 진입점 관리 구간만 갱신"""
        if (Path(repo) / ".ucompany/sdd").is_dir():
            self.write_entry(repo)
        else:
            self.bootstrap(repo)


SAMPLES = [SpecKit(), OpenSpec(), Harper(), URef()]
