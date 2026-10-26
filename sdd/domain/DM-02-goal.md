# DM-02 — Goal, conversation, and revision

## Entity definitions

- **Goal:** CEO intent, immutable project ID, kind (`execution` or `deliberation`), constraints, status, and final report. Execution Goals own an independent QA rework cap and optional display-only work-group label; the cap is copied from the project's preset at creation. A deliberation Goal has exactly one [Meeting](../meeting/ME-01-process.md) and does not use development QA caps or work groups. Switching projects or later changing the preset never rewrites an existing goal. A work-group label never changes task ownership or approval scope.
- **Goal conversation turn:** goal-scoped CEO question and read-only AI answer, with actual agent/connection/model, timestamp, and pending/answered/failed/interrupted state. It does not mutate the goal.
- **Goal guidance:** exact CEO direction-change proposal with pending/review-required/revised state, addressed team member, optional read-only worker escalation memo, engine checkpoint, linked replacement goal, and continuation decision. A replacement goal re-enters CTO/PM review; new file work requires a new approval. Old tasks and approvals remain attributable to the old goal. The versions linked this way are one conversation on screen and are archived and restored together ([ADR-0051](../architecture/decisions/ADR-0051.md)).
- **Post-completion follow-up:** a completed execution Goal can receive a new CEO work instruction in its visible conversation ([ADR-0055](../architecture/decisions/ADR-0055.md)). The engine records the exact instruction and creates a linked Goal ID for the next work pass. The completed record stays completed; the new ID owns its plan, approvals, tasks and QA. This is a continuation of one user-facing goal conversation, not permission to reuse an earlier write approval.
- **Conversation objective ([ADR-0059](../architecture/decisions/ADR-0059.md)):** the original CEO objective of a version chain, keyed by its first goal, with state `open`, `checking`, `achieved`, `closed_by_other` or `reopened`, the CTO's summary, evidence, remaining work, the goals that achieved it and the project changes it saw. It is checked when a version completes and when the CEO resumes. Closing records state only; goals, tasks, approvals and files are unchanged and `다시 열기` restores the conversation.
- **Work-pass handoff:** the next Goal's planning intent includes a bounded structured context from saved earlier records ([ADR-0056](../architecture/decisions/ADR-0056.md)). Exact source records stay with their original Goal IDs. This context is descriptive and carries no approval or proof of reused work.
- **Revision continuation decision:** the linked old/new goal IDs, accepted source task IDs, verified file digests, and remaining work decision. CTO and PM recommend; the engine validates. The decision is saved independently of the model's prose and rechecked before final QA.

## 관계와 소유 경계

| 관계 | 대상 | 의미 |
| --- | --- | --- |
| belongs_to | [DM-01 프로젝트](DM-01-workspace-team.md) | 목표의 프로젝트 소속은 선택 변경으로 바뀌지 않는다. |
| records | [DM-03 업무·증거](DM-03-execution.md) | 업무와 승인은 목표에 귀속된다. |
| transitions | [DM-04 수명주기](DM-04-lifecycle.md) | 단계 전이와 QA 한도의 적용을 따른다. |
| belongs_to | [ME-01 회의](../meeting/ME-01-process.md) | 회의 Goal의 안건과 결과가 Goal ID에 귀속된다. |

## 검증 참조

[검증 기준](../validation.md), [수동 QA](../../qa/MANUAL-QA.md). 구현 증거와 미완료 항목은 [WORKLOG](../../worklog/00_index.md)를 따른다.
