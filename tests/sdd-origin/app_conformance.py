#!/usr/bin/env python3
"""앱의 실제 감지·구조 동작(src-tauri/src/sdd_origin.rs)을 이 하네스의 픽스처와 판정 도구로 채점한다.

1. 임시 폴더에 픽스처 F0~F11을 만든다.
2. 옵트인 Rust 시험(harness_conformance)이 각 픽스처를 실제 코드로 분류하고, 정해진 동작을 그 자리에서 실행한다.
3. 분류를 manifest의 기대 모드와 대조하고, 동작한 픽스처를 judge.py로 채점한다.
4. 업무 종류별 제안(ADR-0045): 개발 목표가 아니면 어떤 픽스처에서도 구조를 제안하지 않는지, goal_cases.json의
   CTO 분석 예시를 앱이 기대한 업무 종류로 읽는지 확인한다.

사용법 (저장소 루트에서): python tests/sdd-origin/app_conformance.py
필요: Rust 툴체인과 Tauri 빌드 의존성(cargo test가 되는 환경). 프로젝트 파일은 바꾸지 않는다.
"""
import json, os, subprocess, sys, tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
sys.path.insert(0, str(HERE))
import judge      # noqa: E402
import ontology   # noqa: E402

# manifest의 기대 모드 → 앱의 분류. 판정 도구의 undecided는 F9 결정 이후 쓰지 않는다.
EXPECT_SCOPE = {"F8": "packages/api"}
EXPECT_FOLLOWED = {"F2": "AGENTS.md", "F3": "CLAUDE.md", "F4": "AGENTS.md"}
# 개발 목표에 구조 질문을 하는 픽스처: 루트 범위이고 이미 U 컴퍼니 구조가 아닌 것.
NOT_OFFERED_FOR_DEVELOPMENT = {"F7", "F8"}


def main():
    failures = []
    with tempfile.TemporaryDirectory() as tmp:
        fixtures = Path(tmp) / "fixtures"
        subprocess.run([sys.executable, str(HERE / "make_fixtures.py"), str(fixtures)], check=True, capture_output=True)
        manifest = json.loads((Path(tmp) / "manifest.json").read_text(encoding="utf-8"))
        out = Path(tmp) / "app_report.json"
        env = {**os.environ, "SDD_ORIGIN_FIXTURES": str(fixtures), "SDD_ORIGIN_OUT": str(out), "SDD_ORIGIN_GOALS": str(HERE / "goal_cases.json")}
        run = subprocess.run(["cargo", "test", "--locked", "--lib", "sdd_origin::tests::harness_conformance", "--", "--ignored", "--exact"],
                             cwd=REPO / "src-tauri", env=env, capture_output=True, text=True)
        if run.returncode != 0 or not out.exists():
            print(run.stdout[-2000:], run.stderr[-2000:])
            sys.exit("Rust 시험 실행 실패")
        report = json.loads(out.read_text(encoding="utf-8"))

        print("## 1. 분류: 앱 vs 픽스처 기대값")
        for fid in sorted(manifest, key=lambda f: int(f[1:])):
            row, want = report[fid], manifest[fid]["expected_mode"]
            problems = []
            if row["mode"] != want: problems.append(f"모드 {row['mode']} ≠ {want}")
            if row["scope"] != EXPECT_SCOPE.get(fid, "."): problems.append(f"범위 {row['scope']}")
            if fid in EXPECT_FOLLOWED and row["followed"] != EXPECT_FOLLOWED[fid]: problems.append(f"따르는 파일 {row['followed']}")
            if fid == "F4" and not row["conflict"]: problems.append("충돌 미표시")
            if fid == "F7" and not (row["nextTask"] or "").startswith("T-003"): problems.append(f"다음 작업 {row['nextTask']}")
            failures += [f"{fid} {p}" for p in problems]
            print(f"{'OK ' if not problems else 'BAD'} {fid:4} {row['mode']:9} 범위 {row['scope']:13} 동작 {row['actions']} {'; '.join(problems)}")

        print("\n## 2. 관리 구간 문구: 앱 = 하네스")
        same = report["managedBlock"] == ontology.MANAGED_BLOCK
        failures += [] if same else ["관리 구간 문구 불일치"]
        print("OK " if same else "BAD", "src-tauri/src/sdd_origin.rs managed_block == ontology.MANAGED_BLOCK")

        print("\n## 3. 실제 동작 결과를 판정 도구로 채점")
        for fid, row in sorted(report.items()):
            if not isinstance(row, dict) or "applied" not in row: continue
            mode, r = judge.check(fixtures / fid, mode=row["applied"])
            bad = [f"{c}: {m}" for c, s, m in r.rows if s == "FAIL"]
            failures += [f"{fid} {b}" for b in bad]
            print(f"{'OK ' if r.passed else 'BAD'} {fid:4} {row['applied']:11} " + (", ".join(c for c, s, _ in r.rows if s == "PASS")) + (f" | 실패: {bad}" if bad else ""))

        print("\n## 4. 업무 종류별 제안 (ADR-0045)")
        for fid in sorted(manifest, key=lambda f: int(f[1:])):
            row = report[fid]
            want = fid not in NOT_OFFERED_FOR_DEVELOPMENT
            problems = []
            if row["offerOther"]: problems.append("개발이 아닌 목표에 제안")
            if row["offerDevelopment"] != want: problems.append(f"개발 목표 제안 {row['offerDevelopment']} ≠ {want}")
            if row["offerDevelopmentMuted"]: problems.append("묻지 않기인데 제안")
            failures += [f"{fid} {p}" for p in problems]
            print(f"{'OK ' if not problems else 'BAD'} {fid:4} 개발 {'제안' if row['offerDevelopment'] else '없음'} · 기타 {'제안' if row['offerOther'] else '없음'} · 묻지 않기 {'제안' if row['offerDevelopmentMuted'] else '없음'} {'; '.join(problems)}")
        cases = {c["id"]: c for c in json.loads((HERE / "goal_cases.json").read_text(encoding="utf-8"))}
        for got in report.get("goals", []):
            case = cases[got["id"]]
            ok = got["workKind"] == case["kind"]
            failures += [] if ok else [f"{got['id']} 업무 종류 {got['workKind']} ≠ {case['kind']}"]
            print(f"{'OK ' if ok else 'BAD'} {got['id']:4} {got['workKind']:11} {case['note']}" + (f" · 조언: {got['advice']}" if got["advice"] else ""))
        if len(report.get("goals", [])) != len(cases): failures.append("goal_cases.json 사례 일부를 앱이 읽지 않음")

    print(f"\n실패 {len(failures)}건")
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
