#!/usr/bin/env python3
"""U 컴퍼니 SDD v0: 관계 기반 인덱싱 온톨로지 (결정 3, 구조는 확정 아님).

- 노드: 헌법, 기능, 요구사항, 계획, 작업, 결정, 코드 구성요소. 각 노드는 파일(또는 파일 안 #앵커)이다.
- 관계: graph.json의 edges. AI는 graph.json만 먼저 읽고, 필요한 노드만 따라가 연다.
- 진입: 루트 AGENTS.md(관리 구간) → .ucompany/sdd/00_index.md(읽는 법) → .ucompany/sdd/graph.json(지도)

  python3 ontology.py validate <repo>     무결성 검사
  python3 ontology.py next <repo>         다음 작업과 그것을 하려고 읽어야 할 파일(바이트) 계산
"""
import json, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
CFG = json.loads((ROOT / "sdd_config.json").read_text(encoding="utf-8"))
ONT = CFG["ontology"]
FILLED = "U-TEST-FILLED"

# 앱(src-tauri/src/sdd_origin.rs의 managed_block)과 글자 하나까지 같아야 한다. app_conformance.py가 대조한다.
MANAGED_BLOCK = (
    "<!-- U-SDD:START -->\n## U 컴퍼니 SDD\n이 프로젝트는 U 컴퍼니 SDD를 사용합니다.\n"
    "1. 작업 전에 `.ucompany/sdd/00_index.md`를 읽으세요. 진행 중인 기능, 다음 작업, 먼저 읽을 파일이 적혀 있습니다.\n"
    "2. 전부 읽지 말고 `.ucompany/sdd/graph.json`에서 지금 작업과 연결된 문서만 여세요.\n"
    "3. 단계(스펙 → 계획 → 작업 → 구현)를 넘어가기 전에 사용자에게 확인받으세요.\n"
    "4. 작업 후 작업 목록 체크박스와 `00_index.md`, `graph.json`, `handoff.md`를 갱신하세요.\n"
    "이 구간은 U 컴퍼니 앱이 관리합니다. 구간 밖의 내용은 앱이 바꾸지 않습니다.\n<!-- U-SDD:END -->")

REQS = [("REQ-001", "할 일에 선택적 마감일(date)을 둔다."), ("REQ-002", "마감일이 지난 미완료 할 일을 조회한다.")]
TASKS = [("T-001", "Todo 모델에 due 필드 추가", "REQ-001", "app"), ("T-002", "create()에 due 인자 추가", "REQ-001", "app"),
         ("T-003", "overdue(today) 구현", "REQ-002", "app"), ("T-004", "overdue 테스트 추가", "REQ-002", "test"),
         ("T-005", "README에 사용법 추가", "REQ-001", None)]
COMPONENTS = {"app": ("CMP-app", "app.py"), "test": ("CMP-test", "test_app.py")}


def fm(id_, type_, body):
    return f"---\nid: {id_}\ntype: {type_}\n---\n\n{body}"


def build(existing_files=(), feature="due-date", done=0):
    """부트스트랩 산출물 {경로: 내용}. existing_files: 저장소에 이미 있는 코드 파일(구성요소 노드로 연결)"""
    fdir = f".ucompany/sdd/features/{feature}"
    fid, pid = f"FEAT-{feature}", f"PLAN-{feature}"
    files = {
        ".ucompany/sdd/constitution.md": fm("CON-1", "constitution", f"# 프로젝트 원칙 ({FILLED})\n\n- 테스트 우선\n- 외부 의존성 추가 금지\n"),
        f"{fdir}/feature.md": fm(fid, "feature", f"# 기능: 마감일 ({FILLED})\n\n할 일에 마감일을 두고 지난 항목을 찾는다.\n"),
        f"{fdir}/requirements.md": "# 요구사항\n\n" + "".join(f"## {r}\n{t}\n\n" for r, t in REQS),
        f"{fdir}/plan.md": fm(pid, "plan", f"# 구현 계획 ({FILLED})\n\n- Todo에 `due: date | None` 추가\n- `overdue(today)` 함수 추가\n- 저장소는 인메모리 유지 (ADR-0001)\n"),
        f"{fdir}/tasks.md": "# 작업\n\n" + "".join(
            f"- [{'x' if i < done else ' '}] {t} {d} (implements: {r})\n" for i, (t, d, r, _) in enumerate(TASKS)),
        ".ucompany/sdd/decisions/ADR-0001.md": fm("ADR-0001", "decision", "# ADR-0001 인메모리 저장소 유지\n\n상태: 채택\n"),
        ".ucompany/sdd/handoff.md": "# Handoff\n\n- 현재 상태와 막힌 점을 적는다. 다음 작업은 graph.json과 tasks.md가 기준.\n",
    }
    nodes = [{"id": "CON-1", "type": "constitution", "path": ".ucompany/sdd/constitution.md"},
             {"id": fid, "type": "feature", "path": f"{fdir}/feature.md", "status": "in-progress"},
             {"id": pid, "type": "plan", "path": f"{fdir}/plan.md"},
             {"id": "ADR-0001", "type": "decision", "path": ".ucompany/sdd/decisions/ADR-0001.md"}]
    nodes += [{"id": r, "type": "requirement", "path": f"{fdir}/requirements.md#{r}"} for r, _ in REQS]
    nodes += [{"id": t, "type": "task", "path": f"{fdir}/tasks.md#{t}"} for t, *_ in TASKS]
    edges = [{"from": fid, "rel": "governed_by", "to": "CON-1"}, {"from": fid, "rel": "planned_by", "to": pid},
             {"from": pid, "rel": "decided_by", "to": "ADR-0001"}]
    edges += [{"from": fid, "rel": "has_requirement", "to": r} for r, _ in REQS]
    edges += [{"from": t, "rel": "implements", "to": r} for t, _, r, _ in TASKS]
    for key, (cid, path) in COMPONENTS.items():
        if path in existing_files:
            nodes.append({"id": cid, "type": "component", "path": path})
            edges += [{"from": t, "rel": "touches", "to": cid} for t, _, _, c in TASKS if c == key]
    files[".ucompany/sdd/graph.json"] = json.dumps({"version": 0, "focus": fid, "nodes": nodes, "edges": edges}, ensure_ascii=False, indent=1)
    files[".ucompany/sdd/00_index.md"] = (
        "# SDD Index\n\n이 프로젝트의 스펙은 관계 그래프로 정리돼 있다. 전부 읽지 말고 아래 순서로 필요한 것만 연다.\n\n"
        "1. `.ucompany/sdd/graph.json`을 읽는다. `focus`가 지금 진행 중인 기능이다.\n"
        "2. focus 기능의 `tasks.md`에서 첫 미완료 작업을 찾는다.\n"
        "3. 그 작업의 `implements` 요구사항과 `touches` 코드만 연다. 원칙은 `governed_by`로 연결된 헌법을 본다.\n"
        "4. 작업 후 tasks.md 체크박스와 graph.json(새 노드·관계), handoff.md를 갱신한다.\n\n"
        f"- 현재 focus: {fid}\n")
    return files


def _load(repo):
    return json.loads((Path(repo) / ONT["graph_file"]).read_text(encoding="utf-8"))


def validate(repo):
    """무결성 문제 목록 (빈 목록 = 통과)"""
    repo, probs = Path(repo), []
    try:
        g = _load(repo)
    except (OSError, json.JSONDecodeError) as e:
        return [f"graph.json을 읽을 수 없음: {e}"]
    nodes = {n.get("id"): n for n in g.get("nodes", [])}
    if len(nodes) != len(g.get("nodes", [])):
        probs.append("중복된 노드 id")
    for need in ONT["required_types"]:
        if not any(n.get("type") == need for n in nodes.values()):
            probs.append(f"필수 노드 종류 없음: {need}")
    for nid, n in nodes.items():
        if n.get("type") not in ONT["node_types"]:
            probs.append(f"{nid}: 알 수 없는 종류 {n.get('type')}")
        path, _, anchor = n.get("path", "").partition("#")
        f = repo / path
        if not f.is_file():
            probs.append(f"{nid}: 파일 없음 {path}"); continue
        if n.get("type") == "component":
            continue
        text = f.read_text(encoding="utf-8")
        if anchor and anchor not in text:
            probs.append(f"{nid}: {path} 안에 앵커 {anchor} 없음")
        if not anchor and f"id: {nid}" not in text:
            probs.append(f"{nid}: {path}의 id 표기가 그래프와 다름")
    outgoing, incoming = {}, {}
    for e in g.get("edges", []):
        a, rel, b = e.get("from"), e.get("rel"), e.get("to")
        if a not in nodes or b not in nodes:
            probs.append(f"끊어진 관계: {a} -{rel}-> {b}"); continue
        sig = ONT["relations"].get(rel)
        if not sig:
            probs.append(f"알 수 없는 관계: {rel}"); continue
        if [nodes[a]["type"], nodes[b]["type"]] != sig:
            probs.append(f"관계 종류 불일치: {a}({nodes[a]['type']}) -{rel}-> {b}({nodes[b]['type']})")
        outgoing.setdefault(a, set()).add(rel); incoming.setdefault(b, set()).add(rel)
    for nid, n in nodes.items():
        for rel in ONT["must_have_outgoing"].get(n["type"], []):
            if rel not in outgoing.get(nid, set()):
                probs.append(f"{nid}: 필수 관계 {rel} 없음")
        for rel in ONT["must_have_incoming"].get(n["type"], []):
            if rel not in incoming.get(nid, set()):
                probs.append(f"{nid}: 들어오는 관계 {rel} 없음 (고아 노드)")
    for tf in repo.glob(CFG["tasks_glob"]):            # tasks.md의 작업과 그래프 노드가 1:1인지
        ids = set(re.findall(r"^\s*- \[[ xX]\]\s*(T-\d+)", tf.read_text(encoding="utf-8"), re.M))
        rel = str(tf.relative_to(repo))
        graph_ids = {i for i, n in nodes.items() if n["type"] == "task" and n["path"].startswith(rel)}
        for t in ids - graph_ids:
            probs.append(f"{t}: tasks.md에 있지만 그래프에 없음")
        for t in graph_ids - ids:
            probs.append(f"{t}: 그래프에 있지만 tasks.md에 없음")
    has_features = any(n.get("type") == "feature" for n in nodes.values())
    if g.get("focus") is None and has_features:
        probs.append("기능이 있는데 focus가 비어 있음")
    if g.get("focus") is not None and (g.get("focus") not in nodes or nodes[g["focus"]]["type"] != "feature"):
        probs.append("focus가 기능 노드를 가리키지 않음")
    return probs


def next_task(repo):
    """그래프를 따라 다음 작업과, 그 작업에 필요한 파일만 고른다 → (작업 id, 설명, 읽을 파일들)"""
    repo = Path(repo)
    g = _load(repo)
    nodes = {n["id"]: n for n in g["nodes"]}
    feat_dir = str(Path(nodes[g["focus"]]["path"]).parent)
    tasks = repo / feat_dir / "tasks.md"
    m = re.search(r"^\s*- \[ \]\s*(T-\d+)\s+(.+)$", tasks.read_text(encoding="utf-8"), re.M)
    if not m:
        return None, None, []
    tid = m.group(1)
    reads = {CFG["index_file"], ONT["graph_file"], str(tasks.relative_to(repo))}
    for e in g["edges"]:
        if e["from"] == tid and e["rel"] in ("implements", "touches"):
            reads.add(nodes[e["to"]]["path"].partition("#")[0])
        if e["from"] == g["focus"] and e["rel"] == "governed_by":
            reads.add(nodes[e["to"]]["path"])
    return tid, m.group(2).strip(), sorted(reads)


def context_saving(repo):
    """다음 작업을 위해 읽는 바이트 vs .ucompany/sdd 전체 + 코드 전체를 읽는 바이트"""
    repo = Path(repo)
    tid, _, reads = next_task(repo)
    need = sum((repo / p).stat().st_size for p in reads if (repo / p).is_file())
    allb = sum(p.stat().st_size for p in repo.rglob("*") if p.is_file() and ".git" not in p.parts)
    return tid, need, allb


if __name__ == "__main__":
    cmd, repo = sys.argv[1], sys.argv[2]
    if cmd == "validate":
        p = validate(repo); print("OK" if not p else "\n".join(p)); sys.exit(1 if p else 0)
    if cmd == "next":
        print(next_task(repo)); print("bytes (필요/전체):", context_saving(repo)[1:])
