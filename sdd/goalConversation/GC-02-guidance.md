# GC-02 — Guidance and safe checkpoints

## Safe delivery checkpoints

The engine checks persisted pending guidance before CTO analysis, after CTO/PM calls, before requesting or consuming project-write approval, before and after each developer dispatch wave, and before/after QA/rework. A currently running provider invocation or worktree is never retroactively changed. Guidance that arrives during a call waits visibly for the next checkpoint. At that checkpoint the old goal blocks and pending approval is superseded. An AI-routed direction creates and starts a linked goal automatically; manually submitted legacy guidance remains reviewable. The new goal must receive a new plan and approval for new file work; an older project-write approval cannot authorize new files. Already integrated work stays attributable to the older goal, and its current project contents become the starting point for a separately approved new goal. If a developer wave finished but was not integrated, its worktree remains for inspection; the UI must not claim it was applied.

Proposal acceptance and goal completion must be mutually exclusive transactions under one store lock. A proposal received before completion prevents the old goal from being marked complete; a proposal received after completion is rejected with a follow-up-goal instruction. Pending approval invalidation and proposal persistence belong to the same transaction so a restart cannot retain a valid old approval for a queued pivot.
Creating the replacement goal and marking guidance revised is also one transaction; retries or two windows must not create two replacements. After replacement, further direction changes belong to the new goal rather than branching the old one again.

Persist guidance status, checkpoint, replacement goal ID, conversation turns, and the original goal's plan/approval/session association. Preserve original text. A replacement is a linked new goal ID, not an in-place rewrite. On screen the versions read as one conversation and are archived together ([ADR-0051](../architecture/decisions/ADR-0051.md)). On restart, reconstruct pending decisions from storage; never infer that an unobserved response or model call finished. All write and transmission rules in [Security](../security.md) apply to the conversation path.

## 관계와 소유 경계

| 관계 | 대상 | 의미 |
| --- | --- | --- |
| reads | [GC-01 대화 턴](GC-01-turns.md) | CEO의 정확한 원문과 선택 팀원을 인계 근거로 사용한다. |
| routes_to | [GC-04 복구](GC-04-recovery.md) | 인계·계획이 실패하면 이유를 저장한다. |
| constrains | [보안](../security.md) | 오래된 승인과 미검증 재사용은 허용하지 않는다. |

## 검증 참조

[주제 수용 기준](acceptance.md), [수동 QA](../../qa/MANUAL-QA.md). 구현 증거와 미완료 항목은 [WORKLOG](../../WORKLOG.md)를 따른다.
