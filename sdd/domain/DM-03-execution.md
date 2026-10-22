# DM-03 — Task, evidence, and approval

## Entity definitions

- **Task integration proof:** per-file digest captured when an approved task's declared changes reach the selected project. It is evidence for later reuse, not a grant to write or a substitute for QA. Older tasks without proof remain ineligible for automatic reuse.
- **Task:** goal-scoped work unit with assignee, dependency IDs, acceptance criteria, worktree and artifact references. A *deliverable task* ([ADR-0038](../architecture/decisions/ADR-0038.md)) also stores a one-line deliverable, 1–8 acceptance criteria, 0–8 read-only evidence paths inside the project, a required capability (`document` or `code`) and the engine's assignment reason. Older tasks without these fields stay readable and follow the earlier QA rule.
- **Focused execution guidance:** the PM names task-specific evidence and useful focused checks. The worker follows required project guidance and reads declared evidence first, then inspects other files as acceptance criteria require. It avoids repeating wide searches or large command outputs, keeps intermediate explanations and the final summary concise, and runs every required test before completion. The guidance changes neither approved file scope nor independent QA. Its effect on internal response count and context size remains unverified until a paired real run.
- **Assignment:** the engine, not the planner, picks exactly one enabled implementation character per task whose saved connection is usable and can write files in an isolated workspace and who is not busy under [DM-01](DM-01-workspace-team.md) for another goal or meeting. It balances by tasks already assigned in the plan, then by roster order, and records why. QA, PM and CTO never receive implementation tasks. With no eligible candidate the plan blocks before approval; another AI is never substituted silently.
- **Display summary:** the analysis lead's reply starts with one `요약:` line (plain Korean, ≤ 80 characters) that the engine stores as a `goal_summary` record for goal titles ([ADR-0042](../architecture/decisions/ADR-0042.md)). It is display-only and optional: a missing line changes nothing, the stored analysis keeps the full reply, and no state, approval or assignment reads it.
- **Work kind ([ADR-0045](../architecture/decisions/ADR-0045.md)):** the next line is `업무: 개발` or `업무: 기타`. The engine stores it, with the static SDD origin detection, as a `goal_origin` record. A missing or unclear line counts as `기타`. Only a `개발` goal may get the planning card's structure question ([SI-03](../specImport/SI-03-origin.md)); when that question could be asked and static detection listed unclassified rule files, the prompt names only those files and the CTO may add one display-only `구조 조언:` line. The record never changes the plan, approval, assignment or project files.
- **Claim:** the `pending → running` change is one SQLite transaction that re-checks pending status, completed dependencies, a still-approved project-write approval, and the assignee's enabled state, usable capable connection, absence of another running task and of a meeting reservation. A workspace is prepared and a provider called only after the claim commits; a failed claim leaves the task `pending` and makes no call.
- **Deliverable review:** for deliverable tasks, independent QA returns one structured verdict per task with one result per acceptance criterion (`met` and concrete evidence) and unverified items. The engine stores it per task and QA round. It is evidence for the CEO's acceptance, not a write grant.
- **Session:** one runtime execution for one agent/task, with native session identifier and resume capability.
- **Event:** ordered, append-only fact about a goal, task, session, approval, or artifact.
- **Progress event:** a bounded, persisted event tied to goal, task when applicable, and agent. It records verified runtime or engine milestones such as tool start/completion, stage change, integration, or failure. It never represents model private reasoning.
- **Approval:** a requested material decision with pending/approved/denied status and actor.

## 관계와 소유 경계

| 관계 | 대상 | 의미 |
| --- | --- | --- |
| belongs_to | [DM-02 목표](DM-02-goal.md) | 업무·승인·증거는 해당 목표와 연결된다. |
| transitions | [DM-04 수명주기](DM-04-lifecycle.md) | 완료는 통합 결과와 검증에 근거한다. |
| recovers | [DM-05 중단 복구](DM-05-recovery.md) | 앱 재시작 때 미완료 세션과 승인을 다시 판단한다. |

## 검증 참조

[검증 기준](../validation.md), [수동 QA](../../qa/MANUAL-QA.md). 구현 증거와 미완료 항목은 [WORKLOG](../../WORKLOG.md)를 따른다.
