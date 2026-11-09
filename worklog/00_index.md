# Worklog index

이 인덱스는 구현·검증 기록의 유일한 탐색 시작점이다. 각 라운드와 단독 마일스톤은 한 파일을 소유한다. 기록은 당시의 사실이며, 현재 제품 규칙이나 승인 권한은 SDD와 보안 명세가 소유한다.

## 읽기 순서와 기록 규칙

1. 아래 표에서 작업 주제와 라운드를 찾아 해당 파일만 읽는다. 현재 상태는 최신 관련 기록과 소유 SDD·QA를 함께 확인한다.
2. 새 라운드는 독립 파일로 만들고 이 인덱스에 라운드 ID, 작업 요약, 검증 상태, 링크를 추가한다. 진행 중에는 같은 라운드 파일에 범위·검증·남은 위험·다음 행동을 갱신한다.
3. 오래된 `다음` 항목은 당시의 재개 지점이며 최신 지시가 아니다. 이 인덱스와 기록은 프로젝트 파일 쓰기나 외부 행동을 승인하지 않는다.

## 최근 단독 마일스톤

- [미통합 작업 브랜치 정리 (2026-11-04, Codex)](milestones/WLG-064.md)
- [작업 차수 경계 구조화 요약과 미완료 기록 점검 (2026-10-25, Codex)](milestones/WLG-058.md)
- [워크로그 폴더와 라운드 인덱스 (2026-10-25, Codex)](milestones/WLG-057.md)
- [한 목표 대화에서 반복 작업 (2026-10-25, Codex)](milestones/WLG-001.md)
- [Codex·Claude CLI 문맥 효율 비교 자료 (2026-10-21, Codex)](milestones/WLG-002.md)
- [다음 실행의 집중 작업 지침 (2026-10-21, Codex)](milestones/WLG-003.md)
- [CLI 에이전트 호출·토큰 분석 자료 (2026-10-21, Codex)](milestones/WLG-004.md)

## 확인된 열린 작업

이 표는 최신 관련 기록의 재개 지점이다. 완료 여부는 연결된 QA와 현행 SDD를 다시 대조한다.

| 작업 | 현재 확인 상태 | 근거 |
| --- | --- | --- |
| 같은 대화의 후속 작업과 차수 요약 | 코드·빌드 확인, Rust 저장소 시험 실행과 v4 앱에서 실제 화면·모의 AI 종단 검증 대기 | [WLG-058](milestones/WLG-058.md), [QA-51](../qa/MANUAL-QA.md#qa-51-완료-후-후속-작업과-차수-경계-요약-adr-0055-adr-0056) |
| v3 출시 준비 파일럿과 중단 복구 | v4 앱에서 실제 Windows QA-39 대기([WLG-073](milestones/WLG-073.md)) | [v3 검증 범위](programs/v3-first-package/open-verification.md) |
| v2 화면·알림·회의 | v4 앱에서 QA-38 오프라인 1~7단계 대기; QA-37은 배포 직전, QA-36은 QA-51·39에 포함, 회의 실제 제공자(8~10) 검증은 회의를 계속 쓸 때만([WLG-073](milestones/WLG-073.md)) | [회의 구현 기록](milestones/WLG-012.md), [v3 검증 범위](programs/v3-first-package/open-verification.md) |
| 런타임 이벤트·취소·세션 재개, 추가 승인, Ubuntu | 초기 체크리스트에서 미완료; 현행 명세·코드를 대조해 별도 마일스톤으로 진행 | [초기 체크리스트](milestones/WLG-054.md), [개선 계획](../sdd/workflow-improvement-plan.md) |

## `지금 할 일` 통합 (2026-10-27, Claude)

| 라운드 | 작업 요약 | 상태 | 파일 |
| --- | --- | --- | --- |
| 개요 | 브랜치 feature/now-todo. ADR-0061. | 기록 | [00_overview.md](programs/now-todo/00_overview.md) |
| T0 | 원래 목표·CEO 할 일·Git 시작을 `지금 할 일`로 모으고 Git 시작을 실제로 실행되게 함 | 완료 | [T0.md](programs/now-todo/T0.md) |

## QA 공회전 끊기 (2026-10-26, Claude)

| 라운드 | 작업 요약 | 상태 | 파일 |
| --- | --- | --- | --- |
| 개요 | 브랜치 feature/qa-loop-breaker. ADR-0060. | 기록 | [00_overview.md](programs/qa-loop-breaker/00_overview.md) |
| Q0 | 결정 기록 | 완료 | [Q0.md](programs/qa-loop-breaker/Q0.md) |
| Q1 | 엔진: 전제 판정, 같은 실패 감지, 완료 기준 결정 카드, 해결 요청 경로, 계획 사실, CEO 할 일 | 완료 | [Q1.md](programs/qa-loop-breaker/Q1.md) |
| Q2 | 화면: 완료 기준 결정 카드, CEO 할 일 | 완료 | [Q2.md](programs/qa-loop-breaker/Q2.md) |
| Q3 | QA-54, PR 본문 | 완료 | [Q3.md](programs/qa-loop-breaker/Q3.md) |

## 진행 중 목표 재개와 달성된 목표 자동 닫기 (2026-10-26, Claude)

| 라운드 | 작업 요약 | 상태 | 파일 |
| --- | --- | --- | --- |
| 개요 | 브랜치 feature/goal-resume. ADR-0059. | 기록 | [00_overview.md](programs/goal-resume/00_overview.md) |
| G0 | 결정 기록 | 완료 | [G0.md](programs/goal-resume/G0.md) |
| G1·G2 | 대화 목표 판단, 이어서 하기, 다시 열기 | 완료 | [G1.md](programs/goal-resume/G1.md) |
| G3 | 원래 목표 줄, 이어서 하기·다시 열기 화면, 대화 단위 열린 작업 | 완료 | [G3.md](programs/goal-resume/G3.md) |
| G4 | QA-53, 모의 AI 방식, PR 본문 | 완료 | [G4.md](programs/goal-resume/G4.md) |

## 명령 권한 승인과 재작업 원인 수정 (2026-10-26, Claude)

| 라운드 | 작업 요약 | 상태 | 파일 |
| --- | --- | --- | --- |
| 개요 | 브랜치 feature/permission-approval. PostGen 후속 차수 분석, ADR-0057·0058. | 기록 | [00_overview.md](programs/permission-approval/00_overview.md) |
| P0 | 기준선, 2번 안 검증, 결정 기록 | 완료 | [P0.md](programs/permission-approval/P0.md) |
| P1 | 파일 이름 대소문자 | 완료 | [P1.md](programs/permission-approval/P1.md) |
| P2 | 결과 전달과 목표 대화 QA 맥락 | 완료 | [P2.md](programs/permission-approval/P2.md) |
| P3 | 명령 권한 승인 | 완료 | [P3.md](programs/permission-approval/P3.md) |
| P4 | 검증과 인계 | 완료 | [P4.md](programs/permission-approval/P4.md) |
| P5 | 명령 카드 줄이기(읽기 전용 명령 제외, 프로젝트 단위 안전 허용)와 `명령 전부 허용` 모드(붉은 띠, push·PR 차단), ADR-0063 | 오프라인 검증 완료, Windows 실사용 QA 대기 | [P5.md](programs/permission-approval/P5.md) |

## 완주율·사용성 개선 라운드 (2026-10-20, Claude)

| 라운드 | 작업 요약 | 상태 | 파일 |
| --- | --- | --- | --- |
| 개요 | 브랜치 feature/completion-usability-rounds(기준 feature/v3-shell-polish ee0ac2d). 계획: 완주율·사용성 개선 라운드, ADR-0052. | 기록 | [00_overview.md](programs/completion-usability/00_overview.md) |
| R0 | 기준선과 계획 기록 | 완료 | [R0.md](programs/completion-usability/R0.md) |
| R1 | 모의 AI 연결 | 완료 | [R1.md](programs/completion-usability/R1.md) |
| R2 | 엔진 종단 시험 | 완료 | [R2.md](programs/completion-usability/R2.md) |
| R3 | 재작업 범위 추가와 "고칠 것 없음" 재작업 | 완료 | [R3.md](programs/completion-usability/R3.md) |
| R4 | 계획 카드 어림과 쉬운 재작업 문구 | 완료 | [R4.md](programs/completion-usability/R4.md) |
| R5 | 결과 요약과 멈춤 통계 | 완료 | [R5.md](programs/completion-usability/R5.md) |
| R6 | 첫 목표 준비와 샘플 앱 흐름 | 완료 | [R6.md](programs/completion-usability/R6.md) |
| R7 | 통합 검증과 인계 | 완료 | [R7.md](programs/completion-usability/R7.md) |

## 대화형 목표 흐름 (2026-10-14, Claude Cowork)

| 라운드 | 작업 요약 | 상태 | 파일 |
| --- | --- | --- | --- |
| 개요 | 작업 중 파일(다른 작업자는 동시에 수정하지 말 것): v3 화면(v3/src/), src/GoalConversationChat.tsx와 새 대화 부품, Office 창, 그리고 M3에서 src-tau… | 기록 | [00_overview.md](programs/conversation-flow/00_overview.md) |
| M0 | 결정 기록과 소유 명세 | 완료 | [M0.md](programs/conversation-flow/M0.md) |
| M1 | 대화 한 줄기 | 완료 | [M1.md](programs/conversation-flow/M1.md) |
| M2 | 홈과 자동 시작 | 완료 | [M2.md](programs/conversation-flow/M2.md) |
| M3 | 개선과 거부 | 완료 | [M3.md](programs/conversation-flow/M3.md) |
| M4 | Office 창 | 완료 | [M4.md](programs/conversation-flow/M4.md) |
| M5 | v3 화면 다듬기 | 완료 | [M5.md](programs/conversation-flow/M5.md) |
| M6 | 원클릭 해결 요청과 계획 형식 자동 교정 | 완료 | [M6.md](programs/conversation-flow/M6.md) |
| M7 | 팀원 요청문에 수용 기준·근거 자료 전달 | 완료 | [M7.md](programs/conversation-flow/M7.md) |
| M8 | 참조 자료(긴 글·이미지)와 자료 정리 담당 | 완료 | [M8.md](programs/conversation-flow/M8.md) |
| M9 | 요구사항 자료는 PM이 읽고, 자료 정리 담당은 기본 꺼짐 | 완료 | [M9.md](programs/conversation-flow/M9.md) |
| M10 | 자료 끌어다 놓기와 상자 표시 | 완료 | [M10.md](programs/conversation-flow/M10.md) |
| M11 | PM이 SDD를 관리 | 완료 | [M11.md](programs/conversation-flow/M11.md) |
| M12 | 버전이 이어지는 목표 대화 | 완료 | [M12.md](programs/conversation-flow/M12.md) |

## 프로젝트 작업 규칙 감지와 구조 잡기 (2026-10-12, Claude Cowork)

| 라운드 | 작업 요약 | 상태 | 파일 |
| --- | --- | --- | --- |
| 개요 | 작업 중 파일(다른 작업자는 동시에 수정하지 말 것): src-tauri/src/sdd_origin.rs, src-tauri/src/spec_import.rs(project_root 공개만), src-… | 기록 | [00_overview.md](programs/sdd-origin/00_overview.md) |
| M3 | 계획 승인 창의 작업 구조 질문 | 완료 | [M3.md](programs/sdd-origin/M3.md) |
| M2 | 미정 항목 결정 기록 | 완료 | [M2.md](programs/sdd-origin/M2.md) |
| M1 | 감지·확인 후 구조 동작·오프라인 하네스 | 완료 | [M1.md](programs/sdd-origin/M1.md) |

## v3 1차 개선 패키지 구현 (2026-10-10, Claude Cowork)

| 라운드 | 작업 요약 | 상태 | 파일 |
| --- | --- | --- | --- |
| 개요 | 작업 중 파일(다른 작업자는 동시에 수정하지 말 것): v3/ 전체, v2/src/V2Navigation.tsx, src/office/, src-tauri/src/squad.rs, src-tauri/s… | 기록 | [00_overview.md](programs/v3-first-package/00_overview.md) |
| M1 | 기준선, 첫 구현 범위, 소유 명세·ADR | 완료 | [M1.md](programs/v3-first-package/M1.md) |
| M2 | 산출물 업무 계약·확인된 배정·호출 전 선점·산출물별 QA | 완료 | [M2.md](programs/v3-first-package/M2.md) |
| M3 | 저장 상태 기반 복구 표시와 목표 화면·Office 동일 표시 | 완료 | [M3.md](programs/v3-first-package/M3.md) |
| M4 | v3 앱 셸 생성 | 완료 | [M4.md](programs/v3-first-package/M4.md) |
| M5 | Office 캐릭터 스프라이트(선택 스킨)와 스쿼드 정책 제안 | M7에서 잔여 자산 해소 | [M5.md](programs/v3-first-package/M5.md) |
| M6 | 목표별 스쿼드와 목표 동안만 계약하는 외부 인력 | 완료 | [M6.md](programs/v3-first-package/M6.md) |
| M7 | MetroCity 원본 팩으로 기본 캐릭터 교체 | 완료 | [M7.md](programs/v3-first-package/M7.md) |
| M8 | 캐릭터를 프로그램 안에서 실시간 합성 + 팀원별 모습 선택 | 완료 | [M8.md](programs/v3-first-package/M8.md) |
| M9 | UI/UX 구현 철학을 SDD 상위 원칙으로 고정 | 완료 | [M9.md](programs/v3-first-package/M9.md) |
| M10 | 업무·회의 화면에 UI/UX 철학 적용 | 완료 | [M10.md](programs/v3-first-package/M10.md) |
| M11 | 목표 보관과 설정 › 백업 삭제 | 완료 | [M11.md](programs/v3-first-package/M11.md) |
| M12 | Git 저장소와 PR 워크플로우 명세 | 완료 | [M12.md](programs/v3-first-package/M12.md) |
| 실제 앱에서 아직 검증하지 못한 범위 | 다음: Windows에서 v2/start-windows.ps1 -DataDir .run-data\v3-pilot로 QA-39를 수행하고 결과를 기록한다. PM/QA JSON 준수율이 낮으면 프롬프트를… | 기록 | [open-verification.md](programs/v3-first-package/open-verification.md) |
## 단독 마일스톤

| 기록 | 작업 요약 | 상태 | 파일 |
| --- | --- | --- | --- |
| WLG-072 (2026-11-10, Claude) | 10-27 이후 미커밋 작업을 한 브랜치로 정리하고 main 병합: ADR-0063→0068·WLG-064→071 번호 변경, `명령 전부 허용` 제거하고 ADR-0066 모드 기준으로 통합 | PR #23 merge(2026-11-09 UTC). 타입 검사·vitest 160개 통과, Rust 시험 컴파일 통과·실행 `0xc0000139`, 실앱 QA 미검증 | [WLG-072.md](milestones/WLG-072.md) |
| WLG-073 (2026-11-10, Claude) | v2·v3 수동 QA(QA-36~39, QA-51)를 v4 앱 하나로 실행하도록 실행 명령과 우선순위 정리 | 문서만 변경; 실제 QA 미실행 | [WLG-073.md](milestones/WLG-073.md) |
| WLG-066 (2026-11-07, Codex) | QA 첫 실패 재작업 경로와 ttj 원인 대조: PM은 `source/`를 CEO 선행 조치로 넘겼고 Claude 세션은 웹 접속을 시도하지 않음 | 코드 확인·v3 빌드 통과; Rust 시험 실행 `0xc0000139`·승인 후 실제 웹 접속·원본 확보 전 계획 차단 미검증 | [WLG-066.md](milestones/WLG-066.md) |
| WLG-067 (2026-11-07, Codex) | Dev의 거절된 명령을 PM이 먼저 검토하고 기존 권한 재시도·CEO 요청·명령 없이 진행으로 분기 | Rust 컴파일·모의 어댑터 구문 통과; Windows 시험 실행 `0xc0000139`, 실제 Claude 미검증 | [WLG-067.md](milestones/WLG-067.md) |
| WLG-068 (2026-11-07, Codex) | 채팅·설정에 수동/오토/풀 오토 명령 모드: 로컬 조회 자동, 선언 파일 삭제·이동은 풀 오토만 | Rust·v3 빌드·UI 단위 시험 통과; Windows Rust 시험 실행·실제 Claude 미검증 | [WLG-068.md](milestones/WLG-068.md) |
| WLG-069 (2026-11-08, Codex) | 열린 작업의 기본 범위를 선택된 프로젝트로 제한하고 펼친 창에 모든 작업 보기 토글 추가 | 데이터 단위 시험·v3 빌드·타입 검사 통과; 데스크톱 수동 QA 미검증 | [WLG-069.md](milestones/WLG-069.md) |
| WLG-070 (2026-11-08, Codex) | PM이 필요한 거절 호출만 승인 요청하고 셸 스크립트 본문은 명령 규칙에서 제외 | Rust 시험 컴파일·모의 어댑터 구문 통과; Windows Rust 실행 `0xc0000139`, 실제 Claude 미검증 | [WLG-070.md](milestones/WLG-070.md) |
| WLG-075 (2026-11-10, Claude) | Windows에서 Rust 시험 실행 파일이 `0xc0000139`로 멈추던 원인(앱 매니페스트가 앱 실행 파일에만 들어감)을 고치고, 빠진 모의 AI 방식 7개 추가 | `cargo test` 219 통과·19 실패·3 무시; 실패 19개는 다음 마일스톤 | [WLG-075.md](milestones/WLG-075.md) |
| WLG-059 (2026-10-31, Developer A) | v4 기술·구조 프리셋 명세와 ADR-0062 결정 기록; 이번 차수에서 파일·검증 근거 재대조 | 명세 기록, 런타임 미검증 | [WLG-059.md](milestones/WLG-059.md) |
| WLG-060 (2026-10-31, Developer B) | 프리셋 엔진·저장·계획 승인 결합; Rust 과거 주장과 이번 실행 결과 분리 | 코드 존재, Rust 실행 검증 대기 | [WLG-060.md](milestones/WLG-060.md) |
| WLG-061 (2026-10-31, Developer A) | v4 설정·AI 전송 승인·계획 카드와 실행 경로; 빌드·화면·데스크톱 확인 추적 | 코드 존재, 빌드·데스크톱 미검증 | [WLG-061.md](milestones/WLG-061.md) |
| WLG-062 (2026-11-01, Developer A) | 비평 반영 기획안과 `설정 > 앱 기술·구조` 안내; 문서·탐색·화면 코드 근거 및 검증 결과 기록 | 문서·정적 코드 확인, 집중 시험·빌드 의존성 없음으로 실패, 데스크톱 QA 미검증 | [WLG-062.md](milestones/WLG-062.md) |
| WLG-063 (2026-11-01, Developer A) | 재작업 3: 실제 차단 건의 작업·승인·통합·QA 원장과 `sdd/product.md` 현재 해시 대조 및 원인 가설 분리; 엔진 재사용 증거 전환·저장 충돌 스냅샷·v3 충돌 화면 보수 코드 확인; 최초 계획·QA 재작업·외부 변경 집중 시험 | 실제 원장·이전 통합 해시 부재로 변경 주체·시점·승인 여부 미확인. 엔진·화면 동작 미검증: Rust 집중·전체 시험 컴파일 통과, 실행 파일은 `0xc0000139`로 시작 실패; v3 빌드는 로컬 `tsc` 부재로 실패. 시나리오·실제 화면 QA 미검증 | [WLG-063.md](milestones/WLG-063.md), [ADR-0068](../sdd/architecture/decisions/ADR-0068.md), [Validation](../sdd/validation.md) |
| WLG-071 (2026-11-04, Codex) | 제품·엔진·UX·보안 관점 Self grill 93문항과 답을 가를 증거 정리 | 읽기 전용 교차 검토 및 문서 작성 완료; 고객·실앱·제공자 실행 검증은 수행하지 않음 | [WLG-071.md](milestones/WLG-071.md), [Self grill](../analysis/ucompany-self-grill-2026-11-04.md) |
| WLG-065 (2026-11-04, Codex) | 세 서브 에이전트의 Self grill 93문항 후속 결정안 통합 | 우선순위·검증·보류 판단 기록; 실제 고객·앱·제공자 검증 및 CEO 전용 결정은 미수행 | [WLG-065.md](milestones/WLG-065.md), [결정안](../analysis/ucompany-self-grill-decisions-2026-11-04.md) |
| WLG-064 (2026-11-04, Codex) | 미통합 목표·권한·모델 선택 브랜치 통합, 기록 충돌 해결 | 프런트 시험·빌드와 Rust 컴파일 통과, Rust 시험 실행 환경 오류·실앱 QA 대기 | [WLG-064.md](milestones/WLG-064.md) |
| 작업 차수 경계 구조화 요약과 미완료 기록 점검 (2026-10-25, Codex) | 새 작업 차수마다 저장 기록으로 요약 인계를 만들고 누적 약 8,000토큰부터 대화 발췌를 제외; 실제 Rust 실행·실앱 QA는 미검증 | 구현, 검증 대기 | [WLG-058.md](milestones/WLG-058.md) |
| 워크로그 폴더와 라운드 인덱스 (2026-10-25, Codex) | 네 프로그램의 36개 라운드와 56개 기존 단독 기록을 파일별로 이관하고 인덱스에서 탐색하도록 연결 | 완료 | [WLG-057.md](milestones/WLG-057.md) |
| 한 목표 대화에서 반복 작업 (2026-10-25, Codex) | CEO 결정: 완료된 실행 목표의 기존 대화에서 후속 작업을 바로 지시한다. 내부적으로는 차수별 Goal ID를 유지하고, 이전 완료·QA·승인은 보존하며 다음 차수의 파일 쓰기는 새 계획 승인을 받는… | 기록 | [WLG-001.md](milestones/WLG-001.md) |
| Codex·Claude CLI 문맥 효율 비교 자료 (2026-10-21, Codex) | 레벨 4·5 Dev1(Codex)/Dev2(Claude)의 동일 수용 기준 독립 구현 두 쌍을 앱 기록과 로컬 CLI 세션 JSONL에서 재대조했다. Codex 응답은 고유 response ID 17/… | 기록 | [WLG-002.md](milestones/WLG-002.md) |
| 다음 실행의 집중 작업 지침 (2026-10-21, Codex) | 범위: 기존 분석→PM 계획→승인→격리 작업→통합→독립 QA·재작업 경로는 유지하고, 다음 목표의 일반 계획·QA 재작업 계획과 각 개발자 업무 요청문에 근거 파일·집중 검증 지침을 추가했다. 필수 프… | 기록 | [WLG-003.md](milestones/WLG-003.md) |
| CLI 에이전트 호출·토큰 분석 자료 (2026-10-21, Codex) | 기본 앱 SQLite의 동일 수용 기준인 레벨 4·5 Dev1(Codex) / Dev2(Claude) 업무 2쌍을 대조했다. 각 쌍은 dev1/dev2 부분을 제외한 선언 파일 구성이 같고, 저장된 수… | 기록 | [WLG-004.md](milestones/WLG-004.md) |
| v3 첫 개선 패키지 문서 인계 (2026-10-10, Codex) | 현재 소유 명세와 구현 경계(Product, Architecture/ADR-0004, Security, DM-03/04/05, AI-03, UX-01/02, GC-03/04, Progress Visib… | 기록 | [WLG-005.md](milestones/WLG-005.md) |
| Meeting blocking edge cases | W0 (Codex): moved MAM-014 D1–D3 recovery and transport contracts into ME-01, ME-02, AI-03, Validation, UF-09 and… | 기록 | [WLG-006.md](milestones/WLG-006.md) |
| Meeting packet length checked at plan validation (2026-10-09, Claude Cowork) | Cause: plan validation accepted up to 24 KB of sources, but the proposing packet sends every source in full to e… | 기록 | [WLG-007.md](milestones/WLG-007.md) |
| Meeting plan approval false "sources changed" (2026-10-09, Claude Cowork) | Cause, from the user's .run-data/meeting-qa data: all five source files were byte-identical to the review, but t… | 기록 | [WLG-008.md](milestones/WLG-008.md) |
| Claude Code connection check on Windows (2026-10-09, Claude Cowork) | Diagnosed the claude.cmd connection: the first failed status was recorded at 18:28:28 KST while Claude Code 2.1.… | 기록 | [WLG-009.md](milestones/WLG-009.md) |
| Office-character meeting first implementation (2026-10-09, Claude Cowork) | MAM-012 response: C1–C4, C6, C8–C10 accepted, C5 partially accepted, C7 (hash chain, Markdown export) deferred.… | 기록 | [WLG-010.md](milestones/WLG-010.md) |
| Office-character meeting redesign proposal (2026-10-09, Claude Cowork) | Recorded the user's intended model as MAM-009: existing Office team members are the participants (snapshotted, n… | 기록 | [WLG-011.md](milestones/WLG-011.md) |
| v2 meeting engine, transmission gate, decision package and UI (2026-10-09, Claude Cowork) | Re-verified the offline first slice before building on it. Correction: the earlier checkpoint's "cargo test --li… | 기록 | [WLG-012.md](milestones/WLG-012.md) |
| Multi-agent meeting cross-review folder (2026-10-09) | Read AGENTS.md, the SDD authoring principle, and the meeting proposal; compare its claims with Security, DM-01–0… | 기록 | [WLG-013.md](milestones/WLG-013.md) |
| Proposed v2 multi-perspective AI meeting specification (2026-10-09) | Compare the requested meeting loop with the current CTO → PM → developer → QA engine and its post-integration QA… | 기록 | [WLG-014.md](milestones/WLG-014.md) |
| v3 first implementation slice audit (2026-10-09) | Inspect current goal planning, task dispatch, file integration, QA, report, model, and v2 goal display against t… | 기록 | [WLG-015.md](milestones/WLG-015.md) |
| v3 scope correction: company, not app factory (2026-10-09) | Recheck the product goal and the first v3 candidate list after the user's correction. | 기록 | [WLG-016.md](milestones/WLG-016.md) |
| v3 first customer-job focus (2026-10-09) | Read the SDD entry point, product/security/workflow ownership, v2 UX plan, ADR-0033, and affected v2 controller… | 기록 | [WLG-017.md](milestones/WLG-017.md) |
| v2 attention sound and Windows notifications (2026-10-09) | Inspect saved approval/goal transitions, current Tauri toolchain, notification support, and UI ownership. | 기록 | [WLG-018.md](milestones/WLG-018.md) |
| v2 UI complexity reduction (2026-10-08) | Inspect the simplification plan, App controller, reusable components, launch configuration, and security boundar… | 기록 | [WLG-019.md](milestones/WLG-019.md) |
| Whole-app UX complexity reduction plan (2026-10-08) | Inspect the six top-level menus, shared goal workspace, Office/team settings, project SDD panel, and token view… | 기록 | [WLG-020.md](milestones/WLG-020.md) |
| Topic-folder SDD principle and existing-document migration (2026-10-08) | Inventory SDD files, headings, existing folder indexes, and affected links. | 기록 | [WLG-021.md](milestones/WLG-021.md) |
| Linked user-flow SDD graph (2026-10-08) | Locate the existing MVP sequence and feature-specific interaction rules; avoid copying their normative contracts… | 기록 | [WLG-022.md](milestones/WLG-022.md) |
| Tauri command registration warning and UI reachability (2026-10-08) | Confirm create_portable_spec and set_goal_review_limit are invoked from the UI but missing from the Tauri comman… | 기록 | [WLG-023.md](milestones/WLG-023.md) |
| In-app Git bootstrap from blocked goal conversation (2026-10-08) | Trace the metadata-only integration failure and current advice-only conversation path. | 기록 | [WLG-024.md](milestones/WLG-024.md) |
| One-send AI-routed conversation (2026-10-08) | Identify the actual read-only ask_goal path, separate guidance command, safe checkpoint, and approval boundary. | 기록 | [WLG-025.md](milestones/WLG-025.md) |
| Goal instructions inside the conversation (2026-10-08) | Confirm that the current mode switch, generic read-only helper, and separate guidance card make the chat appear… | 기록 | [WLG-026.md](milestones/WLG-026.md) |
| Team-routed goal changes and verified continuation (2026-10-08) | Inspect current guidance checkpoint, linked revision, task integration, approvals, and QA path. | 기록 | [WLG-027.md](milestones/WLG-027.md) |
| Conversational teammate replies (2026-10-08) | Trace the saved-turn runtime prompt and current chat presentation; identify the mandatory report scaffold. | 기록 | [WLG-028.md](milestones/WLG-028.md) |
| Conversation-first goal workspace (2026-10-08) | Review the existing goal workspace, recovery flow, task focus, density policy, and goal conversation/security bo… | 기록 | [WLG-029.md](milestones/WLG-029.md) |
| Open-work floating launcher and worker discussion (2026-10-08) | Inspect the goal conversation, actual read-only worker call, project switching, unfinished state, and approval b… | 기록 | [WLG-030.md](milestones/WLG-030.md) |
| Blocked-goal recovery UX (2026-10-08) | Trace blocked-goal entry points and engine-safe retry, approval, and revision transitions. | 기록 | [WLG-031.md](milestones/WLG-031.md) |
| Git-conversion task integration diagnosis (2026-10-08) | Trace the reported Task produced no file changes to ordinary-folder integration: only declared working-file byte… | 기록 | [WLG-032.md](milestones/WLG-032.md) |
| Explorer-style project folder selection (2026-10-08) | Inspect project registry, current Windows picker, nested-folder copy behavior, SDD index, product/security specs… | 기록 | [WLG-033.md](milestones/WLG-033.md) |
| Single-origin project SDD synchronization (2026-10-08) | Recheck the index, import Spec, ADR-0020, preview/apply storage path, and UI after the user identified two mutab… | 기록 | [WLG-034.md](milestones/WLG-034.md) |
| Portable cross-tool specification layer (2026-10-08) | Inspect SDD/ADR, importer, storage, project path rules, and UI. Identify the SQLite profile as the current appli… | 기록 | [WLG-035.md](milestones/WLG-035.md) |
| External SDD import and exception specifications (2026-10-08) | Inspect the SDD index, domain/security/architecture rules, global team storage, project binding, prompt construc… | 기록 | [WLG-036.md](milestones/WLG-036.md) |
| Provider login and recovery guidance (2026-10-05) | Inspect saved-path discovery, connection UI, runtime diagnostics, integration/security specs, and official provi… | 기록 | [WLG-037.md](milestones/WLG-037.md) |
| Claude auth recheck and Office team settings (2026-10-05) | Inspect SDD, discovery, saved-connection refresh, team persistence, and Office UI. Reproduce this Windows user's… | 기록 | [WLG-038.md](milestones/WLG-038.md) |
| Git-independent project folders (2026-10-05) | Read SDD index/workflow, project/security/architecture/integration specs, ADR-0003/0006, and affected registrati… | 기록 | [WLG-039.md](milestones/WLG-039.md) |
| Windows launcher binary selection (2026-10-05) | Reproduce the cause from the manifest and launcher: adding provider-harness created a second Cargo binary, while… | 기록 | [WLG-040.md](milestones/WLG-040.md) |
| Provider/runtime boundary and Claude report (2026-10-04) | Trace the actual failed goal through UI command, engine, saved invocation, CLI launch, stdin/stdout/stderr, and… | 기록 | [WLG-041.md](milestones/WLG-041.md) |
| Resume milestone: unified goal workspace (2026-10-03) | Reopen the SDD index/workflow, goal conversation Spec/ADR, affected code, and old checklist. Confirm protocol 10… | 기록 | [WLG-042.md](milestones/WLG-042.md) |
| Active goal conversation and workflow redesign (2026-10-02) | Audit the current goal engine, CEO/Office/board surfaces, trust boundary, and earlier unfinished plans. The engi… | 기록 | [WLG-043.md](milestones/WLG-043.md) |
| Active information density policy (2026-10-02) | Audit repeated records and tall content across CEO, board, Office, project/AI lists, and token usage. Define the… | 기록 | [WLG-044.md](milestones/WLG-044.md) |
| Active team lifecycle and goal visibility (2026-10-02) | Inspect agent persistence/Office filtering, CEO cards, shared graph, and board. Update Product, Domain, progress… | 기록 | [WLG-045.md](milestones/WLG-045.md) |
| Active goal-owned QA caps and token analytics (2026-10-02) | Inspect QA decision path, project/goal persistence, invocation ledger, and current UI. Correct owning specs; ADR… | 기록 | [WLG-046.md](milestones/WLG-046.md) |
| Active CEO goal controls and fixed chrome (2026-10-02) | Inspect current QA limit save path, goal cards, app shell, and related SDD. Update Product and Validation specs.… | 기록 | [WLG-047.md](milestones/WLG-047.md) |
| Active manual QA follow-up (2026-10-02) | Diagnose clean-tree block: run rejects every nonempty git status --porcelain before planning, although worktree… | 기록 | [WLG-048.md](milestones/WLG-048.md) |
| Active QA rework loop and token visibility (2026-10-01) | Diagnose QA rejection: execute_planned returns an error on any non-PASS QA result, so no PM/developer rework occ… | 기록 | [WLG-049.md](milestones/WLG-049.md) |
| Active Office progress visibility (2026-10-01) | Inspect Office canvas, member detail, board, domain events, and blocking CLI runtime. Confirm that existing task… | 기록 | [WLG-050.md](milestones/WLG-050.md) |
| Active Windows runtime and single-HTML follow-up (2026-10-01) | Diagnose Blocked: batch file arguments are invalid: Windows Rust Command passes .cmd arguments through batch par… | 기록 | [WLG-051.md](milestones/WLG-051.md) |
| Active Claude connection follow-up (2026-10-01) | Confirm Windows has an extensionless npm claude shim and sibling claude.cmd; the currently running desktop binar… | 기록 | [WLG-052.md](milestones/WLG-052.md) |
| Active follow-up QA checklist (2026-10-01) | Inspect current project import/creation, toast, path display, spacing, CLI discovery, model field, and Office ch… | 기록 | [WLG-053.md](milestones/WLG-053.md) |
| Active QA-driven improvement checklist (2026-10-01) | Inspect the current discovery, connection save path, disabled controls, project persistence, Office interaction,… | 기록 | [WLG-054.md](milestones/WLG-054.md) |
| Verified commands | 2026-10-01 follow-up revision: node node_modules/typescript/bin/tsc --noEmit, Vite production build, and vitest… | 기록 | [WLG-055.md](milestones/WLG-055.md) |
| Next action | The currently running Windows app (PID 27372 when last checked) still uses the previous desktop binary. After th… | 기록 | [WLG-056.md](milestones/WLG-056.md) |
