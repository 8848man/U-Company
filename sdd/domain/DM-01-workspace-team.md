# DM-01 — Workspace, project, and team

## Entity definitions

- **Workspace:** the local collection of registered projects and its active project reference.
- **Project:** stable ID, canonical local folder, name, and execution mode (Git root or ordinary folder). Registration does not modify the chosen folder.
- **Organization:** a project-scoped collection of configured role characters.
- **Imported specification profile:** a project-scoped, versioned reference containing external project context, role placeholders/bindings, workflow text, and source-attributed exception specifications. The current workspace-wide character records and in-flight goals are not overwritten when a profile is applied.
- **Agent:** one configured character with role, instructions, reporting relation, runtime and model references, permission profile, office position, and enabled state. Disabling an implementation character keeps historical references; an agent with unfinished assigned tasks cannot be disabled.
- **Agent busy state:** one backend rule derived from saved facts decides whether a character may start new work: it is busy as chair, participant or evaluator of a meeting in `planning`, `awaiting_transfer_approval` or `proposing`–`evaluating`; as the recipient of a `dispatching` or `outcome_unknown` meeting packet; as the analysis lead, planning lead or reviewer of an execution Goal's squad while that Goal is `analyzing`, `planning` or `validating` respectively (Goals without a squad record read the `cto`, `pm` and `qa` IDs, [ADR-0041](../architecture/decisions/ADR-0041.md)); as external staff on standby (never auto-assigned, no meetings) or contracted to another Goal; or as assignee of a `pending`, `running` or `awaiting_integration` task. Meeting plan approval, execution start, task assignment and the task claim re-check it in their transactions. One character never holds two sessions, so two active goals do not share an implementation teammate.

## 관계와 소유 경계

| 관계 | 대상 | 의미 |
| --- | --- | --- |
| owns | [DM-02 목표](DM-02-goal.md) | 목표는 생성 시의 프로젝트 ID에 영구히 묶인다. |
| records | [SI-01 명세 프로필](../specImport/SI-01-import.md) | 이식 명세의 로컬 투영은 프로젝트별로 저장된다. |

## 검증 참조

[검증 기준](../validation.md), [수동 QA](../../qa/MANUAL-QA.md). 구현 증거와 미완료 항목은 [WORKLOG](../../WORKLOG.md)를 따른다.
