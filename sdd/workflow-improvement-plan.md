# Goal-centered workflow improvement plan

Status: Active delivery plan. The checkbox evidence and resume point live in `WORKLOG.md`.

The proposed [whole-app UX simplification plan](uxSimplification/00_index.md) addresses menu and information density across all surfaces. This plan remains the goal-centered workflow delivery record; neither document marks the proposed navigation as implemented.

## Product direction

Build a goal workspace rather than more disconnected status panels. Codex-style mid-task steering is necessary; this product also has explicit roles, delegation, file ownership, approvals, QA, token provenance, and restart records. The interface makes those relationships legible without exposing raw prompts or private reasoning. The CEO can discuss intent before execution, steer at safe checkpoints, inspect why work is blocked, and decide from a concrete plan/diff.

## Delivery slices

1. **Goal conversation foundation:** persisted goal-scoped turns, read-only understanding call through the configured CTO, provider/model identity, errors and restart recovery. Entry from CEO and Office. No false claim of live steering.
2. **Guidance queue and checkpoints:** persist CEO guidance and pending status; engine checkpoints before planning, approval, each dispatch wave, and QA. The first safe implementation paused the old goal and created a linked new goal. [ADR-0027](architecture/decisions/ADR-0027.md) extends that path: an addressed worker escalates to CTO, PM plans remaining file work, the engine verifies any completed-task reuse, and only new writes request fresh approval. Add integration tests for message arrival during a slow call, restart, denial, and repeated guidance. A future proven scope classifier may permit narrower clarification within an existing approval.
3. **Goal workspace:** one view with conversation, current stage/owner, approved scope, task graph, latest evidence, decisions, QA loop, and token details. Bring the previously planned Office current-goal rail into this view as a compact navigation projection. Share selectors with the board; retain Office characters as a secondary spatial view.
4. **Interaction consistency:** every action has visible preconditions and recovery instructions; toasts link to the resulting entity; all lists and long text use `sdd/content-density.md`; loading, stale/backend-version, partial failure, keyboard, and narrow layouts are handled consistently. Test with novice users using tasks, not feature names.
5. **Operational closure:** finish existing real Windows end-to-end QA, runtime cancellation/streaming and recovery, approval enforcement, drift review, then Ubuntu build and real app flow. These are not complete until evidence is in `WORKLOG.md`. Do not auto-install system packages merely to mark Ubuntu done.

## Experience contract by surface

| Surface | First answer the CEO should get | Primary action | Trustworthy evidence |
| --- | --- | --- | --- |
| CEO desk | Which goal needs me, and what happens next? | Ask, change direction, approve, or start | Goal stage, owner, pending guidance, report link |
| Goal workspace | Did the team understand the intended outcome? | Discuss or revise exact goal text | Saved turns, actual model, plan revision, approval scope |
| Office | Who is doing the work right now? | Open goal or teammate detail | Persisted stage/task events and assignee IDs |
| Board | What was delegated and where is it blocked? | Inspect task/approval/QA evidence | File ownership, dependencies, test outcome, timestamps |
| AI connections | Which runtime can do this role? | Diagnose or change assignment | Local executable/status and capability, not just preset name |
| Token usage | Where did the project budget go? | Inspect by goal/member/model | Invocation ledger with estimates distinguished from reported usage |

The goal workspace should become the shared destination behind cards and toasts, reducing repeated detail panels. Every action shows its effect on the saved entity. For a blocked action, explain the exact unmet condition and the next repair step. For a long operation, show active owner, last confirmed event, and the stage at which new CEO guidance will be considered. A success toast links to the changed goal, rather than requiring the CEO to search for it.

## Earlier plan disposition

The previously planned Office current-goal rail is partly delivered in slice 3; its detailed expansion and live layout checks remain open. The historical Windows QA-03B/05G/17–21 checklists are still live acceptance gates, not new implementations to duplicate. Runtime streaming/cancellation and full session resume, stronger runtime operation approvals, and Ubuntu verification are still open foundational work. The unified goal workspace should absorb the older board/Office detail ideas before adding more independent panels. `WORKLOG.md` keeps the exact unchecked evidence and next steps.

## Review gates

At each slice, update the owning Spec first; add an ADR only for cross-boundary decisions; implement; run focused behavior tests and mechanical checks; record a reproducible user flow and remaining limits. A release claim needs a fresh app run. Track usefulness by time to locate the active owner/next action, ability to explain a blocked state, guidance-to-checkpoint latency, approval correctness, and successful restart recovery. Token counts are observed, not optimized by hiding evidence.
