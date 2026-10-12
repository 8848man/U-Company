# Active work checklist

## v3 1차 개선 패키지 구현 (2026-10-10, Claude Cowork)

작업 중 파일(다른 작업자는 동시에 수정하지 말 것): `v3/` 전체, `v2/src/V2Navigation.tsx`, `src/office/`, `src-tauri/src/squad.rs`, `src-tauri/src/archive.rs`, `src/archive.ts`, `src/ArchiveGoalButton.tsx`, `src/BackupSettings.tsx`, `src-tauri/src/delegation.rs`, `src-tauri/src/meeting.rs`, `src/MeetingPanel.tsx`, `src/meetingStatus.ts`, `src/plainText.ts`, `src/SummaryText.tsx`, `v2/src/V2Work.tsx`, `src/squad.ts`, `src/SquadPanel.tsx`, `src/ExternalStaffPanel.tsx`, `src-tauri/src/engine.rs`, `src-tauri/src/model.rs`, `src-tauri/src/storage.rs`, `src-tauri/src/lib.rs`, `src/types.ts`, `src/goalBrief.ts`, `src/goalRecovery.ts`, `src/GoalConversationChat.tsx`, `src/OfficeGoalRail.tsx`, `v2/src/App.tsx` 및 아래 소유 문서.

### M1 — 기준선, 첫 구현 범위, 소유 명세·ADR (완료)

- [x] 기준선: v2 열린 QA는 QA-36(간결한 탐색), QA-37(알림), QA-38(Office 회의 오프라인→실제 Claude/Codex) 모두 체크되지 않은 상태다. 이번 세션은 Linux 컨테이너에서 작업하므로 Windows 앱을 실행하지 않았다. 따라서 v2 기능을 v3의 검증된 강점으로 주장하지 않는다. Codex의 MAM-015 W0~W6 오프라인 구현(프로토콜 27, 83 Rust 시험)은 Windows QA-38 전이다.
- [x] 코드 대조로 확인한 현행 차이: PM 계획은 파일 변경 업무만, 담당은 목록 순서 순환 배정(`developers[index % len]`), 업무를 `running`으로 바꾼 뒤에 연결을 확인(호출 전 연결 실패 시 업무가 `running`으로 남고 재개 불가), QA 완료는 응답 첫 줄 `PASS` 여부만, 산출물별 기준·근거·판정 저장 없음, 차단 이유 분류 없음.
- [x] 첫 구현 범위 확정: (1) 산출물 업무 계약(산출물·수용 기준 1–8·근거 경로 0–8·능력 `document|code`), (2) 엔진의 능력·연결·busy 확인 단일 배정과 배정 이유, (3) 호출 전 SQLite 트랜잭션 선점, (4) 미시작 승인 계획의 재개 확장, (5) 산출물·기준별 QA 판정 저장과 완료 규칙, (6) 차단 종류 기록과 목표 카드·작업공간·Office의 같은 4요소 표시. 실제 웹 조사·게시·외부 전송, 새 메뉴·실행 모드, Paperclip/Pixel Agents 코드 이식은 범위 밖.
- [x] 소유 문서: [ADR-0038](sdd/architecture/decisions/ADR-0038.md)(신규, ADR 색인), DM-01(busy 재확인 지점), DM-03(산출물 업무·배정·선점·산출물 검토), DM-04(계획 검증·완료 규칙·QA 형식 실패), DM-05(차단 종류·미시작 재개·결과 불명), AI-03(배정이 읽는 연결 능력), GC-04(재개 확장·4요소 결정 카드), 진행 표시 정책(4요소·오래 응답 없음·산출물 표시), Security(근거 경로는 읽기 전용 맥락), Product(파일럿 문장), Validation(v3 파일럿 시나리오), UF-03/UF-06, QA-39 신설, v3 개선안 상태 줄.
- [x] 검증: 문서 변경만. 상대 링크 검사 결과는 아래 M2 기록에 함께 남긴다.
- 남은 위험: PM/QA가 새 JSON 계약을 실제 Claude/Codex에서 안정적으로 지키는지는 Windows 실행 전까지 미확인. 두 목표가 개발 팀원을 공유할 수 없게 되어 동시 목표 처리량이 줄어든다.
- 다음: M2 엔진 구현과 Rust 시험(아래).

### M2 — 산출물 업무 계약·확인된 배정·호출 전 선점·산출물별 QA (완료, 오프라인 검증)

- [x] 변경: 새 `src-tauri/src/delegation.rs`(계약 검증, 능력·연결·busy 확인 배정과 이유, 웨이브 단위 트랜잭션 선점·해제, 미시작 계획 재개 판정, 승인 요약, 산출물별 QA 파싱·저장, 차단 종류 표시 `mark/split_mark`). `engine.rs`: PM 계획 스키마에 deliverable/acceptance/evidence/capability, 계약 검증 후 엔진 배정(목록 순환 배정 제거), 승인 사유에 업무별 산출물·담당·파일·기준·근거, 실행 전 `claim_wave` → 작업공간 준비 실패 시 `release_claims`, 계약 업무가 있으면 산출물·기준별 QA JSON을 요구하고 형식 오류는 `qa_form`(재작업 횟수 미사용), 재작업 업무도 계약·배정 경유, 거절/통합 충돌/QA 한도/호출 실패에 차단 종류 기록, `resume_approved`는 `delegation::resumable` 규칙(미시작 업무만, 승인 유효)으로 확장, CTO 분석을 `goal_analysis`에 따로 저장해 재개된 QA에 전달. `model.rs`: `Task` 계약 필드, `Goal.block_kind`, `DeliverableReview`, 스냅샷 `deliverableReviews`(모두 `serde(default)`, 스키마 버전 변경 없음). `storage.rs`: 재시작 시 `block_kind=interrupted`. `meeting.rs`: `extract_json`을 `pub(crate)`로만 변경(동작 동일). 백엔드 프로토콜 28, 프런트 최소 프로토콜 28.
- [x] 검증(Linux 컨테이너): `cargo test --locked --lib` 89 통과·1 무시(새 시험: delegation 7개 — 차단 표시, 계약 거절, 배정 균형·제외 이유·QA 제외, 동시 선점 4스레드 중 1개만 성공, 웨이브 전부/전무·연결 failed·다른 목표 busy·승인 거절 재확인·병렬 웨이브·해제, 미시작 재개 규칙, 산출물별 QA 통과 조건; engine 출시 준비 3문서 합성 계획→계약→배정→승인 요약; storage 재시작 `interrupted`와 재개 불가). `cargo check --locked --all-targets` 경고는 기존 `runtime.rs` 테스트 import 1건뿐(Windows 전용 시험에서 쓰임, 수정하지 않음).
- 남은 위험: 실제 Claude/Codex PM이 새 계획 스키마를, QA가 산출물별 JSON을 지키는지 미확인(지키지 않으면 각각 계획 단계 차단·`qa_form` 차단으로 안전하게 멈춤). 엔진 `run()` 전체 경로는 Tauri `AppHandle`이 필요해 오프라인 종단 시험이 없음 — 규칙 함수 단위로 검증했다.

### M3 — 저장 상태 기반 복구 표시와 목표 화면·Office 동일 표시 (완료, 모의 렌더링 검증)

- [x] 변경: `src/deliverables.ts`(산출물 행, 차단 4요소 `blockFacts`, 진단 전용 `quietMinutes`), `src/DeliverablePanel.tsx`(산출물·담당·배정 이유·파일·근거·기준별 판정/근거/미확인, 4요소 보기), `goalRecovery.ts`(미시작 승인 계획 재개 조건 확장, 차단 종류별 설명), `GoalConversationChat.tsx`(결정 카드 4요소, 산출물 섹션, 진단 표시), `OfficeGoalRail.tsx`·`V2Home.tsx`(같은 4요소 한 줄), v2 승인 대화상자에 산출물 계획, `types.ts`, `ux.css`(좁은 폭에서 4요소 세로 배치), QA-38 프로토콜 문구, implementation-map 행.
- [x] 검증: 두 프런트 `tsc`, `vitest` 37 통과(새 `deliverables.test.ts` 4개 — 산출물 행·최신 판정·재작업 제외, 4요소, 재개/거절/재시작/QA 형식 설명, 조용한 실행은 진단만), `npm run build`, `npm run build:v2`. 실제 엔진 저장소로 만든 스냅샷(승인 대기 3산출물, 호출 전 차단, 재작업 후 완료)을 모의 Tauri 브리지로 v2에 띄워 Home·목표 작업공간·승인 대화상자·Office 레일이 같은 이유/담당/다음 행동을 보이는지 headless Chromium으로 확인했다(임시 내보내기 시험은 삭제).
- 남은 위험: 390px 폭에서 기존 v2 사이드바가 접히지 않아 본문이 잘린다 — 이번 변경 이전부터의 레이아웃 문제(QA-36 좁은 창 항목)로 수정하지 않았다. 키보드 포커스·동작 줄이기는 기존 컴포넌트 규칙을 따랐으나 실제 Windows에서 확인하지 않았다.

### M4 — v3 앱 셸 생성 (완료, 모의 렌더링 검증)

- [x] 사용자 요청("루트 폴더에 v3로 앱을 새로 만들어줘")에 따라 v2와 같은 방식으로 `v3/` 프런트엔드를 추가했다([ADR-0039](sdd/architecture/decisions/ADR-0039.md)). 같은 Tauri 엔진·명령·SQLite·보안 규칙을 쓰고, 다섯 메뉴와 단일 목표 작업공간을 유지한다. 홈만 다르다: 팀을 실행할 수 없을 때만 보이는 준비 카드, 출시 준비 예시를 넣을 수 있는 명령칸, 이유 → 마지막 확인 → 담당 → 다음 행동으로 읽는 `결정할 일` 목록, 산출물별 진행 카드(`산출물 N개 · QA 통과 k`, 완료 시 `산출물 인수 확인`).
- [x] 변경: `v3/index.html`, `v3/vite.config.ts`, `v3/tsconfig.json`, `v3/src/{App.tsx,V3Home.tsx,main.tsx,v3.css}`, `v3/start-windows.ps1`, `v3/start-ubuntu.sh`, `v3/README.md`, `package.json`(`build:v3`), `v2/src/V2Navigation.tsx`(사이드바 버전 표시 prop, 기본 `v2`), ADR 색인, implementation-map, QA-39 실행 명령을 v3 런처로.
- [x] 검증: 세 프런트 `tsc`(루트·v2·v3), `npm run build:v3`, v3 PowerShell 런처 구문 검사 0 오류, `bash -n v3/start-ubuntu.sh`. 실제 엔진 스냅샷과 빈 스냅샷으로 v3를 모의 Tauri 브리지에 띄워 제목·버전 표시, 예시 넣기, 결정할 일 2건의 4요소, 산출물 카드를 확인했다.
- 부채: `v3/src/App.tsx`는 v2 컨트롤러 복사본이다. 공용 명령·대화상자 훅으로 추출해 v2/v3가 함께 쓰게 하는 것이 다음 구조 작업이다.
- 미검증: Windows에서 `v3\start-windows.ps1` 실제 실행과 QA-39.

### M5 — Office 캐릭터 스프라이트(선택 스킨)와 스쿼드 정책 제안 (진행 중)

- [x] Pixel Agents 캐릭터의 원본인 JIK-A-4 *Metro City* 팩이 itch.io에서 CC0 1.0으로 표시됨을 확인(2026-10-10). 사용자가 고른 권장안대로 Pixel Agents의 수정본이 아니라 원본 CC0 팩을 쓰기로 [ADR-0040](sdd/architecture/decisions/ADR-0040.md)에 기록하고 `third_party/METROCITY_CHARACTERS.md`에 출처를 남겼다. Pixel Agents의 가구·바닥·벽 에셋은 권리 기록 전까지 쓰지 않는다.
- [x] `src/office/sprites.ts`(manifest 검증, 프레임 선택, 시트 목록), `PixelCharacter.tsx`(manifest와 시트가 있으면 캔버스 스프라이트, 없으면 기존 도형), `src/office/assets/characters/README.md`, `ux.css`, v3 Office 안내 문구. 작업 중 애니메이션은 엔진이 작업 중으로 보고한 팀원에게만, 동작 줄이기에서는 정지 프레임.
- [x] 검증: 세 프런트 `tsc`, `vitest` 39 통과(새 `sprites.test.ts` 2개), `npm run build:v3`. 합성 시험 시트와 manifest를 임시로 넣어 v3 Office에서 캔버스 스프라이트 5개가 그려지는 것을 확인한 뒤 시험 파일은 지웠다.
- [x] (해소, M7) CEO가 팩을 루트 `assets/characters/`에 넣었고, 실제 시트 배치를 확인해 캐릭터 시트와 `manifest.json`을 만들었다.
- [x] 스쿼드 정책: [v3 개선안](sdd/v3-improvement-plan.md) "2차 개선 제안 — 오피스 명단과 목표별 스쿼드"에 문제, 모델, 흐름, 예약 범위 선택지(권장 단계별), Office 표시, 기본 명단, 수용 기준, 결정 S1~S3을 정리했다. 제안이며 구현 전이다.

### M6 — 목표별 스쿼드와 목표 동안만 계약하는 외부 인력 (완료, 오프라인·모의 렌더링 검증)

- [x] CEO 결정(2026-10-10): S1 일하는 동안만 예약 + "외부 인력 가져오기", S2 최초 계획자가 확인을 요청할 때(계획 승인 카드) 스쿼드 설정, S3 계획자가 구조를 제안하고 CEO가 승인. 추가 답변: 외부 인력은 캐릭터 파일(.json)로, 목표 동안만 계약. [ADR-0041](sdd/architecture/decisions/ADR-0041.md)(신규, 색인), DM-01 busy 규칙, Security(캐릭터 파일은 신뢰하지 않는 데이터), Validation(QA-40 시나리오), implementation-map, v3 개선안 결정 기록.
- [x] 엔진: 새 `squad.rs`. 목표 시작 트랜잭션에서 분석 리드만 고르고 예약(스쿼드 초안), 계획 시작 시 비어 있는 계획 리드를 트랜잭션으로 고르며 없으면 표시하고 대기, 계획자의 `squad.structure`(`standard`/`launch_prep`=출시 준비 팀) 제안으로 작성자·검토자를 엔진이 채우고 부족분을 사실로 표시, 승인 문구에 스쿼드 구성·이유·부족·CEO 변경. CEO는 승인 카드에서만 작성자·검토자 교체/빈 자리 채우기(리드 불가), 승인 시 스쿼드 지문이 다르면 거절하고 같으면 같은 트랜잭션에서 잠금. busy 규칙은 스쿼드의 단계 담당만 예약(분석·계획·검증), 업무 선점은 잠긴 스쿼드 작성자만, 검토자가 여럿이면 산출물을 나눠 각각 판정, 재작업 계획 전 계획 리드가 비기를 대기, 완료 시 스쿼드 기록 유지와 외부 인력 해제. 외부 인력: 캐릭터 파일 미리보기·가져오기(이름·직함·자리·능력·지시문만, 연결·명령·키는 무시하고 표시, 64KB·4000자 제한, SHA-256 기록), CEO가 고른 로컬 연결에만 연결, 대기 중에는 자동 배정·회의 불가, 승인으로 그 목표에만 계약, 목표 완료 시 해제(숨김·기록 유지), 수동 계약 종료, "정식 채용", 내 팀원 캐릭터 파일 내보내기(연결 정보 없음). 외부 지시문은 프롬프트에서 신뢰하지 않는 참고 메모로 표시. `save_agent`는 계약 정보를 보존. 프로토콜 29.
- [x] 화면: `src/squad.ts`(요약·교체 후보·빈 자리·Office 배지), `SquadPanel.tsx`(v2/v3 승인 대화상자: 구조·이유·멤버·교체·빈 자리 채우기·부족, 승인 시 지문 전송), `ExternalStaffPanel.tsx`(v2/v3 Office: 파일 선택→미리보기→연결 선택→대기 인력, 정식 채용·돌려보내기·계약 종료, 내보내기), Office 캐릭터 배지(스쿼드 자리·외부 상태), v1 승인도 지문 전송.
- [x] 검증(Linux 컨테이너): `cargo test --lib` 96 통과·1 무시(새 squad 시험 7개: 리드 선택, 구조별 채우기와 부족, 단계 예약, 캐릭터 파일 경계, 외부 인력 자동 배정 제외·다른 목표 차단, 교체·지문·잠금·해제·채용, 빈 작성자 자리 채우기와 능력 확인), `cargo check` 경고는 기존 `runtime.rs` 1건, 세 프런트 `tsc`, `vitest` 43 통과(새 `squad.test.ts` 4개), `npm run build`·`build:v2`·`build:v3`. v3를 모의 Tauri 브리지에 띄워 승인 대화상자의 스쿼드 패널·교체·빈 자리 채우기가 `swap_squad_member`/`add_squad_member`/`decide_approval{squadDigest}`를 보내는 것과 Office 배지·외부 인력 미리보기를 headless Chromium으로 확인했다.
- 남은 위험: 실제 Claude/Codex 계획자가 `squad` 구조를 제안하는지 미확인(없으면 기본 팀으로 기록). 기본 5명 명단에서는 리드·검토자가 한 명씩이라 동시 목표는 단계가 겹칠 때 대기한다. 승인 뒤 팀원 교체(연결 고장 시)는 범위 밖이라 `precall` 차단 후 연결을 고쳐야 한다. v3 홈의 결정 목록은 목표 작업공간으로 가며, 스쿼드 편집 대화상자는 업무 화면의 "결정이 필요한 일"에서 열린다.
- 미검증: Windows 실제 실행과 QA-40(외부 캐릭터 파일 가져오기 → 승인 카드 교체 → 실행 → 완료 해제).

### M7 — MetroCity 원본 팩으로 기본 캐릭터 교체 (완료, 모의 렌더링 검증)

- [x] 분석: CEO가 넣은 `assets/characters/MetroCity (1)`(1판)과 `MetroCity 2.0 (2)`(2.0 업데이트) 20개 PNG는 한 장짜리 캐릭터가 아니라 **레이어형**이다. 몸(피부색 6줄), 그림자, 머리(1판 8종 + 2.0 5줄, 2.0의 2번째 줄은 수염), 옷(1판 6종, 2.0 셔츠·바지 5종, 모자 포함 정장 4종). 모든 레이어가 32×32 격자, 한 줄 24칸 = 아래 0–5, 오른쪽 6–11, 위 12–17, 왼쪽 18–23의 6프레임 걷기.
- [x] 변경: `scripts/bake_metrocity.py`(그림자→몸→옷→머리 합성, 결정적 재생성)로 캐릭터 10명 시트(768×32)와 `manifest.json`(정지=아래 0칸, 작업 중=아래 걷기 6fps, 배율 3, `assign`으로 cto·pm·developer-a·developer-b·qa 지정)을 `src/office/assets/characters/`에 생성. `sprites.ts`에 `assign` 검증과 `sheetFor`(지정 시트 → 나머지 시트 순환), `PixelCharacter`/`OfficeCanvas`가 팀원 id를 넘김, 스프라이트 크기·배지 폭 CSS. 출처 기록 `third_party/METROCITY_CHARACTERS.md`, 폴더 README, ADR-0040 적용 기록.
- [x] 검증: 루트·v3 `tsc`, `vitest` src/office 3 통과(배정 시험 추가), 합성 결과를 확대 미리보기로 직접 확인, v3 Office를 모의 브리지로 띄워 캔버스 스프라이트 7개(기본 5 + 외부 2)가 그려지는 것을 headless Chromium으로 확인.
- 남은 일: 실제 Windows 앱에서 보기 확인. 작업 중 동작은 팩에 타이핑 동작이 없어 제자리 걷기로 표현한다.

### M8 — 캐릭터를 프로그램 안에서 실시간 합성 + 팀원별 모습 선택 (완료, 모의 렌더링 검증)

- [x] CEO 요청("수동 스크립트가 아니라 프로그램 내부에서 생성")에 따라 M7의 굽기 스크립트·생성 시트·manifest를 지우고, 앱이 `assets/characters/`의 원본 레이어를 그대로 번들해 캔버스에서 합성하도록 바꿨다(`src/office/metrocity.ts`, 모습별 캐시). `PixelCharacter`는 모습을 받아 그리며 팩이 없으면 기존 도형.
- [x] 팀원별 모습: `Agent.appearance`(피부·옷·머리·수염, 선택 사항). Office › 개인 설정에 `캐릭터 모습` 편집기(미리보기, 피부/옷/머리/수염, 무작위, 기본 모습). 백엔드 `save_agent`는 카탈로그에 있는 선택만 허용. 저장된 모습이 없으면 기본 5명은 고정 기본값, 그 외(외부 인력 포함)는 id에서 정해지는 안정적 모습. 캐릭터 파일에 `appearance`를 담고 가져올 수 있으며 모르는 값은 무시 목록에 표시. 프로토콜 30.
- [x] 검증: `cargo test --lib` 96 통과(캐릭터 파일 모습 왕복·무효 무시 포함), 세 프런트 `tsc`, `vitest` 46 통과(새 `metrocity.test.ts` 5개: 팩 번들 확인, 레이어 순서·모자, 저장값→기본값→id 선택, 무작위 유효성, 프레임), `build:v3` 결과물(vite preview)을 headless Chromium으로 띄워 Office 7명 합성과 편집기에서 옷 변경 미리보기, 요청 실패 0건 확인.
- 번들 이미지 이름의 공백·괄호는 빌드 설정(`assetFileNames`)에서 `_`로 바꿔 데스크톱 웹뷰 경로 문제를 피했다. 남은 일: Windows 실제 앱에서 보기 확인.

### M9 — UI/UX 구현 철학을 SDD 상위 원칙으로 고정 (완료, 문서)

- [x] 새 원본 [UI/UX 구현 철학](sdd/ux-principles.md)(UXP-1~8: 한 화면에 한 깊이, 요약 먼저·원문 한 번에, 통제는 항상 보임, 선택지 5개·권장 1개, 쉽게 만드는 데 추가 호출 없음, 쉬운 말, 사실만, 화면 간 일관성 / 세 깊이 한눈에·흐름 요약·자세히 / 요약 규칙과 신뢰 경계 / 쉬운 말 용어표 / 화면 점검표 8항목)과 [ADR-0042](sdd/architecture/decisions/ADR-0042.md). SDD 색인 첫 줄, ADR 색인 등록.
- [x] 충돌 해소: [정보 밀도 UI 정책](sdd/content-density.md)의 "AI 요약 없이 원문만" 문장을 "표시된 요약을 먼저 보일 수 있고 원문은 같은 자리에서 한 번에"로 수정(ADR-0042). 하위 정책 연결: progress-visibility, uxSimplification, product, workflow(UI 작업 시 철학 먼저·점검표), validation(화면 점검표 기록).
- [x] [v3 개선안](sdd/v3-improvement-plan.md)에 "3차 개선 제안 — 업무·회의 화면" 추가(문제 대조, 요약 출처, 화면 세 깊이, U1~U4, 결정 D1~D3). 제안이며 구현 전.
- [x] 검증: 상대 링크 검사.
- 다음: D1~D3 결정 후 U1(프런트만) → U2(요약 필드·명세 이관).

### M10 — 업무·회의 화면에 UI/UX 철학 적용 (완료, 오프라인·모의 렌더링 검증)

- [x] 결정(CEO, 권장안): D1 기존 호출에 요약 얹기, D2 권장안 미리 선택·결정은 CEO 버튼, D3 회의·업무 화면 먼저. ME-01(표시용 요약 필드 규칙), DM-03(분석 `요약:` 줄), UF-09(세 깊이 회의 창), 정보 밀도 감사표(업무 탭·회의 창), Validation(QA-41), implementation-map, v3 개선안 상태.
- [x] 엔진: 의장 계획 `topicSummary`, 제안·비판·수정 `summary`, 평가 `overview`·`recommendedId`·`issues`를 기존 호출의 출력 계약에 추가. `plain_summary`로 공백 정리·제어문자 제거·길이 제한, 잘못된 ID는 버리고 응답을 거절하거나 형식 수정 호출을 쓰지 않음. 패키지에 `overview`·`recommendedId`·`issues`, `Meeting.topicSummary`. 실행 목표는 CTO 분석 첫 줄 `요약:`을 `goal_summary`로 저장. 스냅샷 `goalSummaries`. 오프라인 시험 응답 갱신. 프로토콜 31.
- [x] 화면: 공용 `plainText.ts`(요약 부착 `withSummaries`, `headline`, 단계 한 줄 `stageDigests`, 쟁점 묶기 `meetingIssues`, 통제 한 줄 `controlLine`), `SummaryText`(AI 요약/원문 일부 표시 + 원문 보기), `goalTitle`로 홈·업무·Office·작업공간·토큰 화면 제목 통일. 회의 창 세 깊이: 한눈에(주제 요약, 회의 요약, 제안 ≤5·권장 강조·미리 선택, 쟁점 수, 통제 한 줄, 결정 버튼) → 회의 요약(단계별 한 줄, 쟁점 남음/확인 필요) → 자세히(단계 발언, 쟁점 개별 의견과 답, 모든 기록: 기존 결정 자료·추적·원기록·전송 기록·참여자). 회의 상태·단계 이름·보낼 내용 승인 문구를 쉬운 말로. 업무 탭: 회의·목표 제목 요약+원문, 결정 필요 일은 쉬운 문장+요청 원문 보기, 업무 목록은 산출물·쉬운 상태+지시/결과 원문 보기.
- [x] 검증(Linux 컨테이너): `cargo test --lib` 98 통과·1 무시(오프라인 회의 왕복에서 요약·권장·전체 요약이 저장되고 호출 수 그대로·요약 없는 응답도 수용, `plain_summary` 경계, 분석 `요약:` 줄), `cargo check` 경고 없음(기존 runtime 경고는 테스트 대상에서만), 세 프런트 `tsc`, `vitest` 51 통과(새 `plainText.test.ts` 5개), `build`·`build:v2`·`build:v3`. v3를 모의 브리지로 띄워 업무 탭 → 결정 열기 → 한눈에 → 회의 요약 → 쟁점 자세히 → 단계 자세히를 headless Chromium으로 확인, 페이지 오류 0.
- [x] 화면 점검표(ux-principles): 1 한눈에 첫 줄이 "회의가 끝났어요 · 결정해 주세요" ✓ / 2 깊이별 화면 분리 ✓ / 3 요약마다 원문 보기·표시 ✓(회의 전체 요약은 원문이 기록 전체라 `회의 요약 보기`로 연결) / 4 통제 한 줄·결정 카드 첫 깊이 ✓ / 5 제안 ≤5·권장 1·결정은 버튼 ✓ / 6 추가 호출 0 ✓(시험에서 호출 수 동일) / 7 깊이 1·2에 ID·해시 없음 ✓(깊이 3 쟁점에만 ID) / 8 `goalTitle`·`meetingAttention` 공유 ✓. 계획 승인 카드는 보안 범위를 전부 보여야 해 기존 형태 유지(예외 규칙).
- 남은 위험: 실제 Claude/Codex가 요약 필드를 채우는 비율은 미확인(없으면 원문 일부·결정적 문장). 앱 전체 쉬운 말 교체와 v1 회의 화면은 범위 밖. QA-41(Windows 실제 회의)은 CEO 실행 필요.

### M11 — 목표 보관과 설정 › 백업 삭제 (완료, 오프라인·모의 렌더링 검증)

- [x] 결정(CEO 요청): 목표·회의를 물리 삭제 없이 화면에서 숨기고, 설정의 `백업 삭제`에서 요약만 보여 주는 대화상자로 되돌리기·영구 삭제. [ADR-0043](sdd/architecture/decisions/ADR-0043.md)(신규, 색인), DM-04(보관은 상태가 아닌 표시 겹침), Security(영구 삭제 범위·확인·심볼릭 링크 미추적), [UF-10](sdd/userFlow/UF-10-archive.md)(신규, 색인), 정보 밀도 감사표, Validation(QA-42), implementation-map.
- [x] 엔진: 새 `archive.rs` — `goal_archive` 기록으로 보관(유휴 목표만: 실행 목표 new/completed/blocked이고 실행·통합 중 업무 없음, 회의는 결정 기록·중단·멈춤 뒤), 보관 시 그 목표의 외부 인력 계약 해제, 되돌리기, 화면에 남은 수정 목표·회의 초안 목표가 있으면 영구 삭제 거절, 확인 플래그 필수, 목표 소유 기록·이벤트 삭제 후 `goal_purged` 이벤트 1건, `<앱 데이터>/worktrees/<목표 ID>`만 삭제(실제 폴더일 때, 링크 미추적), 토큰 사용량 유지, 용량 추정. 스냅샷은 보관 목표와 소유 기록을 모두 숨기고 `archivedGoals` 요약(제목·종류·상태·날짜)만 준다. `run_goal`/`resume_goal`은 보관 목표를 거절. 명령 `archive_goal`·`restore_goal`·`backup_usage`·`purge_goal`, 프로토콜 32(프런트 최소 32).
- [x] 화면: 작업공간 머리글과 회의 창 한눈에 아래 `목표 보관`/`회의 보관` 2단계 확인(진행 중이면 이유만), 설정 탭 `백업 삭제`(카드 한 장: 개수 + `보관함 열기`), 보관함 대화상자(개수·대략 용량, 5줄 + `모두 보기`, 줄마다 제목 요약·종류·보관 날짜·용량·`되돌리기`·`영구 삭제…`, 영구 삭제는 줄 안 확인란을 선택해야 켜짐).
- [x] 검증(Linux 컨테이너): `cargo test --lib` 100 통과·1 무시(새 archive 시험 2개, 엔진의 실행·재개 거절은 시험 없이 코드 확인만: 보관·숨김·되돌리기·진행 중 거절 / 확인 없음·미보관·의존 목표 거절, 삭제 후 사용량 유지·작업 폴더 삭제), 세 프런트 `tsc`, `vitest` 56 통과(새 `archive.test.ts` 5개, 프로토콜 시험 32), `build`·`build:v2`·`build:v3`. v3를 모의 브리지로 띄워 업무 → 작업공간 → 목표 보관 → 확인 → `archive_goal` 호출, 회의 창 보관 버튼(진행 중 회의 이유 표시), 설정 › 백업 삭제 카드 → 보관함(7개 중 5줄 + 모두 보기) → 영구 삭제 확인란 전 비활성·후 활성 → `purge_goal{confirmed:true}` 호출을 headless Chromium으로 확인, 페이지 오류 0.
- [x] 화면 점검표(ux-principles): 1 카드 첫 줄 "보관한 항목 N개" ✓ / 2 목록은 대화상자에서만 ✓ / 3 보관함 제목은 요약만(CEO 요청: 다른 데이터 미표시, 원문은 되돌린 뒤 작업공간에서 확인 — 예외로 기록) / 4 영구 삭제는 줄 안 확인 ✓ / 5 줄마다 행동 2개 ✓ / 6 새 AI 호출 0 ✓ / 7 ID·해시 없음 ✓ / 8 `goalTitle`과 같은 요약 출처 ✓.
- 남은 위험: Git 프로젝트에서 삭제된 작업 폴더의 `.git/worktrees` 항목은 프로젝트를 건드리지 않으려고 정리하지 않는다(Git이 `prunable`로 보고 자체 정리). 큰 DB에서 용량 계산은 대화상자를 열 때 전체 객체를 훑는다. Windows 실제 앱 확인(QA-42)은 CEO 실행 필요.

### 실제 앱에서 아직 검증하지 못한 범위

- Windows 데스크톱에서 QA-39(합성 자료 출시 준비 파일럿: 계획 카드 → 승인 → 병렬 문서 작업 → 산출물별 QA → 인수)와 중단 복구 각 분기. 실제 Claude/Codex 호출은 이 세션에서 실행하지 않았다.
- QA-36/37/38 기존 열린 항목.
- 다음: Windows에서 `v2/start-windows.ps1 -DataDir .run-data\v3-pilot`로 QA-39를 수행하고 결과를 기록한다. PM/QA JSON 준수율이 낮으면 프롬프트를 조정하고 MAM-013/ADR-0037의 형식 수정 요청 방식을 실행 목표에도 적용할지 결정한다.

## v3 첫 개선 패키지 문서 인계 (2026-10-10, Codex)

- [x] 현재 소유 명세와 구현 경계(Product, Architecture/ADR-0004, Security, DM-03/04/05, AI-03, UX-01/02, GC-03/04, Progress Visibility, Validation), Office 코드와 v2 열린 QA를 기준으로 [v3 개선안](sdd/v3-improvement-plan.md)에 Claude Cowork 구현 인계안을 추가했다. Paperclip의 위임·원자적 선점·차단 인박스·감시 진단, Pixel Agents의 Office 상태 표현을 공식 자료 링크와 함께 설계 참고로 한정했다.
- [x] 범위: 세 개의 로컬 출시 준비 산출물, 단일 목표 화면, 명시적 담당/능력/파일 범위, 저장된 증거, 보수적 복구, 단계별 검증. 기존 엔진과 승인 경계를 유지하고 첫 구현 전에 소유 Spec·새 ADR을 갱신하도록 명시했다. 이번 마일스톤은 문서만 변경했고 프로젝트 코드·외부 서비스·실제 AI 호출은 실행하지 않았다.
- [ ] Claude 착수 시 v2 실제 Windows QA-36/QA-38 상태와 다른 작업자의 파일 소유를 확인하고, 명세/ADR → 배정·증거 → 복구·UI → 격리 파일럿 순서로 각 결과를 이 WORKLOG에 기록한다. 문서 링크와 제안 내용은 수동 확인했으며 앱 수준 동작은 검증 전이다.

## Meeting blocking edge cases — improvement plan (2026-10-09, Claude Cowork)

- [x] W0 (Codex): moved MAM-014 D1–D3 recovery and transport contracts into ME-01, ME-02, AI-03, Validation, UF-09 and QA-38; added ADR-0037 and its index entry. Scope: documentation only. Relative links and code checks are still pending; no provider call or Windows app run. Next: W1 tests and protocol/blocked-build implementation.
- [x] W1 (Codex): backend protocol 23 and v2 minimum 23; meeting blocks, stop, revoke and restart carry `blockedAt`/`blockBuild`, and the blocked card distinguishes older builds. Rust blocked-build test 1 passed; Vitest protocol tests 2 passed. A pre-existing Windows runtime test had a missing test-scope import and was repaired before Rust tests compiled. No Windows app or provider CLI was run. Next: W2 stdin transport, with provider switches disabled.
- [x] W2 (Codex): optional stdin bytes are written on a separate thread and closed while stdout/stderr drain; provider delivery has an explicit Claude/Codex/default argument table, both production switches remain off. Added `claude-stdin`/`codex-stdin` synthetic 50k-character harness modes, but did not run either real CLI or enable them. Execution-goal argument calls now fail before spawn beyond the conservative 30k UTF-16 command budget. Targeted 100k-byte stdin/no-deadlock Rust test and `cargo check --locked --all-targets` passed. Windows harness evidence remains required. Next: W3 all-stage budget property test.
- [x] W3 (Codex): plan validation measures the actual proposal packet and synthetic worst-case critique, revision and evaluation packets for both 2- and 3-person meetings. Response count/text/reference limits are enforced before a contribution enters later packets; the evaluator receives compact JSON with full current claims, critiques, responses and unshortened quotes. Template v4 was raised to v5 with W4. The required full-cap, quote-heavy property test passed. An attempted 80-character quote-shortening patch was rejected by automatic approval review for violating the user's no-truncation rule; it was replaced by full-content rendering and bounded accepted replies. Not verified: live model adherence to those limits.
- [x] W4 (Codex): accept the first complete JSON object amid prose; normalize equivalent case/zero-padded claim and critique IDs with an audit record; save invalid raw replies; create at most one visible format-repair packet per member/stage when a planned spare call remains; second invalid or no spare blocks. Default plan reserves two calls within 12. Tests for wrapped JSON, ID normalization, one repair, second failure and zero spare passed. The repair packet carries only the original packet ID and violation, not the raw answer; whether a stateless real CLI can meaningfully repair from that limited context remains unverified. Protocol 25 was followed by 27 as W5/W6 commands were added.
- [x] W5 (Codex): added `retry_meeting_packet` for a blocked, still-approved meeting with remaining calls. It creates a new revision for the failed member/stage under the original plan; an uncertain outcome needs explicit duplicate confirmation. Restart expires plan approval, and reapproving in plan mode now forces exact packet confirmation for any uncertain predecessor instead of automatically resending it. Offline tests covered failed critique → retry → completed package, missing duplicate confirmation, exhausted cap, and restart reapproval. Not verified in a Windows app.
- [x] W6 (Codex): before a proposal, a changed source blocks and offers an explicit approved-snapshot choice; later stages use the frozen snapshot and show a change notice. Connection fingerprint v2 covers kind/executable/endpoint/args/model and excludes capabilities; unchanged legacy fingerprints still match the old algorithm. Quote matching normalizes CRLF/LF and repeated whitespace while reporting original byte offsets. Busy-member reapproval errors list eligible alternatives without replacing anyone. Tests for snapshot continuation, v2 capability drift and CRLF quote offsets passed. A legacy v1 fingerprint cannot be reconstructed if its old capabilities have already changed; that meeting conservatively requires renewed plan review. Not verified in a Windows app.
- [x] Final offline gate (Codex): `cargo test --locked --lib` 83 passed, 1 ignored; `cargo check --locked --all-targets` passed; `npx tsc --noEmit -p .` and `-p v2` passed; `npx vitest run` 33 passed; `npm run build` and `npm run build:v2` passed; `npm run provider:test -- offline` passed. Backend/frontend meeting protocol is 27. No desktop app, real Claude/Codex CLI, or provider-specific 50k stdin harness call was run. Production stdin remains disabled for every provider. Next: QA-38 on Windows with synthetic isolated inputs and user-approved real provider calls; record per-provider harness outcome before changing the delivery table.

- [x] Investigated why meetings can still block after plan approval and wrote [MAM-013](sdd/multiAgentMeeting/MAM-013-proposal.md): 13 edge cases (evaluating/critiquing/revising packet length, Windows command-line limit shared with execution goals, model output format drift, call failures, source edits during a meeting, connection fingerprint including capabilities, call budget, restart, stale backend/stored block reasons, chair source choice, CRLF quote matching, busy members) and a P0–P5 plan with acceptance tests.
- [x] The repeated "안전 길이 12000자 … 새 회의를 만드세요" text exists only in pre-fix code: either an app window started before the rebuild or a block reason stored before the fix. Recorded as E10 (protocol number was not raised).
- [x] User adopted D1–D3 ("셋 다 채택으로 가자"), recorded as [MAM-014](sdd/multiAgentMeeting/MAM-014-decision.md). Implementation handoff written as [MAM-015](sdd/multiAgentMeeting/MAM-015-revision.md) (W0 owning docs + ADR-0037, W1 stale backend, W2 stdin transport, W3 all-stage length budget, W4 format repair, W5 in-stage retry, W6 non-blocking changes).
- [x] Implement MAM-015 W0–W6 offline contract and tests; Windows QA-38 and real provider verification remain open.

## Meeting packet length checked at plan validation (2026-10-09, Claude Cowork)

- [x] Cause: plan validation accepted up to 24 KB of sources, but the proposing packet sends every source in full to each participant and the packet limit was 12,000 characters, so a valid-looking plan (five SDD files, about 20 KB) was approved and then blocked at the CTO's 23,671-character proposing packet.
- [x] User decision: align validation and raise the limit. `meeting.rs`: the packet limit is now 30,000 UTF-16 units measured as the quoted Windows command-line argument (`command_arg_len`; Windows refuses command lines over 32,767), sources are capped at 16,000 characters per file and 26,000 in total, the plan validator builds each participant's real proposing packet and reports an oversized one as a plan error with per-file sizes, and the chair request states the limits and asks for 2–4 needed files. ME-01/ME-02 and the plan card label updated.
- [x] `cargo test --locked --lib` 70 passed, 1 ignored (new `oversized_proposing_packets_are_plan_errors_before_approval`, command-line quoting cases); `cargo check --locked --all-targets` clean; v2 `tsc`; `vitest` 31 passed. Not verified: no real Claude/Codex call with a 20–30k-character argument on Windows yet.
- [ ] Restart the app, open the blocked meeting, `계획 검토로 재개`, approve again (its packets are about 23,800 units after quoting), and confirm the real CLIs accept them.

## Meeting plan approval false "sources changed" (2026-10-09, Claude Cowork)

- [x] Cause, from the user's `.run-data/meeting-qa` data: all five source files were byte-identical to the review, but the chair's reason for `sdd/specImport/SI-01-import.md` was cut at 200 characters right after a space (`… against `). Validation trimmed before truncating, so re-validation at approval trimmed again, produced a different digest, and the digest mismatch was reported as "자료가 바뀌었습니다".
- [x] Fix (`meeting.rs`): `bounded_trim` (trim → cut → trim) for objective, constraints, success criteria, stances and source reasons, so a second pass is a no-op. Approval now checks the stored revision's own digest, current validation errors, and source path/hash pairs separately; a real source change names the changed files. Plans already stored with a trailing space approve without re-saving.
- [x] `cargo test --locked --lib` 69 passed, 1 ignored (new `approval_accepts_long_trimmed_text_and_names_changed_sources`); `cargo check --locked --all-targets` clean. Not verified in the Windows app yet.
- [ ] Restart with `v2\start-meeting-qa.cmd` and press `계획 승인 후 진행` on the existing plan.

## Claude Code connection check on Windows (2026-10-09, Claude Cowork)

- [x] Diagnosed the `claude.cmd` connection: the first `failed` status was recorded at 18:28:28 KST while Claude Code 2.1.295 was replacing `claude.exe` (finished 18:28:29). After a re-check the version probe passed but the login-state probe still failed.
- [x] `discovery.rs`: version probe timeout 3 s → 15 s, `auth status` (Claude) and `login status` (Codex) 3 s → 20 s with stdin closed; Claude `auth status` stderr is read on a helper thread, JSON is accepted when surrounded by update notices, and failures report elapsed time, exit code and the first output line with account-like words masked.
- [x] `로그인 창 열기` on Windows now starts PowerShell through `cmd /c start` with `-EncodedCommand`, so the console gets its own input/output instead of inheriting the app's standard handles (the likely reason no usable login window appeared).
- [x] Added `v2/diagnose-claude-connection.ps1` / `.cmd` (runs the same `--version` and `auth status` commands, prints timings and output shape without identity fields; `-TryLogin` opens the login console).
- [x] `cargo test --locked --lib` 68 passed, 1 ignored; `cargo check --locked --all-targets` clean. Not verified: the Windows-only branch was not compiled here (Windows target unavailable in the container) and no real Claude CLI was run.
- [ ] Restart the app with `v2\start-windows.ps1`, press `이 경로로 다시 검사` for Claude Code, and if needed `로그인 창 열기`; record the status text.

## Office-character meeting first implementation (2026-10-09, Claude Cowork)

- [x] MAM-012 response: C1–C4, C6, C8–C10 accepted, C5 partially accepted, C7 (hash chain, Markdown export) deferred. The folder index records MAM-012 and no further review document; contracts moved to owning Specs.
- [x] Migration: ME-01 (Office roles, chair plan, validation, stages, traceability, decision/handoff), ME-02 (plan approval, exact per-stage packets, reservation checks, member-attributed usage), UF-09, Security (D3/D4 boundary), AI-03 (shared read-only profile), DM-01 (Agent busy state), DM-04, product, UX-01/02 (Home dropdown), Validation, [ADR-0036](sdd/architecture/decisions/ADR-0036.md) (partially supersedes ADR-0035), QA-38, implementation map.
- [x] Backend: `read_only_profile` (Claude CLI → `claude_cli_plan`, Codex CLI → `codex_cli_read_only`, debug fixture only with `UTOPIA_MEETING_FIXTURE=1`; Gemini/OpenCode/custom CLI/HTTP/plugin excluded with a reason); `ProviderTransport` calls `provider::execute_with_observer(..., writable=false)`. PM-first, CTO-fallback chair; Home start creates the meeting Goal and exactly one chair planning packet. Immutable plan revisions with digest, re-validated on every CEO edit and at approval; `plan` (default) or `per_stage` approval; calls outside the approved plan are refused in `reserve()`; stop/revoke share the reservation lock; outcome-unknown calls are never re-sent; restart expires plan approval. Shared busy rule across meetings and execution goals (`engine::start` refuses reserved members). Engine-built trace graph with response-derived dissent. Schema 4 (no-op bump), earlier-model meetings close as read-only `legacy_closed`. Token usage records the real member and meeting. Backend protocol 22.
- [x] Frontend: Home `명령 방식` dropdown (`목표 수행`/`회의`) with chair, connection, model and read-scope disclosure, separate submit commands, draft lineage only in goal mode; meeting workspace plan card (edit → new revision, errors block approval, unsaved edits block approval), per-stage packet review, revoke/stop, resume to plan review, trace explorer, legacy notice; shared status labels for planning/plan review/legacy; Office rail shows chair, participants, evaluator and the member being called. The old planner dialog and grant/scope commands were removed.
- [x] Tests: `cargo test --locked --lib` 66 passed, 1 ignored (pre-existing live Codex test); `cargo check --locked --all-targets` clean (removed the stale unused test import in `runtime.rs`); `tsc --noEmit` for both frontends; `vitest run` 31 passed (new `src/meetingPlan.test.ts`, extended `meetingStatus.test.ts`); `npm run build` and `npm run build:v2`. UI rendered in headless Chromium against real engine snapshots (Home meeting mode, plan review, plan errors, per-stage packets, trace, Office rail) with a mocked Tauri bridge; the temporary export test was removed.
- [ ] Not verified: no Windows app run and no real Claude/Codex CLI call. All provider behavior above is offline-tested only (script transport with the real eligibility rule). Windows QA-38 steps 8–10 are the remaining blocker.
- [ ] Later scope: N rounds and other protocols (`position_rebuttal`), review panel and chair participation, contribution hash chain, Markdown export through a separate project-file write approval, Office character animation for meetings.

Next action: run QA-38 on Windows, first with `v2\start-meeting-qa.cmd` (steps 1–7), then with real Claude/Codex connections (steps 8–10), and record PASS/FAIL per step.

## Office-character meeting redesign proposal (2026-10-09, Claude Cowork)

- [x] Recorded the user's intended model as [MAM-009](sdd/multiAgentMeeting/MAM-009-revision.md): existing Office team members are the participants (snapshotted, no new Agents), a chair (PM, else CTO, by fixed ID) proposes objective/constraints/participants/protocol/rounds/caps/sources as validated JSON, the engine enforces hard caps and busy/connection rules, and Home gets one composer with a `목표 수행`/`회의` dropdown.
- [x] Recorded the user's decisions as [MAM-010](sdd/multiAgentMeeting/MAM-010-decision.md): D3 meetings use the same read-only trust boundary as execution goals (Claude/Codex CLI allowed with `plan`/`read-only`; connections that ignore `writable` excluded); D4 one CEO plan approval by default with optional per-stage packet review, superseding MAM-005 D1. MAM-005 D2 stays.
- [x] Traceability proposal: structured argument graph as source of truth (engine-assigned IDs, `targets`/`responds_to`/`revises`/`cites`/`supports`, hash-chained contributions), Markdown rendering for packets and an exported meeting transcript; agent-written Markdown files were rejected because they need project writes and cannot be validated.
- [x] Codex completed [MAM-011](sdd/multiAgentMeeting/MAM-011-critique.md): C1–C10 cover chair selection/first call, immutable plan validation, D4 plan approval and stage confirmation states, D3 provider parity, round budgets, graph traceability, hash/Markdown limits, shared Agent occupancy, Home mode safety, and legacy schema. Reviewed current source and owning SDD without running the app or CLI. D3/D4 and MAM-005 D2 remain fixed.
- [x] Claude Cowork responded as [MAM-012](sdd/multiAgentMeeting/MAM-012-response.md) (chair plans only in the first slice; review panel deferred) and the contracts were migrated and implemented — see the section above. Original item: including the chair participation question; then migrate accepted contracts to ME-01/02, Security, AI-03, DM-01/04, UX-01/02, UF-09, Validation, QA-38 and a new ADR before code. Prioritize a narrow first live Claude/Codex Office-character meeting with plan approval, bounded calls, traceable dissent, shared occupancy and migration guards.

Checkpoint (superseded): this section changed documents only. The migration and first implementation landed in the next section above; the MAM-008 closed-text gate and fixed slots no longer apply to new meetings.

## v2 meeting engine, transmission gate, decision package and UI (2026-10-09, Claude Cowork)

- [x] Re-verified the offline first slice before building on it. Correction: the earlier checkpoint's "`cargo test --lib` passed 60 tests" did not hold on the current tree — 58 tests ran and `storage::tests::simultaneous_goal_questions_enqueue_only_one_pending_turn` failed on the unchanged base because the new Goal-kind guard requires the goal to exist and the fixture never created one. The fixture now creates an execution goal and also asserts meeting/missing goals are rejected.
- [x] Step 1 — offline meeting engine (`src-tauri/src/meeting.rs`): 3 proposals → 3 targeted critiques (`product→ux→security→product`) → 3 revisions → 1 evaluation, at most 10 calls. Global claim/critique/recommendation IDs, quote checks against frozen source bytes, assumptions/unknowns, accept/rebut/defer with evidence-required rebuttal. Invalid replies are kept as raw evidence (`sent_invalid`) but never become contributions; the meeting blocks. Stop, resume (new revisions only for missing slots, `prior_outcome_unknown` flag), partial closure, restart → `outcome_unknown`, call cap, duplicate commands.
- [x] Step 2 — transmission gate: approval binds packet ID, preview revision, SHA-256, template version, dispatch-contract version and the destination's closed-text policy. `reserve()` rechecks bytes/digest/revision/fingerprint/approval/grant/cap inside the same SQLite transaction (`Store::transact`) used by stop and revocation. Failures and unknown outcomes are never retried. Offline-slice and old-template packets/approvals expire on start; approvals recorded while the destination was ineligible can never dispatch; a dispatch refused for eligibility voids the stage's approved packets.
- [x] Step 3 — real providers stay fail-closed. `AppTransport` never calls `provider::execute*`; every CLI/HTTP/plugin connection is refused by `closed_text_eligibility` because no provider-specific closed-text profile or harness evidence exists. A debug-only offline responder (`UTOPIA_MEETING_FIXTURE=1`, labelled "AI 아님") exercises the loop and UI without transmitting anything. Telemetry per attempt: start/finish, duration, command-line length, connection name/model/fingerprint, isolation policy, outcome, reported/estimated usage; meeting usage records use `meeting:<meeting>:<slot>`.
- [x] Step 4 — decision package and UI: recommendations, dissent (rebut/defer/unanswered/flagged), claims with source status and evaluator verdict, assumptions/unknowns, alternatives, provenance, independence note, usage provenance, termination reason, SHA-256. `record_meeting_decision` (adopt/revise/defer) completes the Goal and returns only a draft; `submit_goal` with lineage validates the adopted package hash and project, then stores `goal_lineage`. Shared selector `src/meetingStatus.ts` drives Home, Work, Office rail and open work; Work no longer offers QA-limit controls for meeting goals. Backend protocol 21.
- [x] Step 5 — `미리 승인`: implemented and tested only behind the closed-text gate (offline responder today). Single meeting, remaining calls, fixed source hashes, destination fingerprints, template, 12,000-char cap, 1–120 min expiry; revocation, restart, scope mismatch, meeting stop/end return to per-packet mode. Unavailable for real connections in UI and backend.
- [x] Docs: ME-01 (stage contracts, resume, decisions, lineage), ME-02 (approval binding, legacy expiry, telemetry, offline responder), AI-03 (separate meeting transport), QA-38, implementation map.
- [x] Added `v2/start-meeting-qa.ps1` (sets `UTOPIA_MEETING_FIXTURE=1` for its process only, defaults to `.run-data\meeting-qa`, passes `-CheckOnly`, restores the variable) and a double-click `v2/start-meeting-qa.cmd` (`-ExecutionPolicy Bypass` for that process only). Parsed with PowerShell 7.4 (0 errors) and exercised against a stub launcher in a Linux container: default and explicit `-DataDir`, `-CheckOnly` pass-through, variable set during the run and removed afterwards. Not yet executed on Windows PowerShell 5.1 or with the real launcher.
- [ ] Windows desktop run of QA-38 on isolated data (not run: the change was built and tested in a Linux container; the Windows toolchain was not invoked).
- [ ] Ubuntu desktop QA, QA-36/37 (unchanged, still open).
- [ ] Provider-specific closed-text profile + harness proving denial of tools, MCP, web, outside-file reads, writes and third-party transmission for at least one real connection; only then open real dispatch and real-connection `미리 승인`.

Checkpoint: verified in a Linux container on a copy of the repository: `cargo test --locked --lib` 68 passed, 1 ignored (pre-existing live Codex test); `cargo check --locked --all-targets` clean except the pre-existing unused-import warning in `runtime.rs` tests; `tsc --noEmit` for both frontends; `vitest run` 28 passed; original and v2 Vite builds. The v2 UI was rendered with real engine snapshots (awaiting approval, finished package, real-connection meeting) through a mocked Tauri bridge in headless Chromium: Home/Work/Office/open work show the same meeting state, adoption invoked only `record_meeting_decision` and filled the draft, and the real-connection dispatch showed the fail-closed reason. That is not a Tauri/WebView2 or Windows check. Next: run QA-38 on Windows with `-DataDir .run-data\meeting-qa`; design the first provider closed-text profile and harness probes (outside-file marker, write attempt, tool/MCP/web denial, command-line limit, version pinning) before any live meeting call.

## Multi-agent meeting cross-review folder (2026-10-09)

- [x] Read AGENTS.md, the SDD authoring principle, and the meeting proposal; compare its claims with Security, DM-01–05, AI-01/03/04, GC-01/02, the v3 plan, and `engine.rs`/`provider.rs`/`runtime.rs`/`storage.rs`/`model.rs`.
- [x] Create `sdd/multiAgentMeeting/` with an index that owns document IDs, types, relations, review order, writing rules, status, and exit to owning Specs. Move the proposal to MAM-001 (links re-based only) and leave a body-less move notice at the old path.
- [x] Record Claude Cowork's critique as MAM-002 (issues C1–C9: goal binding, shared agent pool, minimum roles, quality gate, multi-provider transmission grant, unenforced read-only/input scope, token guarantees, first slice, overlap with the v3 evidence contract).
- [x] Point AGENTS.md, the SDD index, and the authoring table to the folder index.
- [x] Codex wrote MAM-003 with direct code/SDD checks for C1–C9, one new issue N1, and a decision summary. C5 transmission scope and C9 first-pilot order need user decisions before implementation.
- [x] Codex wrote MAM-004 as a separate revision reflecting MAM-003 and UX-01–03 without rewriting MAM-001/002/003. It proposes a visible `awaiting_decision` Goal state, keeps the five-menu navigation and one goal workspace, and leaves D1/C5 and D2/C9 unresolved.
- [x] Record the user's D1/C5 and D2/C9 choices in MAM-005: per-packet preview/approval by default with an explicit meeting-scoped `미리 승인` mode, and v2 meeting as the first validation case before using the built program to plan and implement distinct v3 improvements.
- [x] MAM-006 (Claude Cowork, 2026-10-09): critique of MAM-004 that keeps MAM-005 D1/D2 as fixed requirements. Issues C1–C10. First-slice blockers: C1 undefined in-progress `Goal.status` for meetings lets `recover_interrupted()` → `can_retry_before_write()` → `start()` reach the CTO→PM engine, and the original UI still shows `run_goal` for any `new` goal; C2 older binaries drop an unknown `kind` field on rewrite, so the first meeting migration needs `SCHEMA_VERSION` 3; C4 `decide_approval()` flips status without binding the previewed payload digest; C7 CLI calls use the project root as `cwd` (which contains `.run-data/organization.sqlite`) and no closed text-only mode is verified. `미리 승인` stays unavailable until C5 (checkable scope, connection fingerprint, restart/revocation expiry, transmission log) and C6 (Security scoped-grant wording limited to closed-flow participants) are adopted. C9 also records that self-implementation in this non-Git folder exceeds the 512 MiB snapshot limit (`src-tauri/target` is 9.4 GB). Document review only; no app run, model call, code, Spec, or ADR change.
- [x] Codex wrote MAM-007 answering MAM-006 C1–C10 and all eight closing questions with direct code/SDD checks. It accepts C1/C2/C10, partly accepts C3–C9, and adds N1 for other Goal-changing commands without a kind guard.
- [x] Codex wrote MAM-008 as the second revision; it preserves MAM-005 D1/D2 and incorporates MAM-007 C1–C10/N1. The folder exit condition was checked: two revisions, no rebutted issues, and the user explicitly requested implementation after MAM-008.
- [x] Migrate MAM-008 into owning Specs/ADR/user flows/Validation: ME-01/02, ADR-0035, UF-09, Product/Domain/AI/Security/GC/UX, and Validation now own the intended contract.
- [x] Implement the first offline v2 meeting slice: schema 3 and `Goal.kind`, execution-command guards, atomic Meeting/packet/TransmissionApproval storage, source and connection fingerprints, exact preview decisions, restart recovery, Home entry, meeting workspace, and open-work surfacing. The dispatch command always fails closed until a provider passes effective closed-text isolation verification.
- [x] Implement and verify the three-proposal → cross-critique → revision → evaluation loop, usage tracking, stop/resume, decision package, CEO handoff draft, and scoped `미리 승인` behind the gate (see the section above). Actual closed-text provider dispatch remains fail-closed.
- [ ] Run Windows and Ubuntu desktop QA-36 on isolated data; verify real connection isolation before any live meeting call.

Checkpoint: [MAM-008](sdd/multiAgentMeeting/MAM-008-revision.md) has been transferred to owning documents and ADR-0035. The first code slice builds: `cargo test --lib` passed 60 tests (1 pre-existing live-model test ignored), original and v2 frontend builds passed, and 25 frontend tests passed. The UI can create a meeting, freeze selected UTF-8 sources and 3+1 connection slots, prepare three exact proposal packets, and record per-packet approval/denial; source or connection changes reject approval. The legacy UI hides meeting Goals so it cannot show execution controls for them. This is **offline only**: no provider call, critique, revision, evaluation, decision package, scoped grant, or CEO handoff exists yet. The explicit dispatch command is fail-closed. The installed app/CLI isolation and Windows/Ubuntu QA were not run. Next: establish an effective closed-text provider harness and its connection fingerprint, then implement the stage loop and dispatch reservation; rerun QA before exposing `미리 승인`. D2's self-implementation folder/Git choice remains for that later stage.

## Proposed v2 multi-perspective AI meeting specification (2026-10-09)

- [x] Compare the requested meeting loop with the current CTO → PM → developer → QA engine and its post-integration QA repair loop.
- [x] Write a reviewable, unimplemented proposal covering team/process presets, independent proposals, targeted cross-critique, revision, quality gates, usage bounds, recovery, result handoff, and security.
- [x] Link the proposal from the SDD index and AGENTS.md for Claude Cowork critique.
- [x] Obtain Claude Cowork's code- and SDD-grounded objections and alternatives to MAM-001, respond as MAM-003, revise as MAM-004, and record the user's C5/C9 decisions as MAM-005.
- [x] Obtain Claude Cowork's MAM-006 critique of MAM-004 with MAM-005's decisions and answer C1–C10 in MAM-007.
- [x] Incorporate the MAM-007 response in the second revision MAM-008 and check the cross-review exit conditions.
- [x] After acceptance, update owning Specs, user flows, Validation, and ADR-0035 before implementation.

Checkpoint: [the original v2 meeting proposal](sdd/multiAgentMeeting/MAM-001-proposal.md) was superseded by MAM-008 and transferred to owning Specs; the current implementation state and remaining gates are recorded in the first WORKLOG section. The existing QA repair loop remains distinct from the meeting process.

## v3 first implementation slice audit (2026-10-09)

- [x] Inspect current goal planning, task dispatch, file integration, QA, report, model, and v2 goal display against the proposed cross-functional pilot.
- [x] Identify the file-changing-task assumption, implementer-only assignment, free-form QA, and missing structured evidence as the first gaps.
- [x] Define the first vertical slice in the v3 plan: locally sourced launch-preparation documents, approved file outputs, independent review, and one CEO handoff.
- [ ] Update owning Specs and record a new ADR for the task/output model before changing engine code.
- [ ] Implement and verify the first slice in an isolated fixture and actual Windows desktop flow.

Checkpoint: this audit identified the first **v3-specific** coding slice as a general evidence-bearing non-code deliverable contract, exercised with a small launch-preparation goal. Existing `engine.rs` requires every PM task to change named files and routes tasks to implementer roles; `Task` stores file paths and result text, while QA/report are free-form text. No v3 runtime code was changed in this audit. [MAM-005](sdd/multiAgentMeeting/MAM-005-decision.md) now sequences v2 meeting validation before v3 implementation. When v3 begins, define its contract and ADR, then implement the smallest end-to-end path without weakening the existing approval boundary.

## v3 scope correction: company, not app factory (2026-10-09)

- [x] Recheck the product goal and the first v3 candidate list after the user's correction.
- [x] Replace the app-only ranking with cross-functional business-goal candidates; keep software work as one application.
- [x] Update the AGENTS.md handoff and SDD index to point to the corrected plan.
- [x] Audit the current engine's research, non-code artifact, and evidence paths; the limitations and first implementation slice are recorded above.

Checkpoint: the earlier React/TypeScript feature-addition recommendation below is superseded. The first proposed pilot is now preparation for a new product or service launch, ending in a human decision package rather than automatic publication. This is a product hypothesis; no runtime code changed. Next: inspect actual target users' goals and the current engine's non-code capabilities, then define one real pilot with explicit inputs, outputs, and evidence.

## v3 first customer-job focus (2026-10-09)

- [x] Read the SDD entry point, product/security/workflow ownership, v2 UX plan, ADR-0033, and affected v2 controller before planning.
- [x] Rank five repeatable customer jobs and select a small feature addition in an existing React/TypeScript Git project as the first proposed pilot.
- [x] Record pilot boundaries, evidence to collect, and a Codex/Claude Cowork handoff entry in the SDD index and AGENTS.md.
- [ ] Validate the candidate with actual users and paid-pilot interest; finish open v2 desktop QA before claiming a delivered v3 benefit.

Checkpoint: [v3 improvement plan](sdd/v3-improvement-plan.md) is a product hypothesis, not implemented behavior or proof of demand. It specifies only item 1 of the four proposed improvements; evidence-based delivery, onboarding, and delegation predictability remain future design work. No runtime code changed. Next: review the candidate with target users, choose one real feature request and completion criteria, then inspect the affected implementation and owning Specs before any v3 code change.

## v2 attention sound and Windows notifications (2026-10-09)

- [x] Inspect saved approval/goal transitions, current Tauri toolchain, notification support, and UI ownership.
- [x] Specify trigger, privacy, preferences, deduplication, and verification before code.
- [x] Record transport/version/installer choice in ADR-0034.
- [x] Implement shared notification detection, v2 settings, native delivery and sound.
- [x] Build and test transition logic, v2 UI, Rust integration, and installer entry; record desktop QA limits.
- [ ] Install the generated v2 Windows package in a QA environment and run QA-37; run Ubuntu desktop QA separately.

Checkpoint: ADR-0034 fixes the saved-snapshot transition detector and native-plugin boundary. The 2.3.0 notification plugin compiles on Rust 1.89; Tauri JS API/CLI were pinned to the existing Rust 2.11 line after the installer check exposed a 2.12 mismatch. V2 Settings > 알림 has separate opt-in switches and sample actions. The frontend uses fixed, brief notification text and an app-generated Web Audio cue. First successful snapshot becomes a quiet baseline; new pending approval and newly blocked/failed goal states signal once. `cargo check --locked`, original and v2 frontend builds, and 24 frontend tests passed. A debug installer attempt could not replace the running dev exe (Windows access denied). The first optimized installer mistakenly bundled `provider-harness.exe`; setting Cargo `default-run` to `ai-organization-os` corrected this. The final Tauri NSIS build reports `target/release/ai-organization-os.exe` and produced `src-tauri/target/release/bundle/nsis/Utopia Company v2_0.1.0_x64-setup.exe` (4,639,316 bytes). This is a build artifact, not a deployed release. The installer has not been installed, and real notification-center delivery, sound, permission denial, restart persistence, and Ubuntu behavior remain QA-37 items. Next: run QA-37 from an installed Windows app with isolated test data, then the Ubuntu desktop flow.

## v2 UI complexity reduction (2026-10-08)

- [x] Inspect the simplification plan, App controller, reusable components, launch configuration, and security boundary.
- [x] Update the owning UX spec and component replacement rule before code.
- [x] Record v2 frontend/backend sharing decision in ADR-0033.
- [x] Implement v2 source and Windows/Ubuntu launchers while reusing the existing backend and stable components.
- [x] Build v2 and original UI, run focused checks, and record actual QA limits.
- [ ] Run QA-36 in the actual Windows app and Ubuntu desktop; record navigation, persistence, approvals, and novice-task observations.

Checkpoint: `v2/` now has a separate entry and launchers. `V2Navigation`, `V2Home`, `V2Work`, `V2Projects`, and `V2Connections` are typed screen components; existing Office, token, goal conversation, team settings, picker, and import panels are reused. Five menus replace six; project switching is global; settings owns project, AI, team, and spec details. Goal conversation, approval dialog, and Rust/SQLite actions stay on the existing engine. `npx tsc --noEmit -p v2/tsconfig.json`, `npx vite build --config v2/vite.config.ts`, `npm run build`, and 19 frontend tests passed. A v2 Vite server on port 1421 served the page and transformed `src/App.tsx` with HTTP 200. The Windows launcher parsed and passed `-CheckOnly`; port 1420 was already occupied by an existing server, so the v2 Tauri window was not opened. Eleven changed documentation files had no broken relative links. Ubuntu launcher syntax and desktop flow are unverified. The controller/dialog wiring remains copied in `v2/src/App.tsx`; extract it into a shared typed module in a separate structural slice. Next: close the existing app/port 1420 process, launch v2 with isolated `-DataDir`, perform QA-36 and record observations; then repeat the core flow on Ubuntu.

## Whole-app UX complexity reduction plan (2026-10-08)

- [x] Inspect the six top-level menus, shared goal workspace, Office/team settings, project SDD panel, and token view against UF-01–UF-08.
- [x] Define a smaller navigation model and one primary action per first screen without hiding required approvals.
- [x] Record per-menu content priority, detail destination, and cross-flow changes as an SDD plan.
- [x] Add staged implementation and novice-task acceptance checks; validate document links and record the next action.

Checkpoint: the current UI has six top-level tabs plus a project switch and open-work launcher. CEO desk, board, Office, and goal conversation repeat goal status/task/QA information; Projects includes both folder registration and the full SDD import workflow; AI Connections exposes discovered, missing preset, custom, and saved lists together. The five-item policy limits list length but not competing sections. `sdd/uxSimplification/` now proposes five primary destinations (홈, 업무, Office, 토큰, 설정), a fast global project picker, a single goal detail owner, per-screen information priorities, and six implementation/QA stages. All 49 links in the plan/index files resolve. This is based on source inspection; actual screen baselines and novice-task timing remain to be captured before implementation. Next: capture baseline screens/tasks, decide the menu IA in an ADR, then implement UX-03 slices and update UF-01–UF-08 as actual paths change. No UI code or live desktop behavior changed in this turn.

## Topic-folder SDD principle and existing-document migration (2026-10-08)

- [x] Inventory SDD files, headings, existing folder indexes, and affected links.
- [x] Define the split threshold, single-owner rule, relationship vocabulary, and exceptions in the SDD authoring principle; record ADR-0032.
- [x] Apply the rule to four qualifying existing multi-entity topics with topic indexes and 16 linked entity files.
- [x] Update SDD entry points and references without breaking legacy links; review single-file topics that should remain intact.
- [x] Validate all Markdown links, concept ownership, and WORKLOG resume notes.

Checkpoint: `sdd/authoring.md` defines the two-part split test, one-rule owner, migration procedure, and audit of current folders; ADR-0032 records the structural choice. Goal conversation, external SDD import, AI integration, and domain were moved into four indexed folders with 16 entity files and typed relationships. Old root paths now contain only relocation notices; current SDD links point to the topic indexes. Product, security, validation, architecture, workflow, and compact policies remain single owners because splitting shared invariants would duplicate rules. The migration checked the expected original section/entity counts and resolved 364 local links across 87 Markdown files. No runtime code changed, so desktop behavior and QA status are unchanged. Next: when any of these concepts changes, edit its owning entity, adjacent relation links, user flow, QA mapping, and this worklog together.

## Linked user-flow SDD graph (2026-10-08)

- [x] Locate the existing MVP sequence and feature-specific interaction rules; avoid copying their normative contracts.
- [x] Define stable flow IDs, typed relations, file format, and ownership boundary in ADR-0031.
- [x] Add `sdd/userFlow/` index and eight linked CEO journeys covering setup, execution, conversation, approval, recovery, external SDD, and observation.
- [x] Link the graph from the SDD/product indexes and require flow updates in the development workflow.
- [x] Check all relative links in nine user-flow Markdown files and review branch wording against the owning Specs; manual product walkthrough remains separate from documentation review.

Checkpoint: the user-flow layer is a Markdown navigation graph, not a runtime ontology or a second authority for domain and approval rules. Each `UF-##` file names entry, visible actions, success/failure branches, typed links, and owning Specs/QA. All relative links in the index and eight flow files resolved. Next: review a future feature change by traversing its adjacent flows and add any missing real-world branch found in desktop QA. No desktop QA was run for this documentation change.

## Tauri command registration warning and UI reachability (2026-10-08)

- [x] Confirm `create_portable_spec` and `set_goal_review_limit` are invoked from the UI but missing from the Tauri command registry.
- [x] Register both existing commands and raise the backend protocol to 19 so a stale desktop process is detected.
- [x] Run `cargo check --locked` with no dead-code warnings and `npm.cmd run build` successfully.
- [ ] Restart the Windows app and manually confirm the existing-profile portable file action and a goal's QA limit change (QA-28 and QA-19).

Checkpoint: the two functions were implemented and had live frontend callers; only their `generate_handler!` registration was missing. No new behavior or ADR was needed because the existing SDD already specifies both actions. The old process stays unchanged until the app is restarted. Next: close the current desktop app, relaunch, exercise both controls, and record observed results.

## In-app Git bootstrap from blocked goal conversation (2026-10-08)

- [x] Trace the metadata-only integration failure and current advice-only conversation path.
- [x] Specify fixed Git bootstrap scope, approval boundary, and ADR-0030.
- [x] Implement read-only file preview and backend checked Git bootstrap operation.
- [x] Add scope selection and explicit approval in Goal Conversation, including old saved answers.
- [x] Verify selected-file commit, stale preview rejection, unborn-repository retry, and exact blocked-task QA gate with isolated fixtures; refresh code path compiles.
- [x] Run Rust/frontend tests and build; add QA-35 for remaining desktop verification.
- [ ] Restart the Windows app and run QA-35 against an isolated blocked goal with a connected QA agent. Record actual UI, commit, restart, and QA evidence; Ubuntu runtime verification is separate.

Checkpoint: the app now treats a blocked Git-init/first-commit task as a fixed operation with a read-only file preview, explicit CEO selection of files and commit author/message, and backend revalidation. It stages only approved files, creates one local `main` commit, records approval/outcome, and refreshes the same project's Git mode. Saved AI advice remains historical text; it is never parsed as a command or author. Exact task/file matching permits independent QA, while other blocked work is not marked complete. `cargo test --locked --lib`: 59 passed, 1 real-model test ignored; `npm.cmd test`: 19 passed; `npm.cmd run build`: passed. The real desktop path, a historical blocked record, QA completion, and app restart have **not** been observed yet. Next: restart app for backend protocol 18 and run QA-35 in a disposable project. Do not use the user's `D:\projects\새 폴더 (2)` as a test fixture.


## One-send AI-routed conversation (2026-10-08)

- [x] Identify the actual read-only `ask_goal` path, separate guidance command, safe checkpoint, and approval boundary.
- [x] Update goal conversation Spec and record ADR-0029 before implementation.
- [x] Persist AI routing result and automatically turn unambiguous CEO instructions into guidance.
- [x] Continue a routed instruction through safe checkpoint, linked revision, and CTO/PM re-planning; recover after restart.
- [x] Replace the two-action composer with one send and show the actual route/status.
- [x] Run tests/build, add interactive QA, and record remaining desktop verification.
- [ ] Restart the Windows app and run QA-34 with a connected AI, including a real instruction, approval boundary, and restart; record evidence. Ubuntu execution remains a separate platform gate.

Checkpoint: `send_goal_message` records one CEO turn and asks the selected AI, in read-only mode, to classify discussion, direction, guarded continuation, or ambiguity. Only a validated direction routes the exact CEO text through the existing transactional guidance store; a guarded start/retry uses existing engine commands. Auto guidance persists its continuation flag, waits for the engine checkpoint, creates a linked intent with bounded prior chat marked as non-authoritative context, and starts CTO/PM planning. Startup resumes pending auto guidance. The conversation has one send button and distinguishes real AI replies from deterministic engine acknowledgments. File writes still require scoped CEO approval. Rust tests passed 55/55 with one opt-in real-model test ignored; frontend tests passed 19/19; TypeScript/Vite build passed on Windows. A fresh desktop call with connected Claude/Codex and real goal state has not been run, so model classification quality, end-to-end auto continuation, and narrow-window UI remain unverified. Next: restart the app (backend protocol 17), run QA-34 in an isolated goal through question → instruction → new plan → approval, and record observed results; repeat interruption/restart branch.


## Goal instructions inside the conversation (2026-10-08)

- [x] Confirm that the current mode switch, generic read-only helper, and separate guidance card make the chat appear question-only.
- [x] Specify one draft with two explicit send actions, instruction review, and one chronological message stream; record ADR-0028.
- [x] Implement the shared composer, confirmation, and persisted guidance messages in the timeline.
- [x] Verify backend command separation, frontend build/tests, and add manual QA; record desktop QA still needed.

Checkpoint: `src/GoalConversationChat.tsx` uses one draft with explicit `질문 보내기` and `목표 변경 지시` actions. The latter previews the exact text, addressee, and checkpoint effect before calling `propose_goal_change`; saved guidance appears with question turns in chronological history and after restart. `src/App.tsx` still routes questions to `ask_goal` and instructions to `propose_goal_change`. Security, conversation, content-density, product, validation, and QA documents now reflect this distinction. `npm.cmd test` passed 19/19 and `npm.cmd run build` passed on Windows. Desktop interaction with a live goal remains unverified. Next: restart the app and run QA-34, especially instruction submission while a question is pending, the confirmation state-change guard, persistence after restart, and narrow-window scrolling. Record results before calling desktop UX complete.


## Team-routed goal changes and verified continuation (2026-10-08)

- [x] Inspect current guidance checkpoint, linked revision, task integration, approvals, and QA path.
- [x] Update goal conversation/domain/security specs and ADR-0027 with CTO→PM recommendation and engine-proof boundary.
- [x] Persist addressed team member, real read-only worker escalation memo, task integration proofs, and revision continuation decisions.
- [x] Let CTO/PM create a delta plan; validate completed-task reuse and allow a no-write QA path.
- [x] Show routing and engine-verified reuse in the conversation; preserve new-write approval and recheck evidence before QA.
- [x] Run focused Rust/frontend tests and update validation, QA-33, implementation map, and checkpoint notes.
- [ ] Run QA-33 with real connected team members and a disposable project on Windows; repeat the core flow on Ubuntu.

Checkpoint: A CEO direction change now records its addressed agent. A linked revision invokes that agent for a read-only escalation memo, hands it to CTO, then PM proposes new tasks plus prior task IDs for reuse. Integration records SHA-256 per declared file. The engine accepts reuse only for completed tasks with matching current bytes and verified dependency proofs, including inherited reuse across revisions. New file tasks request a fresh approval; a no-write revision proceeds to QA. The engine rechecks reuse before QA and includes reused work in the final report. Rust tests: 54 passed, 1 ignored; frontend tests: 19 passed; TypeScript/Vite build passed on Windows. Existing tasks without proof cannot be auto-skipped; Git initialization/first commit remains a separate unsupported operation. Actual multi-model desktop QA and Ubuntu execution remain open. Next: restart app, run QA-33 in an isolated project, check worker → CTO → PM evidence and approval/no-write branches, then record results.


## Conversational teammate replies (2026-10-08)

- [x] Trace the saved-turn runtime prompt and current chat presentation; identify the mandatory report scaffold.
- [x] Specify concise question-first replies with detail on demand and unchanged read-only/approval boundaries.
- [x] Adjust the goal discussion prompt and task-focused default question without changing goal/task transitions or rewriting saved answers.
- [x] Verify Rust tests (53 passed, 1 ignored) and frontend build; add QA-32 for ordinary questions and explicit draft requests.
- [ ] Run QA-32 in the Windows desktop app with a real Claude worker; compare a short question and a requested detailed draft.

Checkpoint: The mandatory restatement/facts/decisions/next-step scaffold caused formal, repetitive replies. The new runtime prompt asks for a direct answer in the CEO's language, brief ordinary replies, and detailed structure only on request; it still prohibits writes, approval, and fabricated state changes. Existing stored turns are untouched. The provider may vary in how well it follows style, so live QA is required. Next: restart the desktop app and ask the same Claude worker one short status question, then an explicit draft request.


## Conversation-first goal workspace (2026-10-08)

- [x] Review the existing goal workspace, recovery flow, task focus, density policy, and goal conversation/security boundaries.
- [x] Specify one conversation surface, explicit message modes, one current decision card, and one progress disclosure in Goal conversation Spec and ADR-0026.
- [x] Rework the workspace layout and composer while preserving saved turns, approvals, recovery, guidance, and task focus.
- [x] Verify frontend tests/build and add manual QA for a first-time user, blocked goal, long history, and narrow window.
- [ ] Run QA-31 in the Windows desktop app with a connected worker, a blocked goal, and approval; repeat the core flow on Ubuntu.

Checkpoint: `src/GoalConversationChat.tsx` replaces the stacked workspace with a saved-turn conversation, one current decision card, one progress disclosure, and a bottom composer with separate question/change drafts. Existing engine commands and approval boundaries remain. `npm test` passed 19/19 and `npm run build` passed on Windows. The desktop interaction itself is not yet manually verified. Next: restart the app, run QA-31, and record any screen or keyboard issues before calling desktop UX complete.


## Open-work floating launcher and worker discussion (2026-10-08)

- [x] Inspect the goal conversation, actual read-only worker call, project switching, unfinished state, and approval boundary.
- [x] Specify the cross-project launcher and task-focused discussion in Goal conversation Spec and ADR-0025 before code.
- [x] Implement persisted cross-project work selection, a fixed launcher, task-focused worker conversation using the existing real `ask_goal` call, and safe resolution navigation.
- [x] Verify the open-work selector (2 focused cases), all frontend tests (19 passed), TypeScript/Vite build, and add QA-30 plus validation/implementation map.
- [ ] Run QA-30 in the Windows desktop app with two projects and a real connected worker; repeat the core flow on Ubuntu.

Checkpoint: The launcher uses saved goal/task state and routes to the existing conversation. A task selection preselects its assigned AI worker and prepares an editable task question; the worker reply remains read-only. CEO approval/retry/guidance remains the only route to a state change. This is not a native task-session chat or arbitrary worker-issued file edit. Next: restart the desktop app and perform QA-30 with isolated two-project data.


## Blocked-goal recovery UX (2026-10-08)

- [x] Trace blocked-goal entry points and engine-safe retry, approval, and revision transitions.
- [x] Define the one-workspace recovery flow in Goal conversation Spec and ADR-0024 before code.
- [x] Route CEO, Office rail/member activity, project, and board blocked entries into the workspace and show state-specific actions. Approval shows its saved scope; pre-write retry and approved-plan resume call existing guarded commands; later failures lead to CEO-authored revision, and already revised goals link to their replacement.
- [x] Verify recovery selection (3 new cases), all frontend tests (17 passed), TypeScript, and Vite build. Update QA-17/26 and add QA-29 plus validation and implementation map.
- [ ] Run QA-29 in the actual Windows desktop app, including a real blocked provider call and approval interruption; then repeat the core flow on Ubuntu.

Checkpoint: The reason-only blocked dialog is removed from goal entry points. The workspace shows the saved report and a state-safe next action. The engine still cannot natively resume a developer session or integrate an interrupted worktree after execution/integration/QA failure; those cases use a linked, freshly approved goal version. Next: restart the desktop app and perform QA-29 against isolated project data.


## Git-conversion task integration diagnosis (2026-10-08)

- [x] Trace the reported `Task produced no file changes` to ordinary-folder integration: only declared working-file byte deltas are integrated, `.git` is excluded, and agent tasks are instructed not to commit. The reported task title asks for Git initialization and a first commit, which this file-task path cannot perform. The exact run was not found in `.run-data/organization.sqlite`, so the user's task result itself was not inspected.
- [x] Identify a second issue: selecting an already registered path reused its saved `folder` mode even after external Git conversion. Define explicit re-detection in Product/Security and ADR-0023.
- [x] Re-detect project mode on selection/switch while preserving ID and QA preset. Add an actionable no-file-change explanation for Git setup tasks.
- [x] Focused tests passed for mode refresh and diagnostic. 
- [x] Run full Rust suite (53 passed, 1 real-model opt-in ignored) and frontend suite (14 passed).
- [ ] Run manual QA for an actual externally converted project. The app still has no dedicated Git-init/commit operation; repository setup requires explicit handling outside the current AI file-task loop.

Checkpoint: The original task cannot integrate Git metadata from an isolated folder; a future dedicated, reviewed Git setup operation would be needed for in-app conversion. Re-selection now notices a committed repository.

## Explorer-style project folder selection (2026-10-08)

- [x] Inspect project registry, current Windows picker, nested-folder copy behavior, SDD index, product/security specs, and relevant ADRs. Registration already accepted nonempty parents; the Windows picker used legacy `FolderBrowserDialog`.
- [x] Specify native Explorer-style folder selection and nested repository metadata handling in Product/Security and ADR-0022 before code.
- [x] Replace Windows picker with `IFileOpenDialog` folder mode on an STA thread. Keep macOS/Linux picker and manual path fallback. Permit ordinary parent-folder copies to include nested working files while omitting `.git` metadata.
- [x] Verify Windows Cargo check, TypeScript check, and focused parent-folder registration/copy test. The focused test passed; `.git` metadata stayed out of the copy.
- [x] Run full Rust suite (52 passed, 1 opt-in real-model test ignored), frontend suite (14 passed), TypeScript check, Cargo check, and Vite build. QA-02/05C include the new interaction.
- [ ] Click through the native dialog in the Windows desktop app and run Ubuntu regression for the unchanged Linux picker.

Checkpoint: The requested nonempty parent was already registerable; its Windows navigation UI is now the Explorer-style Common Item Dialog. A real desktop click-through is still needed to verify appearance and cancellation. Selected-folder execution retains existing size and symlink limits.

## Single-origin project SDD synchronization (2026-10-08)

- [x] Recheck the index, import Spec, ADR-0020, preview/apply storage path, and UI after the user identified two mutable SDD origins.
- [x] Define project-folder origin, built-in presets plus DB runtime/read model, and one-way file → reviewed draft → DB apply in the Spec and ADR-0021. Record that existing Markdown SDD versus JSON manifest ownership still needs a reference-based schema.
- [x] Require a file-origin draft and fresh file-content comparison at DB apply; let a generic draft initialize an absent project file after confirmation, then preview the file separately. Backend protocol 16 requires restart.
- [x] Run focused Rust import tests (6 passed), full Rust suite (52 passed, 1 real-model opt-in ignored), TypeScript, frontend suite (14 passed), and Vite build. Unit tests confirm generic drafts cannot apply and a file edit invalidates its previous preview.
- [ ] Run QA-27/28 in the Windows desktop app and Ubuntu; no GUI click-through was performed in this turn.

Checkpoint: New project SDD definitions originate in `.ucompany/spec.json`; SQLite keeps an applied read model and operational state. Existing SDD Markdown files are not automatically kept in sync with this JSON yet, so avoid duplicating the same requirement in both until a document-reference schema exists.

## Portable cross-tool specification layer (2026-10-08)

- [x] Inspect SDD/ADR, importer, storage, project path rules, and UI. Identify the SQLite profile as the current applied copy and the missing project-file sync path.
- [x] Define `.ucompany/spec.json` as the tool-neutral project/team/workflow source, DB as applied projection plus runtime state, explicit drift/preview, and create-only initialization in the owning Spec and ADR-0020.
- [x] Implement fixed-path status, project-file preview through the existing staged importer, create-only portable export, project-bound path checks, and a confirmation UI. Backend protocol 15 requires restart.
- [x] Verify focused portable tests (6 passed), full Rust suite (52 passed, 1 real-model opt-in ignored), frontend suite (14 passed), TypeScript, Cargo check, and Vite build. Add QA-28 for cross-tool edit/apply/restart.
- [ ] Run QA-28 in the Windows desktop app and Ubuntu. No real external-tool edit or GUI click-through was performed in this turn.
- [ ] Design the separate provider handoff that shows exact selected text and destination before any runtime prompt inclusion. Current portable file is reference data, not executable engine policy.

Checkpoint: Cross-tool definitions can now live in a project file and be revised by other tools; Ucompany detects changes and applies them only after review. Existing AGENTS.md/CLAUDE.md are not rewritten, and operational state stays in SQLite.

## External SDD import and exception specifications (2026-10-08)

- [x] Inspect the SDD index, domain/security/architecture rules, global team storage, project binding, prompt construction, and current UI.
- [x] Define project-scoped staged import in `sdd/spec-import.md` and ADR-0019 before code.
- [x] Implement bounded JSON/Markdown parsing, preserved unknown fields/sections, `AGENTS.md`/`CLAUDE.md` harness exceptions, explicit team placeholders, and rejection of several obvious secret formats before staging.
- [x] Add atomic versioned project profiles, one draft per project, stale-draft rejection, selected-project snapshot, Projects preview/apply/export UI, and Office team-spec view. Backend protocol 14 requires restart.
- [x] Verify parser/security/storage/frontend behavior: full Rust suite 51 passed, 1 real-model opt-in ignored; frontend 14 passed; final focused import tests 5 passed; TypeScript/Vite build passed. QA-27 and implementation map updated. Source search confirmed no imported text enters `engine.rs`.
- [ ] Run QA-27 in the actual Windows app and Ubuntu. Runtime prompt transmission remains unimplemented: automatic approval review rejected sending imported files to AI providers because exact content and provider destination were not authorized. The app keeps all imported text local; a separate payload-and-destination approval design is required.

Checkpoint: External SDD is locally importable as a project-scoped versioned profile, not yet executable workflow policy or provider prompt context. Existing harness files are preserved and can be represented as machine-readable exception specifications, with local JSON export. Next: restart desktop app, perform QA-27, then design an explicit per-provider handoff of reviewed content before changing runtime prompts.


## Provider login and recovery guidance (2026-10-05)

- [x] Inspect saved-path discovery, connection UI, runtime diagnostics, integration/security specs, and official provider login instructions.
- [x] Define interactive login and fixed provider recovery behavior in Integrations/Security and ADR-0018 before code.
- [x] Implement saved-path login launcher and provider-aware error guidance in AI Connections and blocked goals. Login state changes only after an explicit saved-path recheck. Backend protocol 13 requires an app restart.
- [x] Run `cargo check --locked --bin ai-organization-os`, full Rust suite (46 passed, 1 opt-in ignored), full frontend suite (14 passed), and TypeScript/Vite build. Add QA-26 for the remaining interactive gate.
- [ ] Perform QA-26 with actual CLI login windows and authentication transitions on Windows and Ubuntu. No login was started or changed during this automated turn.

Checkpoint: The app now opens fixed login commands from a registered built-in CLI path only after a click. Claude still needs the user's own browser/terminal authentication, and a newly opened window is not evidence of readiness. Next: restart the desktop app, press `로그인 창 열기` for the registered Claude connection, complete the provider flow, and press `이 경로로 다시 검사`.


## Claude auth recheck and Office team settings (2026-10-05)

- [x] Inspect SDD, discovery, saved-connection refresh, team persistence, and Office UI. Reproduce this Windows user's Claude `auth status` safely: both npm `claude.cmd` and its package-native `claude.exe` returned exit code 1 with `loggedIn=false` under the current Windows user. The app must not claim this is logged in.
- [x] Update integration/product/validation Specs and ADR-0017 before code. Define exact saved-path recheck, false-vs-unverified diagnosis, atomic selected-team AI assignment, and separate team-wide Office settings.
- [x] Probe each registered built-in CLI at its saved executable path even if PATH discovery uses another path. Add a registered-connection recheck button with checked path, safe result, and next action. Different discovered paths require explicit comparison/confirmation before replacement. Claude false JSON stays `needs_setup`; missing/invalid auth response stays `needs_auth_check`.
- [x] Add one-transaction bulk AI assignment for selected active teammates; clear obsolete model overrides, preserve other member fields/history, and reject HTTP text-only assignment to implementation roles. Move add/hide/restore into Office team settings; keep individual fields in the member panel.
- [x] Run Rust full suite (45 passed, 1 opt-in ignored), focused atomic assignment and auth tests, frontend 12 tests, TypeScript, and Vite build. Final path-change UI build and narrowed saved-CLI guard checks passed.
- [ ] Restart the desktop backend and perform QA-25 in the actual UI, including CLI login transition on the same Windows user and saved-path mismatch. No CLI login was changed by this work.

Checkpoint: Backend protocol 12 flags an older running app. Claude's current local CLI result was `loggedIn=false`, so its `로그인 필요` status is evidence-based in this environment. The prior app could also leave a stale status when its saved executable differed from PATH discovery; that refresh bug is corrected. QA-25 documents the exact interactive checks. Next: close/restart the current app after preserving unsaved input, recheck the registered Claude path, then log in through that exact executable if it still returns false.

## Git-independent project folders (2026-10-05)

- [x] Read SDD index/workflow, project/security/architecture/integration specs, ADR-0003/0006, and affected registration/execution code. The prior flow required a Git root at registration and a worktree at execution.
- [x] Update owning Specs and add ADR-0016 before implementation. The selected existing folder is registered unchanged; Git roots retain worktrees, other folders use isolated copies with baselines.
- [x] Persist project `kind` with a Git default for legacy records. Allow empty, populated, and nested folders; keep canonical path validation and per-goal project binding. Remove Git from Windows launcher prerequisites and permit Codex CLI outside a repository.
- [x] Add bounded plain-folder snapshots, declared-file and conflict checks, approval wording, UI folder labels, and manual QA-24. Keep each task's copy and baseline for interruption diagnosis.
- [x] Verify Rust 44 passed, 1 opt-in test ignored; frontend 12 passed; TypeScript, Vite, Cargo check, and launcher prerequisite check passed. Repair the HTTP mock test to drain request bodies before closing on Windows. `cargo fmt --check` is unavailable because rustfmt is not installed in this toolchain.
- [ ] Restart the already-running desktop app and perform QA-24 through the UI, including a real approved non-Git goal and conflict path. Ubuntu execution remains unverified.

Checkpoint: Backend protocol 11 requires a desktop app restart for this change. A pre-existing app process was running during verification and was not terminated. Automated folder copy/conflict tests pass; no live AI non-Git project workflow has been claimed as verified. The copy limit is 20,000 files or 512 MiB; symlinks block snapshotting. Next: restart after preserving any unsaved UI work, then complete QA-24 and the older QA-23/provider and Ubuntu gates below.

## Windows launcher binary selection (2026-10-05)

- [x] Reproduce the cause from the manifest and launcher: adding `provider-harness` created a second Cargo binary, while the launcher still used an unqualified `cargo run`.
- [x] Update the validation Spec, then make the launcher select `--bin ai-organization-os` explicitly. No ADR is needed for this launcher command correction.
- [x] Verify PowerShell parsing, `-CheckOnly`, and `cargo check --locked --bin ai-organization-os`.
- [x] Smoke launch with isolated `.run-data/launcher-smoke-20261005`: Vite became ready and Cargo ran `target\debug\ai-organization-os.exe` with that data directory. The test app was stopped by process termination, so that smoke session returned exit code -1 while the launcher cleaned up its Vite process.

Checkpoint: The reported multi-binary startup error is fixed. Next: user can run `scripts/start-windows.ps1` normally and close the app window when finished. The earlier QA-23 real Claude goal and other live acceptance gates remain open below.

## Provider/runtime boundary and Claude report (2026-10-04)

- [x] Trace the actual failed goal through UI command, engine, saved invocation, CLI launch, stdin/stdout/stderr, and exit handling. The saved failed invocations used CTO → Codex CLI, not Claude; Claude standalone read-only invocation and synthetic Codex invocations succeeded. Record the evidence and uncertainty before implementation.
- [x] Update the integration Spec, investigation note, and ADR-0015 for provider-vs-runtime responsibilities, independent harness, and diagnostic error shape.
- [x] Extract provider-specific CLI and HTTP request/response logic into `provider.rs` behind `AiProvider`; keep process I/O, explicit stdin EOF, timeout, and HTTP transport in `runtime.rs`. Engine now calls the provider entry point; saved connection and frontend protocol formats remain unchanged. The external adapter path still passes its existing test.
- [x] Add standalone `provider-harness` with an offline Node fixture by default, explicit real Claude/Codex opt-in, isolated temp Git directory, and bounded result output. The offline check verifies cwd, inherited PATH, stdin EOF, JSONL, and exit code.
- [x] Test process exit, structured stdout error, sensitive diagnostic redaction, timeout, HTTP normalization, Claude/Codex standalone paths, and engine regression. Add the current analysis AI to new/retry goal cards so CTO/Codex cannot be mistaken for a developer's Claude assignment.
- [ ] Restart the running desktop app and complete QA-23 with a fresh isolated goal actually assigned to Claude. The historical exit-code-1 condition is not yet reproduced; the old binary cannot prove the new app behavior. Do not replay the saved CEO prompt after automatic approval review rejected that transmission.

Checkpoint: Default app data contains two failed `cto`/`codex` analysis invocations for goal `566e1137-37a7-4bef-b6b1-1c1d3de67df1`; Claude was registered and logged in but not assigned to the CTO. Installed Claude Code 2.1.289 and Codex CLI 0.159.3 each passed synthetic read-only standalone calls through the new Rust harness after explicit stdin EOF. The offline harness passed through `cargo run` and `npm.cmd run provider:test -- offline`; the PowerShell `npm.ps1` shim still points to a missing npm package. Rust: 41 full-suite tests passed plus a focused missing-CLI test (42 current regular tests), 1 opt-in real-model test ignored; frontend: 12 tests passed; TypeScript, Vite, and `cargo check --locked` passed. `cargo fmt --check` could not run because rustfmt is not installed. The currently running desktop process predates this backend build; no fresh Co-work goal has been executed through the changed process transport. A request to close/restart the running app is pending user response because unsaved UI input could be lost. See `sdd/provider-runtime-investigation.md` for observed facts, uncertainty, and the automatic approval rejection. Next: restart the app when approved, verify QA-23 with a new isolated goal and explicit CTO→Claude assignment, then continue earlier QA-22 and Ubuntu gates.

Updated: 2026-10-05. This file is the restart checkpoint. Read `sdd/00_index.md` and the relevant source before continuing.

## Resume milestone: unified goal workspace (2026-10-03)

- [x] Reopen the SDD index/workflow, goal conversation Spec/ADR, affected code, and old checklist. Confirm protocol 10 changes are present.
- [x] Extend the existing goal conversation into one compact goal workspace with confirmed stage, owner, next action, last operational event, assigned tasks, approval scope/direct confirmation, QA evidence, and token provenance. CEO, Office, and board cards open it. Reuse `GoalGraph` and the information-density controls.
- [x] Make proposal/approval invalidation, proposal/completion exclusion, and replacement creation atomic in SQLite. Add race tests for completion vs proposal and duplicate replacement, plus goal brief tests.
- [x] Let the CEO choose an enabled teammate with a usable connection for read-only goal discussion. Snapshot the selected teammate, connection, and model at submission; label each saved answer with its original speaker. The UI explains when no teammate is available. Reset the draft on goal switch and distinguish interrupted replies.
- [x] Make the one-pending-question rule atomic across app windows. A simultaneous-submit storage test proves exactly one saved turn and event; a role selection test checks disabled/unavailable teammates and model override snapshots.
- [x] Run full local checks and attempt a fresh desktop QA-22 run. The isolated app launched with protocol 10 and its Vite server, but Windows Computer Use returned no targetable Tauri window; the test app/server were stopped. Real provider dialogue and click-through QA remain unverified.
- [ ] Complete QA-22 with a targetable desktop window and real CTO connection; then continue runtime cancellation/resume, richer goal history, Windows full flow, and Ubuntu verification.

Checkpoint: Shared goal workspace, atomic steering persistence, and role-selected discussion are implemented. Rust tests: 38 passed, 1 opt-in real-model test ignored; frontend tests: 12 passed; TypeScript and Vite build passed. The isolated launcher printed `Running target\\debug\\ai-organization-os.exe` and test data path `.run-data/qa22-workspace-20261003`; sky `list_apps`/`list_windows` did not expose that process even though Windows reported a main window handle. No UI screenshots or actual AI responses were observed. Next: perform QA-22, including unavailable-role handling and simultaneous submission, in a targetable desktop session without using the user's normal app data; then finish the larger runtime/Ubuntu backlog in `sdd/workflow-improvement-plan.md`.

## Active goal conversation and workflow redesign (2026-10-02)

- [x] Audit the current goal engine, CEO/Office/board surfaces, trust boundary, and earlier unfinished plans. The engine has no in-flight steering channel or goal-scoped AI dialogue; the Office current-goal rail is still planned, and live QA/recovery/Ubuntu work remains open.
- [x] Specify the product distinction in `sdd/goal-conversation.md`, the staged delivery in `sdd/workflow-improvement-plan.md`, and checkpoint rationale in ADR-0014. Product, domain, security, validation, and implementation maps reference them.
- [x] Implement persisted goal turns, actual read-only CTO answer, provider/model identity, and restart interruption state. CEO cards and the Office rail open the same goal conversation.
- [x] Add CEO guidance queue and safe engine checkpoints. Pending old approvals are superseded; a human-reviewed linked new goal starts with fresh analysis, plan, and approval. Storage restart tests cover turn interruption and guidance/approval recovery. QA-22 records live concurrency checks still needed.
- [x] Implement the compact Office active-goal rail from the earlier plan using the shared phase graph and assignee data. Mark its expanded workspace and live keyboard/narrow-layout checks as outstanding in its own plan.
- [ ] Run a fresh desktop AI interaction and the QA-22 concurrency scenarios. Continue runtime cancellation/live session resume, stronger adapter/runtime approvals, Windows full flow, and Ubuntu verification from earlier plans. The unified workspace implementation is recorded in the 2026-10-03 milestone above.

Checkpoint: Backend protocol 10. Rust unit tests: 33 passed, 1 opt-in real-model test ignored; frontend tests: 10 passed; final Rust check, TypeScript, and Vite build passed. The global `npx` shim is broken, so project-local tools were used. Next: launch a fresh desktop app for QA-22 with a real CTO connection, especially guidance arriving during a call, approval, and developer wave. No real provider dialogue or live pivot timing has yet been observed. The unified goal workspace and stronger concurrency/recovery checks remain in `sdd/workflow-improvement-plan.md`; historical WORKLOG unchecked entries retain their QA/Ubuntu gates.

## Active information density policy (2026-10-02)

- [x] Audit repeated records and tall content across CEO, board, Office, project/AI lists, and token usage. Define the five-item default, 160px preview, disclosure behavior, and explicit exceptions in `sdd/content-density.md` before code.
- [x] Reuse shared frontend disclosure/preview components across CEO, board, Office, project/AI, and token views. Pending approvals remain fully visible; Office Canvas still shows all active characters. No backend or stored-data changes.
- [x] Verify newest-first member logs with a selector test; run TypeScript, frontend tests, and Vite build. `qa/MANUAL-QA.md` QA-21 records live interaction checks.

Checkpoint: `sdd/content-density.md` owns the policy; `src/ContentDensity.tsx` provides the shared five-item disclosure and 160px text preview. Local TypeScript and Vite build passed; 9 frontend tests passed. The global `npx` shim fails because its npm package path is missing, so verification used project-local `node_modules/.bin` commands. Desktop interaction and responsive layout still need QA-21 in a fresh app run.

## Active team lifecycle and goal visibility (2026-10-02)

- [x] Inspect agent persistence/Office filtering, CEO cards, shared graph, and board. Update Product, Domain, progress policy; ADR-0013 records reversible hiding and display-only grouping.
- [x] Write Office current-goal visibility implementation plan with card fields, layout, data provenance, and acceptance cases.
- [x] Implement safe hide/restore for developers, with unfinished-task block and preserved history. Existing `enabled` state and Office Canvas filtering are reused; Rust storage test covers pending/completed assignment count.
- [x] Implement deterministic CEO goal collapse/expand without a model call. Full intent/report remains available by explicit expansion; frontend test covers local shortening.
- [x] Show a graph for every board goal and explicit visual grouping of related goals without changing execution scope. Existing `GoalGraph`/`phaseState` are reused; group label persistence and grouping selector are tested.
- [x] Run focused Rust/frontend tests, TypeScript/build, update manual QA and this checkpoint. Rust: 31 passed, 1 opt-in ignored; frontend: 8 passed; TypeScript and Vite passed. TypeScript and Vite passed again after the final developer-role match edit.

Checkpoint: Backend protocol is 8. QA-20 records live checks; the desktop UI has not been exercised in this turn. The planned Office current-goal rail remains future work, as requested as a plan. Next: restart the desktop app and perform QA-20.

## Active goal-owned QA caps and token analytics (2026-10-02)

- [x] Inspect QA decision path, project/goal persistence, invocation ledger, and current UI. Correct owning specs; ADR-0012 records goal cap migration and invocation-time model identity.
- [x] Persist per-goal QA cap copied from the project preset and apply it at QA; add independent goal setter and one-time legacy migration. Storage migration test passed.
- [x] Add token menu with project/goal/team/model breakdown, project-denominator ratio bars, sorting, expandable cards, and grounded reduction insights. Invocation records snapshot their effective connection/model.
- [x] Run Rust, TypeScript, Vite, and focused aggregation checks; update manual QA and record actual evidence. Final rerun: Rust 30 passed, 1 opt-in ignored; TypeScript and Vite passed; frontend 6 tests passed.

Checkpoint: Backend protocol is 7. QA-19 covers live interaction and model identity. Next: restart the app and perform QA-18/19. Live desktop interaction and real provider usage remain unverified. Preserve previous Office rollback.

## Active CEO goal controls and fixed chrome (2026-10-02)

- [x] Inspect current QA limit save path, goal cards, app shell, and related SDD. Update Product and Validation specs. Existing confirmation/save path can be reused; no new ADR is needed for this UI placement.
- [x] Initial project-wide goal-card dropdown was replaced by goal-owned settings in the milestone above. Legacy goals without a project binding omit the control.
- [x] Fix sidebar and top app bar dimensions and place tab content in an independent scroll region; dialogs stay outside that region.
- [x] Run TypeScript, Vite, and focused UI checks; update manual QA and this checkpoint. TypeScript clean; Vite build passed; progress UI tests 3 passed. QA-18 records live checks.

Checkpoint: Fixed chrome and goal-card placement were implemented; the QA ownership model was superseded in the milestone above. Live Tauri scroll interaction remains unverified.

## Active manual QA follow-up (2026-10-02)

- [x] Diagnose clean-tree block: `run` rejects every nonempty `git status --porcelain` before planning, although worktree preparation already snapshots current root contents. A newly registered Git project can become dirty as soon as the user adds files.
- [x] Update Product, Domain, Security specs for editable live review cap with confirmation, contextual approval action, and safe dirty-tree execution.
- [x] Add CEO and project-card 0–5 dropdowns with confirmation before saving. Active goals use the saved latest limit at their next QA decision.
- [x] Provide `확인하기` on approval-waiting goal cards and a shared approval review modal. Blocked goals without pending approvals open a reason/next-step dialog.
- [x] Remove blanket clean-tree block and test preserving preexisting tracked and untracked files through worktree integration. Focused Rust test passed, including approved edits to an untracked file.
- [x] Run Rust, TS, UI checks; update manual QA and this checkpoint. Rust: 29 passed, 1 opt-in ignored; TypeScript clean; Vite build passed; progress UI: 3 tests passed. Live Tauri verification remains QA-17.

Checkpoint: The full Rust, TypeScript, Vite, and UI test suite passed. ADR-0011 records safe dirty-tree handling; QA-17 lists interactive checks. Backend protocol is now 6. Next: restart the Tauri app, retry the previously clean-tree-blocked goal if its card offers `다시 실행`, and run QA-17. Do not mark the live dialogs or real model execution as verified until observed in the restarted app.

## Active QA rework loop and token visibility (2026-10-01)

- [x] Diagnose QA rejection: `execute_planned` returns an error on any non-PASS QA result, so no PM/developer rework occurs. `prepare_worktree` already snapshots integrated root changes into a new worktree, which enables a safe same-file rework round.
- [x] Specify QA rework bounds, approval scope, persistence, token sources/estimates, and low-frequency refresh; ADR-0010 records the decision. Model and snapshot fields have been added; runtime/engine behavior is still in progress.
- [x] Persist per-project maximum QA rework rounds (0–5, default 2) and expose it on each project card. Backend and frontend are connected; live UI check remains in QA-16.
- [x] Loop QA → PM repair plan → developer worktrees → integration → QA within previously approved files. Saved count/review and scope rejection are covered; live FAIL→repair→PASS/limit check remains in QA-16.
- [x] Capture provider-reported token usage when available and fallback estimates per run; aggregate by project and character. The CLI observer persists active output estimates no more than every five seconds and final counts at completion. Live provider output check remains in QA-16.
- [x] Animate actual team handoffs in Office through recent persisted `agent_handoff` events. Source moves toward the recipient and returns without saving a new coordinate; reduced-motion CSS removes travel. Live visual check remains in QA-16.
- [x] Run Rust/TypeScript/Vite checks, update SDD and manual QA. Rust: 28 passed, 1 opt-in ignored; TypeScript clean; Vite build passed; progress UI: 3 tests passed. Live GUI and actual QA FAIL→repair acceptance remain in QA-16.

Checkpoint: Rust 28 tests passed (1 ignored); TypeScript, Vite, and 3 progress UI tests passed. `sdd/progress-visibility.md`, `sdd/integrations.md`, `sdd/validation.md`, and QA-16 match the implementation. Next: restart Tauri with protocol version 5, run QA-16 on an isolated project, record screenshots/logs for first FAIL→repair→PASS or limit, token updates, and motion behavior. Current Tauri process may still be an older binary. Do not claim live GUI verification without a new app run.

## Active Office progress visibility (2026-10-01)

- [x] Inspect Office canvas, member detail, board, domain events, and blocking CLI runtime. Confirm that existing task start/end events have assignee IDs but CLI stdout is collected only after process exit; CTO/PM/QA work has no member-linked live event.
- [x] Update Product, Domain, and Integrations specs for active avatar indicators, illustrative idle dialogue, genuine member activity logs, and a progress graph. Runtime output is operational evidence, not private model reasoning.
- [x] Add bounded, sanitized CLI progress event streaming (80 unique summaries per runtime call) and tie CTO/PM/QA/Developer stages to agent IDs in persisted events. CLI reasoning, prompts, command strings, and arbitrary output are not copied into UI events. A live app restart and SQLite persistence inspection remain manual QA.
- [x] Implement Office bubble/ellipsis, activity-first member details with a settings tab, board-to-member navigation, and goal-stage/task-count graphs from the shared progress selectors. Add reduced-motion styling and accessible active labels. Desktop/narrow live layout remains manual QA.
- [x] Add the canonical `sdd/progress-visibility.md` policy and ADR-0009 so future work types use the same provenance/state mapping. Progress selector tests (3) passed; Rust 25 regular tests passed with 1 opt-in ignored; TypeScript and Vite build passed. QA-05G records live acceptance. A SQLite restart test confirms scoped progress events persist.
- [ ] Restart the app and run QA-05G through CTO → PM → developer → QA, plus approval wait and a blocked goal. Confirm event persistence and layout in the actual Tauri UI.

## Active Windows runtime and single-HTML follow-up (2026-10-01)

- [x] Diagnose `Blocked: batch file arguments are invalid`: Windows Rust `Command` passes `.cmd` arguments through batch parsing; a natural-language CEO goal can contain unescapable shell characters. The installed Claude wrapper invokes package `claude.exe`; Codex's wrapper invokes `codex.js` through Node. Both package entry points were found. Rust's process documentation confirms the batch argument hazard.
- [x] Confirm Claude login independently: package-native `claude.exe auth status` returned `loggedIn: true`. Do not display identity fields. Probe this JSON in discovery and refresh matching saved connection status on AI recheck.
- [x] Route built-in Claude and Codex npm runtime calls to package-native or Node entry points, keeping goals as literal process arguments. Reject unsupported Windows batch wrappers with an actionable message. Regression test passes a Korean HTML request containing shell metacharacters through a mock npm Codex entry point.
- [x] Accept one implementation task for a requested single HTML file. Preserve parallel worktrees for genuinely independent files. Expose `다시 실행` only for goals blocked in analysis/planning before tasks or approvals exist.
- [x] Update integration/domain/product specs and ADR-0007/0008 before implementation. `cargo test --locked` passed 23 regular tests (1 opt-in ignored); TypeScript and Vite build passed. A live GUI goal run and generated HTML remain **unverified** because the app process started from the prior binary and requires a restart, and project-write approval must be handled in the app.
- [ ] Restart the current Tauri app, recheck Claude status, retry the blocked goal if eligible, review the PM's single-file plan, approve project write in the UI, and verify the app-generated HTML and QA report. Use QA-03B; do not create the HTML manually.

## Active Claude connection follow-up (2026-10-01)

- [x] Confirm Windows has an extensionless npm `claude` shim and sibling `claude.cmd`; the currently running desktop binary started at 20:07 before the `.cmd` preference fix. The saved `.run-data` database contains Codex but no Claude connection. Running the installed `claude.cmd --version` outside the restricted shell returned `2.1.286 (Claude Code)`.
- [x] Add a backend revision handshake. An older running Rust process shows a restart-required banner and guarded connection actions in the live frontend.
- [x] Normalize known Windows CLI wrappers during registration and repair previously saved extensionless paths on startup; preserve intentional custom connections.
- [x] Test wrapper normalization and persisted repair. `cargo test --locked` passed 19 regular tests (1 opt-in ignored); TypeScript and Vite build passed. QA-03A documents live UI checks. The currently running old backend has not been restarted, so actual post-restart Claude registration remains manual QA.
- [x] Install Codex CLI globally with `npm.cmd install -g @openai/codex@latest`. Outside the restricted shell, npm worked (11.16.0), so no repair attempts were needed. Global package `@openai/codex@0.159.3`, `codex.cmd --version` (`codex-cli 0.159.3`), and `codex.cmd login status` (`Logged in using ChatGPT`) passed. The restricted shell's plain `npm` failure was a sandbox path/module resolution artifact.

## Active follow-up QA checklist (2026-10-01)

- [x] Inspect current project import/creation, toast, path display, spacing, CLI discovery, model field, and Office characters. Windows `claude` resolves to an extensionless npm shell shim before `claude.cmd`, explaining Win32 error 193.
- [x] Update Product, Integrations, Security, and Validation specs; ADR-0006 records unified folder adoption and initialization.
- [x] Replace the two project cards with one folder choice. An existing Git root is adopted unchanged; an empty folder outside Git is initialized; nonempty non-Git and nested Git folders are rejected unchanged. Windows verbatim path prefixes are removed for display only. Focused Rust folder test and frontend path test passed.
- [x] Make success feedback a fixed, six-second action toast; goal submission offers a `목표 보기` action. Blocking reasons remain dialogs.
- [x] Review spacing across dashboard, projects, board, connections, and Office; add shared section rhythm, separate project and connection card groups, and responsive single-column layouts. Actual rendered GUI spacing remains for manual QA-05E.
- [x] Prefer runnable `.exe`/`.cmd`/`.bat` Windows CLI wrappers and explain probe failures. The regression test proves a `.cmd` is selected and executed ahead of an extensionless shell shim. Custom AI registration is in a modal dialog.
- [x] Restore distinct pixel character SVGs in Office. Connection registration fills only blank assignments in one SQLite transaction; existing assignments remain. HTTP model IDs come from `/models`, and Office uses a model choice or provider default instead of free text. Focused tests passed.
- [x] Run TypeScript, Vite, Vitest, and Rust checks; update manual QA and implementation map. Manual GUI acceptance remains open because an existing app and Vite session already occupy port 1420. Do not treat the running app as restarted with the new Rust backend.

## Active QA-driven improvement checklist (2026-10-01)

- [x] Inspect the current discovery, connection save path, disabled controls, project persistence, Office interaction, and platform folder picker options. On this Windows host only the Codex executable is on PATH and its version/login probe succeeds; Claude, Gemini, and OpenCode rows were static missing presets, not local discoveries. The repository has no dialog plugin dependency, so use existing OS folder interfaces without installing system software.
- [x] Update Product, Domain, Integrations, Security, and Validation specs; record ADR-0005 for multi-project goal binding and native folder selection.
- [x] Separate local AI discoveries from missing presets; show executable path, probe result, registration result, and actionable errors. On this PC only Codex CLI is installed. The discovery invariant test passed. **Manual UI registration and restart persistence remain to be checked with QA-03/05.**
- [x] Replace silent disabled buttons with clickable reason/next-step dialogs for missing project, short goal, incomplete AI registration, unavailable preset, missing role or AI assignment, duplicate selection, approval wait, and busy state. No `disabled` button attribute remains in `src/App.tsx`.
- [x] Move team configuration under Office; drag or keyboard-move characters and click them for settings. Seed CTO, PM, Developer, QA, and Specialist instruction presets while retaining nonblank user instructions.
- [x] Add a visible project list, switching, OS folder picker with path fallback, and safe new Git-project creation. New goals capture a project ID; an old goal without a binding cannot run. Storage restart and project creation tests passed.
- [x] Simplify labels, empty states, guidance, and feedback while retaining the existing colors and layout identity.
- [ ] Complete interactive Windows UI QA: actual connection registration/restart, folder dialog, Office drag, blocked-action dialogs, project switching, and approval flow. Automated checks passed, but the already running app occupies port 1420, so a new isolated launcher smoke run was not started or used to replace it.

- [x] Audit Windows toolchain and provider availability. Node 22, Rust 1.89, Git, WebView2, and Codex CLI are present. Codex is authenticated outside the restricted shell; Claude, OpenCode, Ollama, and LM Studio are unavailable. WSL Ubuntu 24.04.3 x86_64 is present but lacks its Linux build dependencies.
- [x] Write initial SDD specs, ADRs, and `AGENTS.md` / `CLAUDE.md` entry points before implementation.
- [x] Create Tauri 2, Rust, React, and SQLite skeleton. Evidence: `cargo check --locked` and frontend build passed; Windows app opened when launched outside the restricted sandbox with `UTOPIA_DATA_DIR` inside the workspace.
- [x] Persist organization config, goals, tasks, events, approvals; edit team and register CLI, HTTP, or external process connections.
- [x] Implement basic CLI/model server discovery, executable probes, Codex login probe, manual adapter entry, and per-agent model override.
- [x] Implement CTO analysis, PM JSON task planning, two or more parallel worktrees, dependency waves, serialized patch integration, QA result, CEO report, and approval gate.
- [x] Add dashboard, task board, approval controls, reports, and Canvas office. Reused Pixel Agents coordinate projection code under MIT notice; no third-party sprites included.
- [x] Verify the Windows backend runtime with two real Codex CLI instances in parallel worktrees, declared-file integration, Node tests, and independent read-only Codex QA. Evidence: the opt-in `real_codex_parallel_worktrees_integration_and_qa` test passed; its ignored fixture is under `.run-data/real-codex-e2e-245f29a5-fe3c-4929-941d-771eda3582cf`.
- [ ] Verify the complete Tauri CEO goal → approval → parallel execution → QA → report flow on Windows. CTO/PM and approval request were exercised in the app, but UI automation was stopped before approval. The backend fixture does not establish GUI acceptance.
- [ ] Implement normalized live runtime event streaming, cancellation, stronger adapter capability/health validation, and full task session resume.
- [ ] Enforce runtime operation approvals beyond the initial project-write gate; test malicious repository instructions and custom adapter behavior.
- [ ] Verify the same build, launch, and full goal on Ubuntu 24.04 x86_64. Existing WSL Ubuntu 24.04.3 x86_64 is available, but it lacks Linux Node, Rust, and Tauri GTK/WebKit development packages. The supplied master prompt says to notify the user first and never install system software automatically. `scripts/verify-ubuntu.sh` is ready for use after prerequisites are installed.
- [ ] Finish spec/code/ADR drift review and acceptance scenarios from `sdd/validation.md`.
- [x] Add a one-command Windows development launcher and an interactive manual QA runbook. The launcher checks existing tools, starts Vite hidden, runs Cargo in the foreground, supports an isolated data directory, logs startup, and cleans up its own Vite process. The runbook covers setup, persistence, approval denial, approval-pending restart, full goal execution, Office, and optional failure cases.
- [ ] Execute the new interactive QA checklist with the user and record actual PASS/FAIL evidence. Writing the checklist and smoke launching the app do not establish GUI flow acceptance.

## Verified commands

- 2026-10-01 follow-up revision: `node node_modules/typescript/bin/tsc --noEmit`, Vite production build, and `vitest run src/paths.test.ts` passed. `cargo test --locked` passed 18 regular tests; the opt-in real Codex test remains ignored. New Rust tests cover `.cmd` launcher selection/execution, HTTP model catalog parsing, empty folder initialization versus adoption, and transactional assignment only to blank members. The current Windows GUI was already running during this revision and was not restarted or interrupted; the updated Rust commands require a restart before manual UI verification.

- 2026-10-01 usability revision: direct `node node_modules/typescript/bin/tsc --noEmit` and `node node_modules/vite/bin/vite.js build` passed; `cargo test --locked` passed 14 regular tests with one opt-in real Codex test ignored. New tests cover discovery path truth, new project initialization, and project binding after switching/restart. `scripts/start-windows.ps1 -CheckOnly` passed prerequisites but reported port 1420 in use by an existing Node process; `ai-organization-os.exe` was also already running. Neither process was stopped or reused as an acceptance run.

- `npm run build` passed after the Canvas and board UI were added.
- `cargo check --locked` passed after the approval resume and DAG changes.
- `cargo test --locked` passed with 11 regular tests plus 1 opt-in real Codex fixture. New tests cover pending-approval restart recovery, completion only after declared patch integration, SQLite schema migration, traversal and Git metadata path rejection, and literal custom CLI argument handling for shell-shaped CEO text. `npm` currently resolves to a broken npm CLI path in this shell, so the equivalent direct `node .../tsc --noEmit` and `node .../vite.js build` checks passed.
- The opt-in real Codex fixture passed on Windows: two distinct native session IDs, separate worktrees, declared-file integration, three passing Node tests, and QA `PASS` with test evidence. The first run exposed Windows Git CRLF conversion in an over-strict fixture assertion; the assertion now accepts platform newline conventions. The fixture does not exercise Tauri UI approval or app restart.
- A first real Windows app attempt reached CTO analysis, PM delegation, and approval with 3 planned tasks. The PM added a read-only QA task as a developer task. Its project write approval was denied; no worktree edits were started. The planner now requires declared file changes and a separate QA phase. A second app attempt in `.e2e-project` reached CTO analysis, PM planning, and approval with two file-declared tasks. Computer Use was stopped with Escape before approval or parallel execution. After an app restart, a read-only SQLite inspection confirmed the goal is `blocked` with `resumed_from=awaiting_approval`; its approval and planned tasks remain pending. The app and Vite server are closed.
- Task status now changes from `running` to `awaiting_integration` when an agent returns, and to `completed` only after its declared patch applies. An undeclared-file integration failure leaves the task awaiting integration, with a focused regression test.
- SQLite now uses transactional schema version 2. Version 0 data upgrades without losing objects, the selected legacy project becomes a registered project, and legacy goals remain unbound until resubmitted; an unknown newer version is rejected before writes.
- `cargo run --locked` produced a running Windows GUI process when invoked with permission outside the restricted sandbox and `UTOPIA_DATA_DIR` set to `.run-data`. In-sandbox Tauri/WebView2 setup returned Access Denied before app setup.
- `cargo fmt --check` could not run because `rustfmt` is not installed in the current Rust toolchain. No system component was installed.
- Windows UI automation app launch timed out at the Computer Use approval step. No click-through flow was verified.
- On two later attempts Computer Use reported a physical Escape stop before any approval click. Respect that stop; do not infer the user approved the pending `.e2e-project` write. The second goal's approval remains pending in `.run-data/organization.sqlite`.
- Unsandboxed `wsl -d Ubuntu` confirmed Ubuntu 24.04.3 x86_64 and WSLg display variables; the Linux toolchain and Tauri development packages are missing.
- Running `scripts/verify-ubuntu.sh` in the existing Ubuntu 24.04 WSL environment stopped at `Missing node` before building. No packages were installed or system settings changed.
- `scripts/start-windows.ps1 -CheckOnly` passed, its PowerShell parser reported no errors, and a real launcher smoke run reached the Windows Tauri app with Vite ready. After stopping the run, port 1420 was free and no app process remained. `tsc --noEmit`, Vite build, and the 11 regular Rust tests passed after the approval-scope UI change.

## Next action

The currently running Windows app (PID 27372 when last checked) still uses the previous desktop binary. After the user closes it or it can be safely restarted, run `scripts/start-windows.ps1` with an isolated `-DataDir` and execute QA-23: verify the new goal card's CTO connection, explicitly assign Claude to the CTO, submit a new synthetic goal in an isolated Git project, and inspect invocation identity and success/error details. Do not replay the saved CEO goal after the automatic approval rejection. Then execute QA-22/05G/03B and continue session-resume, operation-approval, and Ubuntu gates. Record actual PASS/FAIL evidence; harness success alone is not desktop acceptance.
