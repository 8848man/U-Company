#!/usr/bin/env python3
"""results.json → 케이스 단위 집계 (cases.json, cases.md). 케이스는 필수(FAIL 가능) 항목이 모두 PASS일 때 합격."""
import json, sys
from pathlib import Path

D = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent / "sample_results"
rows = json.loads((D / "results.json").read_text(encoding="utf-8"))
SAMPLES = ["speckit", "openspec", "harper", "uref"]
FIXA = ["F0", "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11"]
FIXB = ["F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10"]


def ok(sel):
    sel = [r for r in rows if sel(r) and r["status"] != "WARN"]
    return bool(sel) and all(r["status"] == "PASS" for r in sel)


def cases_for(exp, s):
    if not any(r["exp"] == exp and r["sample"] == s for r in rows):
        return None
    c = {}
    for f in FIXA:
        c[f"A-{f}"] = ok(lambda r: r["exp"] == exp and r["sample"] == s and r["fixture"] == f and r["check"] == "A-mode")
    for f in FIXB:
        c[f"B-{f}"] = ok(lambda r: r["exp"] == exp and r["sample"] == s and r["fixture"] == f and r["case"] == "B-existing")
    one = lambda f, chk: ok(lambda r: r["exp"] == exp and r["sample"] == s and r["fixture"] == f and r["check"] == chk)
    c["C1"] = one("F0", "C1-structure")
    c["C2"] = one("F1", "C2-originals-untouched")
    c["C3"] = one("F0", "C3-single-entry")
    c["C4"] = one("F0", "C4-spec-plan-tasks")
    c["C5"] = ok(lambda r: r["exp"] == "rerun" and r["sample"] == s)
    c["C6"] = one("F0", "C6-claude-bridge")
    for k, chk in [("D1", "D1-next-task-from-files"), ("D2", "D2-single-entry"), ("D3", "D3-multi-tool")]:
        c[k] = ok(lambda r: r["exp"] == "handoff" and r["sample"] == s and r["check"] == chk)
    return c


out = {exp: {s: c for s in SAMPLES if (c := cases_for(exp, s)) is not None} for exp in ("exp1", "exp2")}
(D / "cases.json").write_text(json.dumps(out, ensure_ascii=False, indent=1), encoding="utf-8")
groups = {"A": "A-", "B": "B-", "C": "C", "D": "D"}
n = {k: sum(1 for kk in next(iter(out["exp2"].values())) if kk.startswith(p)) for k, p in groups.items()}
total = sum(n.values())
lines = [f"| 실험 | 표본 | A 감지({n['A']}) | B 기존 보호({n['B']}) | C 부트스트랩({n['C']}) | D 이어받기({n['D']}) | 합계({total}) |",
         "|---|---|---|---|---|---|---|"]
for exp in out:
    for s, c in out[exp].items():
        g = [sum(v for kk, v in c.items() if kk.startswith(p)) for p in groups.values()]
        lines.append(f"| {exp} | {s} | " + " | ".join(map(str, g)) + f" | {sum(c.values())} |")
(D / "cases.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
print("\n".join(lines))
for exp in out:
    for s, c in out[exp].items():
        print(exp, s, "실패:", [k for k, v in c.items() if not v])
