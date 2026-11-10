# DM-05 — Restart and interrupted work

## Recovery

### 변경 없는 작업과 부분 통합 (ADR-0072)

변경 0건은 실제 패치 충돌과 다른 `no_changes`로 기록한다. Dev의 보고는 관찰 자료이고, 엔진이 선언 파일 안에서 존재 인용 또는 리터럴 부재 검색을 다시 확인해 파일별 SHA-256·결과·시각을 별도 `task_no_change_proof`에 저장한다. `task_integrated`나 `task_proof`는 만들지 않는다. 확인 근거가 있는 변경 없음 작업은 `completed`로 옮겨 독립 QA에 넘긴다. 이 상태는 QA 합격을 뜻하지 않는다. QA는 목표의 모든 원래 산출물을 수용 기준별로 판정하고, 합격하면 변경 없음 기록에 그 판정을 연결한다. 불합격하면 기존 재작업으로 간다. 이전 저장 데이터의 근거 없는 `task_unchanged`는 자동 재사용하지 않는다.

한 차례에서 독립 작업의 통합은 각각 시도한다. 실패마다 구분된 오류 종류를 기록한 뒤 목표를 멈추며 의존 작업은 다음 차례로 진행하지 않는다. 이미 통합된 파일과 증거는 보존한다. `no_changes` 복구의 `충족 처리`는 엔진 확인 근거가 있을 때만 해당 작업을 `completed`로 옮긴다. 해당 차례의 나머지 결과를 보존하고, 모든 미완료 작업이 `pending`이 되면 기존 승인과 claim 검사를 거치는 재개 명령으로 남은 작업을 실행한다. 새 파일 승인이나 범위 확대는 없다. 아직 `awaiting_integration`인 작업에 근거가 없다면 충족 처리는 거절하고 확인 자료가 필요하다고 알린다. `재계획`은 새 목표 버전과 새 승인을 거친다. 같은 목표 계보에서 저장된 오류 종류·정렬한 선언 파일 집합 서명이 두 번 반복되면 자동 재계획을 막고 CEO의 충족 처리 또는 직접 지시를 기다린다.

For an application plan with presets, restart loads the saved plan revision and immutable selection snapshot, not the project's current setting. A pending card keeps its original fingerprint; a changed setting cannot silently update it. A missing definition, corrupt snapshot, mismatched fingerprint or unknown version blocks new approval/claim with a visible reason and offers restore of the exact saved definition or explicit replanning. It never falls back to a newer catalog version. An interrupted approved task follows the existing guarded resume path with the approved snapshot and prior file scope. A changed choice after approval takes a safe checkpoint and linked Goal; already integrated task reuse still requires [ADR-0027](../architecture/decisions/ADR-0027.md) file and dependency digest checks. No restart or recovery action infers a performance result or revives an old approval ([ADR-0062](../architecture/decisions/ADR-0062.md)).

For a linked goal or repair touching a reused file ([ADR-0068](../architecture/decisions/ADR-0068.md)), recovery re-runs the overlap and approval-scope checks before showing an approval card and after restart. A pending conversion is not completion: only a verified integration event with new current-byte digest and the new task/approval link changes file ownership. QA re-entry uses that new proof for converted files and the preserved old proof for all other reused files and dependencies. On an out-of-scope or untraceable change, keep the goal blocked and show the file diff, recorded/current hashes, missing event/task/approval evidence, and the applicable restore, fresh approval/replan, or evidence-retrieval action. Restart never silently adopts changed bytes or an old approval.

At startup, an interrupted `running` task becomes `blocked` with a recovery reason until its native session can be resumed or a new run is authorized. Its outcome is unknown: the isolated workspace may hold partial changes, so it is never re-run or integrated automatically. Pending approvals remain pending. The engine never infers completion from a process disappearing or a UI animation.

When the engine blocks an execution Goal it records the block kind from the blocking site, never from model text: `precall` (claim or connection check failed before any call), `approval_denied`, `interrupted` (restart), `integration_conflict`, `qa_failed` (rework cap), `needs_scope` (a repair needs files that cannot be added on the same goal, or the CEO declined adding them — [ADR-0053](../architecture/decisions/ADR-0053.md)), `qa_form` (no valid per-deliverable verdict), `plan_form` (the planner's plan failed the app's checks twice; the first failure is sent back once automatically, [ADR-0048](../architecture/decisions/ADR-0048.md)), `provider` (a started call failed) or `other`. The recovery card's one-click `해결 요청` routes a review-stage block with every task completed and an approved scope to a rework on the same goal inside the approved files, and other blocks to an automatic new goal version whose plan needs approval; connection blocks and CEO denials are not routed to a teammate. A goal blocked with only never-started `pending` tasks under a still-approved project-write approval may resume its existing plan through the guarded resume command, which re-runs the claim checks; completed tasks keep their integration proof. Every other kind follows [GC-04](../goalConversation/GC-04-recovery.md). A quiet runtime is a diagnostic signal only and never changes a state.

An interrupted deliberation Goal keeps its Goal kind and coarse status. The Meeting becomes blocked, any in-flight provider call and usage are uncertain, pending exact packet decisions survive, and a meeting-scoped preapproval expires. Restart never resends a meeting packet automatically. A meeting-specific resume checks source hashes, destination identity, limits and current approval before a new call ([ME-01](../meeting/ME-01-process.md), [ME-02](../meeting/ME-02-packets.md)).

## 관계와 소유 경계

| 관계 | 대상 | 의미 |
| --- | --- | --- |
| recovers | [DM-04 수명주기](DM-04-lifecycle.md) | 재시작은 관찰되지 않은 완료를 추정하지 않는다. |
| reads | [DM-03 업무·증거](DM-03-execution.md) | 저장된 작업·승인·세션 기록을 확인한다. |

## 검증 참조

[검증 기준](../validation.md), [수동 QA](../../qa/MANUAL-QA.md). 구현 증거와 미완료 항목은 [WORKLOG](../../worklog/00_index.md)를 따른다.
