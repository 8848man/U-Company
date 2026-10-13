#!/usr/bin/env python3
"""정적 SDD 신호 스캐너 (결정 2: 하위 폴더까지 감지하되, AI 토큰은 아낀다).

AI가 프로젝트 전체를 읽는 대신, 이 스캐너가 파일 이름·위치·간단한 키워드만 보고
'범위(scope)별 신호'와 'AI가 꼭 읽어야 할 후보'를 작은 보고서로 만든다.
AI(3단계)는 이 보고서와 후보 파일만 읽으면 된다.

  python3 scan.py <repo>          보고서(JSON) 출력
"""
import json, os, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
CFG = json.loads((ROOT / "sdd_config.json").read_text(encoding="utf-8"))
SIG = CFG["signals"]
TEXT_EXT = {".md", ".mdc", ".txt", ".py", ".js", ".ts", ".tsx", ".jsx", ".json", ".yaml", ".yml", ".toml",
            ".java", ".kt", ".go", ".rs", ".rb", ".cs", ".swift", ".dart", ".html", ".css", ".sh"}


def _read(p: Path):
    try:
        return p.read_text(encoding="utf-8")
    except (UnicodeDecodeError, OSError):
        return ""


def _has_md(d: Path):
    return any(f.suffix in (".md", ".mdc") for f in d.rglob("*") if f.is_file())


def _scope_signals(repo: Path, d: Path):
    """폴더 d 바로 아래의 신호 (하위 폴더의 신호는 그 폴더의 범위로 따로 잡힌다)"""
    out = []
    if (d / CFG["sdd_dir"]).is_dir():
        out.append({"path": str((d / CFG["sdd_dir"]).relative_to(repo)), "kind": "u_sdd"})
    for name in SIG["priority1"] + SIG["priority2_files"]:
        f = d / name
        if f.is_file():
            t = _read(f)
            if not t.strip():                                  # F9 결정: 빈 지침 파일은 없는 것으로 간주
                continue
            kws = [k for k in CFG["workflow_keywords"] if k.lower() in t.lower()]
            out.append({"path": str(f.relative_to(repo)), "kind": "guideline", "chars": len(t.strip()),
                        "workflow": bool(kws)})
    for name in SIG["priority2_dirs"]:
        if (d / name).is_dir():
            out.append({"path": str((d / name).relative_to(repo)) + "/", "kind": "guideline_dir"})
    for name in SIG["external_sdd_dirs"]:
        if (d / name).is_dir() and _has_md(d / name):        # 코드 테스트용 specs/ 오탐 방지: 문서가 있어야 신호
            out.append({"path": str((d / name).relative_to(repo)) + "/", "kind": "external_sdd"})
    return out


def _mode(signals):
    """한 범위의 신호 → 모드 (계획서 감지 규칙)"""
    if any(s["kind"] == "u_sdd" for s in signals):
        return "continue"
    if not signals:
        return "bootstrap"
    files = [s for s in signals if s["kind"] == "guideline"]
    # F9 결정: 내용이 조금이라도 있으면 기존 파일을 유지하는 기본 처리. 스펙 워크플로우가 없으면 확인(ask)
    if len(files) == len(signals) and not any(s["workflow"] for s in files):
        return "ask"
    return "existing"


def scan(repo):
    repo = Path(repo).resolve()
    ignore, max_depth = set(CFG["scan"]["ignore_dirs"]), CFG["scan"]["max_depth"]
    scopes, files_scanned, text_bytes = [], 0, 0
    for cur, dirs, files in os.walk(repo):
        d = Path(cur)
        depth = len(d.relative_to(repo).parts)
        dirs[:] = sorted(x for x in dirs if x not in ignore and depth < max_depth)
        for f in files:
            p = d / f
            if p.suffix in TEXT_EXT or f in SIG["priority1"]:
                files_scanned += 1
                text_bytes += p.stat().st_size
        if d != repo and any(part in (".ucompany", ".cursor", ".specify", "openspec", ".kiro", "specs") for part in d.relative_to(repo).parts):
            continue                                            # 신호 폴더 내부는 다시 범위로 잡지 않음
        sigs = _scope_signals(repo, d)
        if sigs or d == repo:
            scopes.append({"dir": str(d.relative_to(repo)) or ".", "signals": sigs, "mode": _mode(sigs)})

    root = scopes[0]
    nested = [s for s in scopes[1:] if s["signals"]]
    if root["signals"]:
        root_mode, evidence = root["mode"], [s["path"] for s in root["signals"]]
    elif nested:
        root_mode, evidence = "existing", [f"{s['dir']}/ 범위: {', '.join(x['path'] for x in s['signals'])}" for s in nested]
    else:
        root_mode, evidence = "bootstrap", []

    # AI가 직접 읽어야 하는 후보: 워크플로우 여부가 애매한 지침 파일, 충돌 가능성이 있는 복수 지침
    review = []
    for s in scopes:
        if s["mode"] == "continue":                            # 이미 U 컴퍼니 SDD → 자기 진입점은 검토 불필요
            continue
        g = [x for x in s["signals"] if x["kind"] == "guideline"
             and "@" + CFG["entry_file"] not in _read(repo / x["path"])]   # "@AGENTS.md" 연결 파일은 제외
        for x in g:
            reason = ("내용 거의 없음" if x["chars"] < CFG["min_signal_chars"] else
                      "스펙 워크플로우 키워드 없음" if not x["workflow"] else
                      "같은 범위에 지침 여러 개 (충돌 확인)" if len(g) > 1 else None)
            if reason:
                review.append({"path": x["path"], "bytes": (repo / x["path"]).stat().st_size, "reason": reason})

    report = {"root_mode": root_mode, "evidence": evidence, "scopes": scopes, "ai_review": review}
    report_bytes = len(json.dumps(report, ensure_ascii=False).encode("utf-8"))
    ai_bytes = report_bytes + sum(r["bytes"] for r in review)
    report["stats"] = {"files_scanned": files_scanned, "project_text_bytes": text_bytes,
                       "report_bytes": report_bytes, "ai_reads_bytes": ai_bytes,
                       "ai_reads_ratio": round(ai_bytes / text_bytes, 3) if text_bytes else None}
    return report


if __name__ == "__main__":
    print(json.dumps(scan(sys.argv[1] if len(sys.argv) > 1 else "."), ensure_ascii=False, indent=1))
