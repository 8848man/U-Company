# DM-06 — 참조 자료

CEO가 첨부한 긴 글과 이미지는 목표 문장이 아니라 프로젝트 안 `work-references/`의 파일로 둔다([ADR-0049](../architecture/decisions/ADR-0049.md)). 파일과 인덱스는 앱이 정해진 규칙으로만 만든다. 원본 기록은 SQLite `reference`이고 인덱스 파일은 그 기록에서 다시 만드는 사본이다.

## 엔티티

| 엔티티 | 속성 | 메모 |
| --- | --- | --- |
| Reference | `id`, `projectId`, `goalId`(보내기 전 없음), `label`(R1…), `kind`(text·image), `role`(requirement·reference), `source`(paste·file·image), `title`, `originalName`, `originalBytes`, `sha256`, `chars`, `width`, `height`, `bytes`, `path`, `summary`·`summarySource`(rule·curator·cto·ceo), `caption`, `tags`, `relations`, `curation`(none·pending·done·failed·skipped), `detail`, `status`(staged·ready·missing) | 첨부하면 `staged`(앱 데이터에 보관), 목표를 보내면 `ready`, 파일이 사라지면 `missing` |
| Section | `n`, `title`, `file`, `chars`, `firstLine`, `summary`, `background`·`backgroundReason` | 20,000자를 넘는 글만. 절을 이어 붙이면 원문과 같다. 요구사항 자료에서 배경이 아닌 절은 계획이 모두 반영해야 한다 |

## 관계

| 관계 | 무엇에서 → 무엇으로 | 기록하는 쪽 |
| --- | --- | --- |
| `has_reference` | 목표 → 자료 | 목표를 보낼 때 앱 |
| `part_of` | 절 → 자료 | 나눌 때 앱 |
| `derived_from` | 자료 → 원본(이름, 크기, SHA-256) | 첨부할 때 앱 |
| `uses` | 업무 → 자료·절 | PM 계획의 근거 자료를 엔진이 검증한 뒤 |
| `covers` | 계획 → 요구사항 절 | 요구사항 절이 업무 근거 자료에 있거나 이유와 함께 제외됐는지 엔진이 확인 |
| `assigned_to` | 업무 → 팀원 | 엔진 배정 |
| `inherits` | 새 목표 버전 → 이전 버전의 자료 | 방향 변경으로 새 버전이 생길 때. 파일은 복사하지 않는다 |
| `illustrates`, `relates_to` | 자료 → 자료 | 자료 정리 담당의 제안을 앱이 확인한 뒤 |

## 수명

첨부 → 보관(staged, 7일 지나면 정리) → 보내기(ready, 폴더·인덱스 생성) → 목표 보관(그대로 유지) → 백업 삭제(폴더 삭제. 새 버전이 물려받아 쓰고 있으면 기록을 그 버전으로 옮기고 파일은 남김).

## 관계와 소유 경계

| 관계 | 대상 | 의미 |
| --- | --- | --- |
| belongs_to | [DM-02 목표](DM-02-goal.md) | 자료는 목표에 속하고 새 버전이 물려받는다. |
| reads | [DM-03 업무·증거](DM-03-execution.md) | 업무의 근거 자료가 자료·절을 가리킨다. |
| constrained_by | [보안](../security.md) | 자료는 데이터이며 앱만 쓴다. |
