# 명령 권한 승인과 재작업 원인 수정 (2026-10-26, Claude)

브랜치 `feature/permission-approval`(기준 `origin/main` `7bd1d11`). 첫 커밋 `9dfbfd6`은 2026-10-25 작업 폴더에 커밋되지 않고 남아 있던 변경(ADR-0055·0056, `goal_context.rs`, `worklog/`)을 CEO 승인으로 그대로 보존한 것이다(백업 `.run-data/backup-uncommitted-20261025-153822.tgz`).

계기: PostGen 후속 차수(목표 `35d86ef5`)가 QA에서 두 번 PM으로 돌아간 뒤 통합에서 멈췄다. 기록 분석 결과 원인은 (1) Claude 팀원·QA가 명령을 실행할 수 없음, (2) 같은 차수 업무 사이 결과 미전달, (3) 재작업 요청문의 소문자 파일 이름으로 인한 통합 충돌, (4) 목표 대화 인계에 QA 판정 없음. 결정: [ADR-0057](../../../sdd/architecture/decisions/ADR-0057.md)(1·3·4번 권장안), [ADR-0058](../../../sdd/architecture/decisions/ADR-0058.md)(명령 권한 승인: 목표 대화 범위, QA는 `dontAsk`+복사본, 계획 카드에서 미리 + 막히면 다시).
