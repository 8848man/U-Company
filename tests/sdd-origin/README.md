# SDD 구조 감지·부트스트랩 오프라인 하네스

[SI-03](../../sdd/specImport/SI-03-origin.md)과 [ADR-0044](../../sdd/architecture/decisions/ADR-0044.md)의 오프라인 검증 도구다. 규칙의 원본은 명세이고, 이 폴더는 그 규칙을 픽스처와 판정 스크립트로 확인한다. [Validation](../../sdd/validation.md)과 [QA-43](../../qa/MANUAL-QA.md)이 이 폴더를 참조한다. 오프라인 결과만으로 실제 Windows 앱이나 실제 Claude/GPT 동작을 검증했다고 보지 않는다.

필요: Python 3.11+, git. `app_conformance.py`는 추가로 `cargo test`가 되는 환경(Rust, Tauri 빌드 의존성)이 필요하다.

## 무엇을 확인하나

| 명령 (이 폴더에서) | 확인하는 것 | 결과 기준 |
| --- | --- | --- |
| `python selftest.py` | 판정 도구 자체: 정상 동작은 통과, 위반 동작은 정확히 그 항목에서 실패 | "실패 0건" |
| `python app_conformance.py` | **앱의 실제 코드**(`src-tauri/src/sdd_origin.rs`): 픽스처 F0~F11 분류, 관리 구간 문구 일치, 실제로 쓴 파일을 판정 도구로 채점 | "실패 0건" |
| `python samples/run_matrix.py && python samples/summarize.py` | 공개 SDD 표본 3종(Spec Kit, OpenSpec, 3파일 워크플로우)과 참조 오리진의 비교 기록 | `sample_results/` 갱신. 합격 기준이 아니라 설계 근거 |

표본 비교에는 `pip install git+https://github.com/github/spec-kit.git`와 `npm install -g @fission-ai/openspec`가 필요하다.

## 파일

| 파일 | 역할 |
| --- | --- |
| `sdd_config.json` | 판정 기준: 구조 경로(`.ucompany/sdd`), 필수 파일, 감지 신호, 온톨로지 노드·관계, 정적 스캔 범위. 명세가 바뀌면 여기를 먼저 고친다 |
| `make_fixtures.py` | 픽스처 저장소 F0~F11 생성. `fixture-init` 태그로 초기 상태 고정. 기대값은 저장소 밖 `manifest.json` |
| `judge.py` | 실행 전후 파일 차이로 판정. 모드: `bootstrap`, `add_section`, `convert`, `existing`, `ask`, `continue` |
| `scan.py` | 정적 감지의 파이썬 참조 구현(앱 구현과 같은 규칙). 보고서 크기로 AI가 읽을 양을 잰다 |
| `ontology.py` | 구조 v0의 예시 생성, 그래프 무결성 검사(`validate`), 다음 작업 찾기(`next`). `MANAGED_BLOCK`은 앱과 같은 문구 |
| `selftest.py` | 판정 도구 자체 검증 (62건) |
| `app_conformance.py` | 앱 코드와 하네스 대조 (옵트인 Rust 시험 `harness_conformance` 실행) |
| `tools.json`, `origin_templates/adapters/` | 실측 툴(Claude, GPT)과 placeholder 툴(Gemini, Cursor, Copilot)의 연결 정의. placeholder 템플릿은 앱이 만들지 않는다 |
| `samples/`, `sample_results/` | 표본 비교 실행기와 결과 |
| `results_template.csv` | 실제 툴로 수동 실행할 때의 결과 기록표 |

## 픽스처

| ID | 구성 | 앱 분류 | 동작 시험 |
| --- | --- | --- | --- |
| F0 | 빈 저장소 | bootstrap | bootstrap |
| F1 | 코드 + README | bootstrap | bootstrap |
| F2 | 스펙 절차가 있는 AGENTS.md (`docs/rfc/`) | existing | convert |
| F3 | CLAUDE.md만 (`design/`) | existing | — |
| F4 | AGENTS.md + CLAUDE.md, 스펙 위치가 다름 | existing, AGENTS.md를 따름, 충돌 표시 | convert |
| F5 | Spec Kit 형식 `specs/` + `.specify/` | existing | — |
| F6 | `.cursor/rules/`만 | existing | convert |
| F7 | U 컴퍼니 구조 진행 중 (T-001·T-002 완료, 팀 메모) | continue, 다음 작업 T-003 | — |
| F8 | `packages/api/AGENTS.md`만 | existing, 범위 `packages/api`, 루트 동작 없음 | — |
| F9 | 제목 한 줄만 있는 AGENTS.md | ask | add_section |
| F10 | 코드 스타일만 있는 AGENTS.md | ask | add_section |
| F11 | 0바이트 AGENTS.md | bootstrap | bootstrap |

## 정책 대응표

정책 번호는 2026-10-12 CEO 결정을 정리한 UX·동작 정책 문서의 번호다. 원본 규칙은 SI-03에 있다.

| 정책 | 내용 | SI-03 절 | 판정 항목 / 시험 |
| --- | --- | --- | --- |
| P1 | 정적 감지 먼저, AI는 보고서와 후보만 | 감지 | `scan.py` 보고서 크기, Rust `detection_and_planning_change_nothing` |
| P2 | 외부 SDD 폴더·다른 툴 지침도 기존 SDD | 감지 | F5·F6 분류 |
| P3 | 하위 폴더 신호는 그 범위만, 루트는 그대로 | 감지 | F8 분류, `stale_digest…nested_scope_are_refused` |
| P4 | 기존 규칙대로 작업 | 감지 | F2·F3 분류, `no-sdd-dir`, `guidelines-untouched`, `spec-in-rule-location` |
| P5 | 충돌 시 AGENTS.md 우선, 알림 | 감지 | F4 `followed`·`conflict`, 셀프테스트 F4 |
| P6 | 빈 지침은 없음으로, 내용이 있으면 유지 | 감지 | F9·F11 |
| P7 | 절차 없는 지침은 그대로 두고 추가 여부 확인 | 동작과 쓰기 계약 | F10 `no-change-before-consent`, `original-prefix-kept` |
| P8 | 기존 규칙이 있으면 항상 바꾸기 여부를 묻고, 답 전 무변경 | 동작과 쓰기 계약 | `actions`에 `convert`, `no-change-before-consent`, 확인 없는 실행 거절 |
| P9 | 바꾸기 전 `.ucompany/legacy/`에 백업, AGENTS.md 원문 보존 | 동작과 쓰기 계약 | `backup-byte-identical`, `agents-original-kept`, `tool-rules-untouched` |
| P10 | 신호가 없을 때만 부트스트랩 | 동작과 쓰기 계약 | `structure`, `entry-points-to-sdd`, `claude-bridge` |
| P11 | 툴 전용 지침만 있으면 진입점을 몰래 만들지 않음 | 감지 | 셀프테스트 "F3 AGENTS.md 새로 생성 위반" |
| P12 | 이미 구조가 있으면 다시 만들지 않고 관리 구간만 갱신 | 동작과 쓰기 계약 | `done-tasks-kept`, `human-text-kept`, Rust `rerun_replaces_only_the_managed_block` |
| P13 | 노드·관계 그래프와 무결성 | 구조 (v0) | `ontology-integrity` |
| P14 | 단계를 넘기 전 사용자 확인 | 관리 구간 3번 | 파일로 판정 불가, QA-43 대화 확인 |
| P15 | 실측 툴 연결만 생성 | 툴 연결 | `claude-bridge`, placeholder 파일 미생성 |

## 실제 툴로 수동 확인할 때

픽스처를 만든 뒤(`python make_fixtures.py`), 각 픽스처를 앱 또는 Claude Code·Codex로 열어 표준 요청("할 일에 마감일 필드를 추가해줘")을 보낸다. 끝나면 `python judge.py check fixtures/<ID>`로 채점하고 `results_template.csv`에 기록한다. 초기화는 `git -C fixtures/<ID> reset --hard fixture-init`, `git -C fixtures/<ID> clean -fdx`.

`manifest.json`은 픽스처 폴더 밖에 둔다. 테스트 대상이 기대값을 읽지 못하게 하기 위해서다.
