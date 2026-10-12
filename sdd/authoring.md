# SDD 작성 및 구조 원칙

## 원본과 탐색 층

각 규칙에는 **하나의 현행 Spec 원본**만 둔다. 제품 목표, 도메인 상태, 보안 승인, AI 계약, UI 정책, 사용자 흐름은 SDD 색인에서 소유 문서를 찾을 수 있어야 한다. ADR은 선택의 당시 이유를 기록하고 현행 규칙을 복제하거나 소급 수정하지 않는다. `WORKLOG.md`와 QA는 구현·검증의 증거를 기록하며 Spec 완료를 대신 선언하지 않는다. 외부에서 이식한 문서와 AI 답변은 참조 자료이고, 앱의 보안·승인 원본이 아니다.

## 주제 폴더를 만드는 기준

다음 두 조건을 모두 만족하면 `sdd/<topic>/00_index.md`와 엔티티별 파일로 나눈다.

1. 같은 주제에 **둘 이상의 독립적인 책임·수명주기·변경 이유**를 가진 엔티티 또는 흐름이 있다.
2. 이들 사이의 선행 조건, 소유, 전이, 예외, 참조 관계를 추적해야 새 변경의 영향 범위를 알 수 있다.

문서가 길다는 이유만으로 나누지 않는다. 짧더라도 관계가 중요한 주제는 나눌 수 있다. 하나의 불변식 또는 보안 경계가 여러 개념을 함께 통제한다면 그 규칙은 한 원본에 두고 각 엔티티에서 링크한다. 제품·보안처럼 전역 정책인 문서는 주제 폴더에 복제하지 않는다. 별도의 독립 엔티티가 생기기 전까지 단일 정책·조사 기록·한 기능 계획은 한 파일로 유지한다.

## 폴더와 엔티티 파일 계약

- 폴더 인덱스는 주제 목적, 용어/ID, 엔티티 목록과 **각 파일이 소유하는 사실**, 관계 어휘, 읽기 순서를 제공한다. 규칙 본문을 다시 쓰지 않는다.
- 엔티티 파일에는 안정적인 ID, 범위, 고유 규칙, 입력·출력 또는 상태, 다른 엔티티와의 **타입이 있는 관계**, 관련 상위 Spec·ADR·검증 링크를 둔다. 같은 개념을 두 파일이 소유하지 않는다.
- 관계 이름과 방향은 폴더 인덱스에 정의한다. 링크 대상 ID가 실제 파일에 있고 의미가 맞는지 확인한다. `requires`는 선행 조건이지 자동 실행이 아니며, `references`는 규칙을 물려받는다는 뜻이 아니다.
- 사용자 여정 그래프의 `requires/next/branch/resume/observe`는 [userFlow 인덱스](userFlow/00_index.md)가 소유한다. 다른 주제는 필요한 최소한의 관계 어휘만 정하고, 엔진 상태 전이나 승인 권한을 그래프에서 새로 만들지 않는다.
- 옛 경로를 바꿀 때는 참조를 새 원본으로 갱신한다. 외부 링크 호환이 필요하면 옛 파일을 **본문이 없는 이동 안내**로 남길 수 있다. 이동 안내는 두 번째 원본이 아니다.

## 변경 절차

1. [SDD 색인](00_index.md)에서 소유자를 찾고 인접 엔티티의 링크를 확인한다. 분리 기준에 맞는지 판단해 해당 원본만 고친다.
2. 새 엔티티라면 안정 ID, 인덱스 행, 관계를 추가한다. 기존 관계의 역방향 또는 조건도 확인한다. 경계를 바꾸는 설계 선택은 새 ADR로 기록한다.
3. 관련 [userFlow](userFlow/00_index.md)의 사용자 행동·분기만 갱신하고, 구현·QA 지도를 동기화한다. 파일 이동 시 저장소 전체의 Markdown 링크를 확인한다.
4. `WORKLOG.md`에 현재 규칙, 구현 여부, 검증 근거, 다음 행동을 남긴다. 문서 관계가 맞는 것과 데스크톱 실행이 검증된 것은 별도로 표시한다.

이 구조는 **문서 그래프**다. RDF 추론이나 런타임 권한 시스템을 암시하지 않는다. 나중에 기계 판독이 필요해도 현재 원본 소유권과 승인 경계를 그대로 유지해야 한다.

## 현재 SDD에 적용한 경계

| 형태 | 주제 | 판단 |
| --- | --- | --- |
| 주제 폴더 | [userFlow](userFlow/00_index.md), [goalConversation](goalConversation/00_index.md), [specImport](specImport/00_index.md), [aiIntegration](aiIntegration/00_index.md), [domain](domain/00_index.md) | 독립적으로 바뀌는 여정·계약·엔티티와 영향 관계가 있다. |
| 주제 폴더 | [meeting](meeting/00_index.md) | Meeting 단계와 전송 packet/승인은 서로 다른 수명주기와 검증 경계를 갖는다. |
| 검토 토론 폴더 | [multiAgentMeeting](multiAgentMeeting/00_index.md) | 한 제안을 여러 에이전트와 사용자가 제안·비판·응답·개정·결정 문서로 주고받는 추가 전용 기록이다. 인덱스가 문서 상태·관계·순서를 소유하며 현행 Spec 원본이 아니다. 채택된 내용만 소유 Spec과 ADR로 이관한다. |
| 역사 색인 | [architecture/decisions](architecture/decisions/00_index.md) | ADR의 번호·결과를 찾는 기록 색인이다. 현행 Spec의 엔티티 그래프나 규칙 원본이 아니다. |
| 단일 원본 | [product](product.md), [security](security.md), [architecture](architecture.md), [validation](validation.md), [workflow](workflow.md) | 전역 목적·경계·검증·절차가 각각 한 불변식으로 묶인다. 분리하면 같은 규칙을 반복할 위험이 크다. |
| 단일 정책·계획·조사 | [progress-visibility](progress-visibility.md), [content-density](content-density.md), [office-goal-visibility-plan](office-goal-visibility-plan.md), [workflow-improvement-plan](workflow-improvement-plan.md), [provider-runtime-investigation](provider-runtime-investigation.md) | 현 시점에 별도 수명주기를 가진 하위 엔티티가 없어 한 파일로 유지한다. |

이 표는 현재 소유권 감사 결과다. 새 책임이 생기면 위의 두 조건을 다시 적용한다.
