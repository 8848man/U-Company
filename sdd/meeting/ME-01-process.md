# ME-01 — Goal-bound Office-character meeting process

## Identity and scope

A deliberation Goal belongs to one registered project and exactly one Meeting. Participants are existing Office team members (Agents); a meeting never creates Agents. When a plan is approved the engine snapshots each member's ID, name, role, instructions, stance, effective connection (with the member's model override) and connection fingerprint. Later team edits never rewrite a running meeting or its history.

Roles in the first contract:

- **Chair:** `pm` if active, read-only eligible ([AI-03](../aiIntegration/AI-03-adapter.md)) and not busy ([DM-01](../domain/DM-01-workspace-team.md)); otherwise `cto`. Selection uses these fixed IDs only, never role-text fallback. If neither qualifies, no meeting is created and the reason is shown. The chair plans and is neither a debate participant nor the evaluator. Changing the chair means closing the meeting and starting another.
- **Participants:** 2–3 distinct members, each with a stance.
- **Evaluator:** one member distinct from the chair and the participants.

The only protocol is `propose_critique_revise` with one round: every participant proposes independently, then participant *i* (in plan order) critiques participant *(i+1) mod n*, then each participant handles every critique it received, then the evaluator assesses once. Required calls are `1 (chair) + 3n + 1 (evaluator)`; the hard cap is 12. Other protocols, more rounds, a chair synthesis call, hash chains and transcript export are later scope.

## Plan

`회의 시작` sends the CEO topic, the team roster with eligibility/busy reasons, the engine caps and the allowed protocol to the chair as one read-only call. That click is the approval for this single call ([Security](../security.md)); it is recorded as a `planning` packet. The chair returns a JSON plan: objective, constraints, participants (agentId, stance), evaluatorAgentId, protocol, rounds, maxCalls, sources (project-relative path, reason) and success criteria.

The engine validates the plan with one backend function and never silently repairs it: JSON shape; 2–3 distinct participants; chair excluded from participants and evaluator; evaluator distinct; every member active, read-only eligible and not busy; protocol and round allowed; `required ≤ maxCalls ≤ 12`; sources inside the project, UTF-8, at most 16,000 characters each and 26,000 in total, frozen with bytes and SHA-256 at validation; the validator measures every proposing packet and conservative worst-case lengths for critique, revision and evaluation before approval, so an oversized plan is fixed before approval rather than blocking after it. The chair request states these limits and asks for the few files the decision needs. An invalid or unreadable chair reply still yields a plan in `plan_review` with its error list, so the CEO can edit it. Every CEO edit creates a new immutable plan revision and digest (SHA-256 of the normalized plan without errors); an edit against an older revision is refused.

## Stages and approval

Meeting stages: `planning → plan_review → (awaiting_transfer_approval) → proposing → critiquing → revising → evaluating → package_ready → decision_recorded`, with `blocked` and `stopped` from active stages and `legacy_closed` for meetings of an older model. [DM-04](../domain/DM-04-lifecycle.md) owns the coarse Goal status (`deliberating`, `awaiting_decision`, `completed`).

The CEO approves an exact plan revision and digest in one card ([ME-02](ME-02-packets.md) owns the approval record and dispatch checks). Approval re-runs validation and the busy check in the same transaction. In `plan` mode each stage's packets are approved by that plan approval and dispatched in order; in `per_stage` mode each stage additionally waits for packet decisions. Rejecting the plan records a `defer` decision and completes the Goal with zero debate calls. Revoking the plan approval stops new calls. A denied packet or changed connection blocks the meeting. A first invalid reply may receive one planned format-repair call; a second invalid reply blocks. Source changes before a proposal require CEO choice, and later source changes are informational. Nothing is fabricated as a contribution.

On restart an in-flight call becomes `outcome_unknown`, the meeting becomes `blocked`, and the plan approval expires. `resume_meeting` revalidates current inputs for the next stage, connections and busy state, returns to `plan_review` for re-approval, and then prepares new packet revisions only for missing contributions; an uncertain predecessor is flagged and never re-sent automatically. A blocked or stopped meeting may instead close as a partial package.

## Contributions and traceability

The engine extracts the first complete JSON object even when surrounded by prose and retains the entire raw reply. The engine assigns IDs (`<agentId>.C1`, `<agentId>.K1`, `R1`) and records equivalent case/zero-padding ID normalization and rejects unknown claim, critique or source IDs, unhandled received critiques, rebuttals without a source reference, and out-of-bound sizes. A quote is `verified` only when it matches the frozen source after CRLF/LF and whitespace normalization; the first occurrence offset and the occurrence count are recorded. Source text is sent only in proposal packets.

The decision package contains engine-built trace nodes (`claim`, `critique`, `response`, `revision`, `assessment`, `recommendation`, `source`) with author member, stage, packet ID and packet SHA-256, linked by `targets`, `responds_to`, `revises`, `cites`, `supports` and `assesses`, navigable in both directions. Unresolved dissent (rebutted, deferred or unanswered critiques) is derived by the engine from the stored responses regardless of the evaluator's list; the evaluator's opinion stays separate. The package also shows recommendations, assumptions, unknowns, alternatives, member/connection/model provenance, usage provenance and the termination reason (`one round completed` or `partial: <reason>`), and is never labelled quality-certified.

**Display summaries ([ADR-0042](../architecture/decisions/ADR-0042.md), [UI/UX 구현 철학](../ux-principles.md)).** Calls that already happen may carry optional plain-language fields: the chair plan reply `topicSummary` (stored as `Meeting.topicSummary`, ≤ 80 characters), each proposal/critique/revision reply `summary` (≤ 120), and the evaluation reply `overview` (≤ 300), `recommendedId` (one existing recommendation ID) and `issues` (≤ 5 items, each `text` ≤ 120 with `critiqueIds` limited to the meeting's critiques). The engine stores them as plain text after collapsing whitespace and removing control characters, cuts overlong text, drops invalid IDs, and never rejects a reply, requests a format repair or spends a call because of them. They are display-only: plan validation, digests, stage progress, dissent derivation, the decision and handoff never read them. The package carries `overview`, `recommendedId` and `issues`; the UI shows dissent the issues do not cover grouped by target claim, and anything missing falls back to excerpts or sentences built from saved counts. The UI opens with the recommended proposal selected, but only the CEO's button records a decision.

## Decision and handoff

`Goal.status=awaiting_decision` persists until the CEO records `adopt` (at least one recommendation), `revise` or `defer`; each completes the Goal and none authorizes execution. Adoption only fills an editable execution-goal draft in the Home composer's `목표 수행` mode. No Goal, Task or Approval is created until the CEO submits it; submission with lineage is refused unless the meeting recorded `adopt` for the same package hash in the same project. File changes then follow the ordinary plan, approval and QA flow. Writing a transcript into the project folder is a separate project-file write approval and is not part of the first contract.

## Relations

## Approved post-approval recovery contract (MAM-014)

Before approval, plan validation checks a conservative upper bound for **every** stage packet using enforced response count and length limits, not only the real proposing packet. The plan card shows per-source and total size and allows removing a source by making a new plan revision. No content is silently truncated to fit a packet.

An invalid model answer remains raw evidence. When the approved plan has spare calls, the engine may ask that same member once to repair the format in that stage, recording the violation and spending one call; a second invalid answer blocks. The default plan includes two spare calls within the 12-call hard cap and shows that allowance. A definitely failed or invalid packet may be manually retried in its stage under the still-approved plan. An `outcome_unknown` packet requires explicit CEO confirmation of possible duplicate transmission, and is never automatically retried. Restart still expires plan approval; after reapproval completed contributions are not resent.

Approved source snapshots remain the transmitted source bytes. An on-disk source change detected before the first proposal asks the CEO to continue with those approved bytes or review a new plan. Once proposals start, later source changes are informational, not a reason to block critique, revision or evaluation. The engine may suggest eligible alternatives when a member is busy but never swaps a plan member automatically.

| Relation | Target | Meaning |
| --- | --- | --- |
| `belongs_to` | [DM-02 Goal](../domain/DM-02-goal.md) | Same project/goal identity and token attribution |
| `requires` | [ME-02 packets](ME-02-packets.md) | Every provider call needs an approved plan and a recorded packet |
| `requires` | [DM-01 Agent busy state](../domain/DM-01-workspace-team.md) | Members are reserved through one shared busy rule |
| `produces` | [GC-03 workspace](../goalConversation/GC-03-workspace.md) | One actionable card and an inspectable decision package |
| `constrained_by` | [Security](../security.md) | Meeting prose never grants transmission or file rights |
