# Organization domain topic index

제품이 저장하는 조직·목표·업무의 어휘와 그 상태 변화를 찾는 지도다. 상태 전이, 재시작 복구, 각 엔티티의 속성을 같은 문서에 섞지 않는다. [보안](../security.md)은 프로젝트 경계와 승인 권한을 별도로 소유한다.

| ID | 원본 | 소유하는 사실 |
| --- | --- | --- |
| DM-01 | [작업공간·프로젝트·팀](DM-01-workspace-team.md) | 등록 프로젝트, 조직, 구성원, 이식 명세 프로필의 정체성 |
| DM-02 | [목표·대화·개정](DM-02-goal.md) | 목표와 대화/지침, 이어지는 목표 버전의 기록 |
| DM-03 | [업무·증거·승인](DM-03-execution.md) | Task, Session, Event, 통합 증거와 승인 엔티티 |
| DM-04 | [수명주기](DM-04-lifecycle.md) | 목표·업무 상태 전이와 QA 재작업 루프 |
| DM-05 | [중단 복구](DM-05-recovery.md) | 재시작 때 미완료 실행의 보수적 판정 |

관계 `owns`는 하위 기록의 소유, `belongs_to`는 저장된 소속, `requires`는 선행 상태, `records`는 증거 생성, `transitions`는 상태 이동, `recovers`는 재시작 후 재판정, `reads`는 저장된 사실의 조회를 뜻한다. 실제 전이와 복구의 원본은 DM-04/DM-05이며 개별 엔티티 파일은 그 정의를 참조한다. [사용자 여정](../userFlow/UF-03-goal.md)은 CEO가 보는 순서를, [목표 대화](../goalConversation/00_index.md)는 대화의 상세 계약을 소유한다.
