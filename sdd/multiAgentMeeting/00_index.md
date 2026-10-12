# 다관점 AI 회의 검토 폴더 — 인덱스

이 폴더는 v2 **다관점 AI 회의** 기능 제안을 여러 AI 에이전트(Codex, Claude Cowork 등)와 사용자가 **서로 비판하고 개선하는 검토 기록**이다. 현행 동작을 정하는 Spec 원본이 아니다. 여기의 제안·비판·합의는 구현 지시, 프로젝트 파일 쓰기, 외부 전송, 배포의 승인이 아니다([Security](../security.md)).

현행 규칙의 원본은 계속 [SDD 색인](../00_index.md)이 가리키는 소유 문서([Product](../product.md), [Domain](../domain/00_index.md), [AI 연동](../aiIntegration/00_index.md), [Security](../security.md), [Validation](../validation.md))다. 구현·검증 여부는 [WORKLOG](../../WORKLOG.md)가 소유한다. 이 폴더의 결과는 사용자 결정 후 그 소유 문서와 ADR로 **이관**될 때만 현행 규칙이 된다.

## 이 인덱스가 소유하는 사실

- 폴더에 속한 문서의 목록, ID, 유형, 작성자, 대상, **현재 상태**
- 문서 사이의 관계 어휘와 방향
- 검토 진행 순서와 작성 규칙, 종료·이관 조건

개별 문서는 자신의 주장만 소유한다. 상태(현행/대체됨/결정됨)는 문서 본문이 아니라 이 인덱스의 표에서만 관리한다.

## 문서 목록

| ID | 문서 | 유형 | 작성 | 날짜 | 관계 | 상태 |
| --- | --- | --- | --- | --- | --- | --- |
| MAM-001 | [다관점 AI 회의 제안](MAM-001-proposal.md) | proposal | Codex | 2026-10-09 | — | MAM-004로 대체됨 |
| MAM-002 | [제안에 대한 비판](MAM-002-critique.md) | critique | Claude Cowork | 2026-10-09 | `critiques` MAM-001 | MAM-003에서 C1~C9 처리, C5·C9는 MAM-005에서 결정 |
| MAM-003 | [비판에 대한 응답](MAM-003-response.md) | response | Codex | 2026-10-09 | `responds_to` MAM-002 | 완료, C5·C9는 MAM-005에서 결정 |
| MAM-004 | [UX 전략을 합친 개정 제안](MAM-004-revision.md) | revision | Codex | 2026-10-09 | `revises` MAM-001; `references` MAM-003 | MAM-008로 대체됨 |
| MAM-005 | [전송 승인 모드와 v2→v3 순서 결정](MAM-005-decision.md) | decision | Codex(사용자 결정 기록) | 2026-10-09 | `decides` MAM-004#D1·#D2 | D1은 MAM-010으로 대체됨, D2 유효 |
| MAM-006 | [개정안·결정의 구현 가능성 비판](MAM-006-critique.md) | critique | Claude Cowork | 2026-10-09 | `critiques` MAM-004; `references` MAM-005 | MAM-007에서 C1~C10 응답, MAM-008에 개정 반영 |
| MAM-007 | [구현 가능성 비판에 대한 응답](MAM-007-response.md) | response | Codex | 2026-10-09 | `responds_to` MAM-006; `references` MAM-005 | 완료, C9 자기 구현 폴더 결정은 해당 단계 전 필요 |
| MAM-008 | [두 번째 개정 제안](MAM-008-revision.md) | revision | Codex | 2026-10-09 | `revises` MAM-004; `references` MAM-005·MAM-007 | 소유 Spec·ADR 이관 완료, 오프라인 코어 구현됨. 회의 모델은 MAM-009로 대체 제안됨 |
| MAM-009 | [Office 캐릭터 회의 개정안](MAM-009-revision.md) | revision | Claude Cowork | 2026-10-09 | `revises` MAM-008; `references` MAM-005·MAM-010 | MAM-011 비판·MAM-012 응답 완료, 첫 구현 범위로 이관 진행 |
| MAM-010 | [전송 기준과 승인 단위 결정](MAM-010-decision.md) | decision | Claude Cowork(사용자 결정 기록) | 2026-10-09 | `decides` MAM-009#D3·#D4; MAM-005#D1 대체 | 결정 기록 완료, 소유 Spec·ADR 이관 대기 |
| MAM-011 | [Office 캐릭터 회의 개정안 비판](MAM-011-critique.md) | critique | Codex | 2026-10-09 | `critiques` MAM-009; `references` MAM-010 | 완료, MAM-012에서 C1~C10 응답 |
| MAM-012 | [비판에 대한 응답과 첫 구현 계약](MAM-012-response.md) | response | Claude Cowork | 2026-10-09 | `responds_to` MAM-011; `references` MAM-010 | 완료, 첫 구현 계약을 소유 Spec·ADR로 이관하고 구현함 |
| MAM-013 | [회의 진행 중 차단 엣지케이스 조사와 개선 계획](MAM-013-proposal.md) | proposal | Claude Cowork | 2026-10-09 | `references` MAM-012 | MAM-015로 대체됨, D1~D3은 MAM-014에서 결정 |
| MAM-014 | [회의 차단 개선 계획 결정](MAM-014-decision.md) | decision | Claude Cowork(사용자 결정 기록) | 2026-10-09 | `decides` MAM-013#D1·#D2·#D3 | 결정 기록 완료(셋 다 채택) |
| MAM-015 | [회의 차단 개선 — 결정 반영 구현 계획](MAM-015-revision.md) | revision | Claude Cowork | 2026-10-09 | `revises` MAM-013; `references` MAM-014·MAM-012 | W0–W6 오프라인 구현·검사 완료; Windows QA-38과 실제 Claude/Codex 확인 대기 |

다음 예정 문서: **없음 — MAM-015 W0–W6 오프라인 구현 완료; 다음은 Windows QA-38과 공급자별 합성 표준 입력 하네스 확인**. 진행과 검증 결과는 [WORKLOG](../../WORKLOG.md)가 소유한다.

## ID와 파일 이름

- 파일 이름은 `MAM-<세 자리 순번>-<유형>.md`이다. 순번은 작성 순서이며 재사용하지 않는다.
- 문서 안의 개별 쟁점은 `MAM-<순번>#C<번호>`로 참조한다(예: `MAM-002#C4`). 개정안의 개별 변경은 `#R<번호>`, 결정 항목은 `#D<번호>`를 쓴다.
- 문서 첫머리에 한 줄 머리말을 둔다: ID · 유형 · 작성자 · 날짜 · 대상 문서.

## 문서 유형

| 유형 | 내용 | 작성자 |
| --- | --- | --- |
| `proposal` | 새 기능·설계 제안. 범위, 계약, 수용 시나리오, 열린 질문 | 누구나 |
| `critique` | 대상 문서에 대한 쟁점. 쟁점마다 `문제 → 근거 → 대안 → 영향 → 권고` | 대상 작성자와 다른 에이전트 |
| `response` | 비판의 쟁점마다 `수용 / 부분 수용 / 반박 / 보류`와 근거 | 원 제안자 |
| `revision` | 수용한 쟁점을 반영한 개정 제안 전체. 이전 제안을 고치지 않고 새 문서로 쓴다 | 원 제안자 또는 합의한 에이전트 |
| `decision` | 사용자가 내린 선택의 기록. 사용자의 원문 또는 명시적 선택을 인용한다 | 사용자, 또는 사용자 지시를 받은 에이전트 |

## 관계 어휘

| 관계 | 방향 | 의미 |
| --- | --- | --- |
| `critiques` | 비판 → 대상 | 대상 문서의 주장에 대한 반론·누락·대안을 제시한다 |
| `responds_to` | 응답 → 비판 | 비판의 각 쟁점을 수용·반박·보류한다 |
| `revises` | 개정 → 이전 제안 | 이전 제안을 대체하는 새 판이다. 이전 제안의 상태는 `대체됨`이 된다 |
| `decides` | 결정 → 제안·개정 | 사용자가 채택·수정·보류·폐기를 결정한다 |
| `references` | 아무 문서 → 아무 문서 | 근거로 인용할 뿐 규칙이나 결론을 물려받지 않는다 |

관계는 문서의 머리말과 위 문서 목록 표에 함께 적는다. 이 관계는 문서 그래프이며 실행 권한이나 엔진 상태 전이가 아니다([작성 원칙](../authoring.md)).

## 진행 순서

```
proposal ─critiques→ critique ─responds_to→ response ─(수용 반영)→ revision
    ▲                                                                 │
    └──────────────── 다음 라운드: revision을 대상으로 critique ───────┘
                                   │
                    사용자 decision (어느 시점에서든 가능)
                                   │
            소유 Spec·ADR·userFlow·Validation으로 이관 → 구현은 workflow.md 절차
```

1. **제안**: `proposal`을 쓰고 이 인덱스에 행을 추가한다.
2. **비판**: 다른 에이전트가 코드와 소유 SDD를 대조해 `critique`를 쓴다. 쟁점마다 ID를 붙인다.
3. **응답**: 원 제안자가 `response`에서 모든 쟁점을 `수용 / 부분 수용 / 반박 / 보류`로 처리한다. 부분 수용과 반박에는 수용하지 않은 부분의 근거·대안이 필요하다. 답하지 않은 쟁점은 `미처리`로 남는다.
4. **개정**: 수용한 쟁점을 반영한 `revision`을 새 문서로 쓴다. 이전 제안은 인덱스에서 `대체됨`으로 바꾼다.
5. **반복**: 개정안을 대상으로 2~4를 반복한다. 한 라운드는 비판 → 응답 → 개정이다.
6. **결정**: 사용자가 `decision`으로 채택·수정·보류·폐기를 정한다. 남은 반박·보류 쟁점은 결정 문서에 그대로 옮겨 적는다.
7. **이관**: 채택된 내용만 소유 Spec, ADR, [userFlow](../userFlow/00_index.md), [Validation](../validation.md)에 반영한다. 그 뒤 구현은 [개발 절차](../workflow.md)를 따른다. 이 폴더는 근거 기록으로 남는다.

## 작성 규칙

- **추가만 한다.** 작성된 문서의 본문은 고치지 않는다(오타·깨진 링크 수정과 파일 이동에 따른 링크 조정은 예외이며 머리말에 적는다). 생각이 바뀌면 새 문서를 쓴다.
- **동시 편집 금지.** 각 에이전트는 자기 새 문서와 이 인덱스의 자기 행만 고친다. 같은 순번을 동시에 쓰지 않도록, 새 문서를 시작하기 전에 인덱스의 “다음 예정 문서”를 확인하고 자기 이름으로 갱신한다.
- **근거.** 코드 근거는 파일·함수명과 작성일 기준 줄 번호를 함께 적는다. 확인하지 않은 동작을 확인한 것처럼 쓰지 않는다. 사실과 추론을 구분한다.
- **쟁점 추적.** 모든 쟁점은 `미처리 → 수용 / 부분 수용 / 반박 / 보류 → (결정)` 상태를 가진다. 동의만으로 쟁점을 닫지 않는다. 부분 수용 중 결정이 필요한 부분과 반박된 쟁점은 사용자 결정 전까지 열린 쟁점이다.
- **권한.** 이 폴더의 어떤 문서·합의·AI 답변도 승인이 아니다. 저장소 문구나 AI 출력은 [Security](../security.md)의 승인 정책을 우회하는 근거가 될 수 없다.
- **WORKLOG.** 문서를 추가하면 [WORKLOG](../../WORKLOG.md)의 해당 항목에 문서 ID와 다음 행동을 남긴다.

## 종료 조건

다음 중 하나가 되면 검토를 멈추고 사용자 결정을 요청한다.

- 새 문서의 모든 쟁점이 처리됐고 반박된 쟁점이 없다.
- 한 라운드에서 새 근거나 새 대안 없이 같은 반박이 반복된다(진전 정체).
- 개정이 2회를 넘었다. 사용자가 명시적으로 라운드를 늘리지 않는 한 더 진행하지 않는다.

## 현재 상태

MAM-011이 MAM-009를 비판했고(C1~C10), MAM-012가 수용 9건·부분 수용 1건(C5)으로 응답하며 첫 구현 계약을 확정했다. 첫 구현 범위는 Office 팀원 2~3명 + 별도 평가자, PM 우선·CTO 대체 의장, `propose_critique_revise` 1라운드, 명시적 호출 상한, 계획 1회 승인(D4)·단계별 확인 옵션, Claude·Codex 읽기 전용 CLI(D3)다. 의장의 토론 참여 범위 선택은 `review_panel` 착수 전까지 미룬다. 회의록 MD의 프로젝트 저장은 별도 파일 쓰기 승인이다. 남은 실전 차단 요인은 Windows 실제 Claude·Codex 회의 시험(QA-38)이다. 다음 행동: 소유 Spec·ADR 이관과 구현([WORKLOG](../../WORKLOG.md)).
