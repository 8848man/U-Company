# DM-04 — Goal and task lifecycle

## Lifecycle

Execution Goals progress through `new → analyzing → planning → awaiting_approval → executing → integrating → validating → completed`. An execution Goal may enter `blocked` from an active stage, or `cancelled` from `awaiting_approval` when the CEO rejects the plan ([ADR-0046](../architecture/decisions/ADR-0046.md)); `cancelled` is terminal, writes nothing and cannot be run or resumed; the initial MVP can automatically resume only a plan interrupted while awaiting project-write approval. Deliberation Goals use `deliberating → awaiting_decision → completed`; [ME-01](../meeting/ME-01-process.md) owns their detailed meeting stage, including chair planning, plan approval, interruption and partial closure. Starting an execution Goal is refused while its CTO, PM, QA or a developer is busy in a meeting ([DM-01](DM-01-workspace-team.md)). Execution commands reject deliberation Goals on the backend; meeting commands reject execution Goals. Tasks progress through `pending → running → awaiting_integration → completed`, with `blocked` and `failed` as explicit outcomes. When the CEO asks to revise the plan before approval, the pending approval becomes `revise_requested` and the replaced plan's tasks, which never started, are removed; the replaced approval keeps their full scope text as the record. Each plan pass gets its own approval, and only the pending one can be approved, so an older plan can never start work. Unstarted tasks of a `cancelled` goal keep nobody busy. A task becomes completed only after its declared file changes apply to the selected project. Dependencies are a directed acyclic graph. Only tasks whose dependencies are completed can be dispatched. Each status change creates an event in the same transaction as the snapshot update.

**Archive is not a status** ([ADR-0043](../architecture/decisions/ADR-0043.md)). An idle goal (execution `new`/`completed`/`blocked` with no running or integrating task; a meeting after its decision, stop or block) can be archived: a separate `goal_archive` record hides it and everything it owns from the snapshot while its status and records stay unchanged. Archived goals cannot be run or resumed until restored. Permanent deletion is allowed only for archived goals that no visible goal was created from, and keeps token usage records.

A goal blocked during analysis or planning can be retried from its existing card only when it has no tasks or approvals. Retrying clears the previous error report and restarts analysis. Later-stage goals retain their stricter inspection and approval recovery path.

The PM proposes a task graph of file-changing implementation tasks. Every task declares relative output files and acceptance criteria; a deliverable task also declares its deliverable, read-only evidence paths and required capability ([DM-03](DM-03-execution.md)). The engine rejects traversal paths, duplicate file ownership, empty acceptance criteria, unknown capabilities and evidence paths that are not existing project files, assigns each task by the checked assignment rule, and validates acyclicity before requesting approval. It then claims and runs independent tasks in parallel; tasks sharing an assignee run one after another. QA and CEO reporting are separate roles after integration, never implementation tasks. Integration rejects undeclared file changes and patch conflicts. Native runtime sessions and artifacts are recorded before declaring a task complete.

The task graph may contain one implementation task for a single-file goal. The planner must not invent a second deliverable merely to create parallelism. It uses separate file ownership and concurrent worktrees only when the requested output genuinely has independent file changes.

A registered Git project may contain tracked or untracked local changes when the CEO starts a goal. The engine snapshots the current project contents into each isolated worktree immediately before developer execution, preserving the original working tree. Integration applies only the developer's declared-file delta onto that original working tree and fails on a conflict rather than discarding local content. This does not widen the set of files approved for AI changes.

For deliverable tasks QA answers per task and per criterion. The goal completes only when the overall verdict passes and every deliverable task passes with every criterion met and evidenced. A missing or malformed per-deliverable answer blocks the goal as a QA form failure without spending a rework round. If QA rejects the integrated result, the engine records the review, increments the goal's rework round, asks PM for concrete repair tasks, runs developers in new isolated worktrees based on the integrated project snapshot, integrates declared changes, and asks QA again. Each goal owns its maximum *additional* repair rounds, copied from the project's 0–5 preset at creation. Changing a goal's cap while it runs takes effect at its next QA decision; changing the project preset affects only future goals. Legacy goals without a saved cap inherit the project preset once during storage migration. When the cap is exhausted, the goal blocks with the latest QA reason. A repair plan may modify only files in the original project-write approval; a new file or broader scope requires a separate CEO approval and is not silently attempted. The loop count is persisted so a restart never resets it.

## 관계와 소유 경계

| 관계 | 대상 | 의미 |
| --- | --- | --- |
| requires | [DM-02 목표](DM-02-goal.md) | 목표 상태가 실행의 맥락이다. |
| requires | [DM-03 업무·증거](DM-03-execution.md) | 의존성·통합·QA 근거가 전이를 제한한다. |
| recovers | [DM-05 중단 복구](DM-05-recovery.md) | 중단된 상태는 보수적으로 재판정한다. |
| refined_by | [ADR-0043 보관·영구 삭제](../architecture/decisions/ADR-0043.md) | 보관은 표시 겹침이며 상태를 바꾸지 않는다. |

## 검증 참조

[검증 기준](../validation.md), [수동 QA](../../qa/MANUAL-QA.md). 구현 증거와 미완료 항목은 [WORKLOG](../../WORKLOG.md)를 따른다.
