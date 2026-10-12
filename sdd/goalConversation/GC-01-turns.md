# GC-01 — Goal-bound turns and teammate replies

## One goal, one conversation

Each goal owns a persisted conversation. The CEO can open it from the CEO goal card, Office goal rail, or board goal card without changing projects or losing the goal context. Turns identify sender, selected teammate, time, saved route, request ID, and status. The route is discussion, clarification, direction, or guarded continuation; older turns may have no route field.

Only one AI-interpreted turn may be pending for a goal. The pending-turn check and insertion are one SQLite transaction, so simultaneous submissions cannot launch two indistinguishable calls. The selected teammate, connection, and model are captured at submission and remain the turn's identity if settings change later.

- **Understanding check:** ask the configured CTO/analysis agent to restate the outcome, constraints, open questions, risks, and proposed next step. This is a real read-only AI call, not a template or simulated dialogue. Show the agent and model actually used and the result's provenance. If no capable connection is available, show why and how to fix it. The answer is advice, not a change to the saved goal or plan.
- **Discussion:** ask a goal-scoped question of an enabled team role with a configured text-capable connection. Default to CTO when available and let the CEO choose a different role before sending. Show the selected teammate and actual connection/model, saved goal, current stage, relevant plan, and bounded recent turns. Discussion cannot execute file edits or grant approval. A missing/failed connection is explained before submission; untrusted model output is displayed as data.
- **Guidance/pivot:** propose a change to outcome, priority, scope, or acceptance criteria. Save the CEO's exact text and show `대기 중`, `검토 필요`, or `새 목표 버전 생성` with the checkpoint and reason. Acknowledging receipt is not claiming an active invocation read it. At the first safe checkpoint, pause the old goal. For an AI-routed instruction, the engine creates a linked intent from the previous goal and exact CEO message, then starts CTO analysis and PM planning automatically. A legacy manually submitted guidance proposal retains its explicit revision review. New file work requests a fresh approval; a verified no-write continuation proceeds to QA. The old goal, its tasks, and its approval remain attributable. The UI never silently edits the original intent.

For an AI-routed direction change, the CEO addresses a team member in the same conversation. Save the addressee as routing provenance, not as proof that a model has read the message. After the safe checkpoint and linked revision starts, the selected connected member gives a real read-only escalation memo to CTO; if that runtime is unavailable, the revised goal blocks with a repair reason instead of claiming delivery. A direction addressed to CTO proceeds directly to CTO analysis. CTO analyzes the changed requirements against the old goal and saved task results; PM proposes only remaining file work and may nominate completed tasks for reuse. The engine independently verifies each nomination against an integration-time file digest and the current project files. A task without complete evidence, an unintegrated worktree, an invalid dependency, or changed bytes is not skipped. Record the accepted reuse decision in the goal lineage; QA inspects the whole revised outcome, including reused artifacts. Fresh file writes require a new explicit approval; an old approval never authorizes a revised goal. A revision with no remaining file work still requires QA but not project-write approval.

The default goal card offers `대화 및 방향 변경` regardless of stage, including blocked and completed goals. Completed goals support discussion; an execution request on a completed goal explains that the CEO must create a follow-up goal. Goal conversation is separate from the illustrative idle character dialogue.

## Teammate reply style

Goal conversation answers should read like a teammate replying in chat. Use the CEO's language, answer the actual question first, and keep ordinary replies to a few short paragraphs. Include a clarifying question or one next action only when it helps the CEO continue. Use headings, tables, detailed checklists, and long evidence lists when the CEO explicitly asks for a draft, plan, audit, or other detailed artifact. Do not prepend a repeated role/read-only disclaimer, reproduce the same context as a report template, or mention unrelated connector status. The single message action and saved connection identity explain those boundaries without consuming every answer.

Concise style must not weaken truthfulness or the security boundary: distinguish an observed fact from an inference when it matters, do not claim a file or command was checked unless it was, and do not imply that a chat response approved, reassigned, resumed, or edited work. Preserve the provider's complete answer in the saved turn and allow expansion of long messages; do not rewrite past turns or summarize them with another AI call.

## 관계와 소유 경계

| 관계 | 대상 | 의미 |
| --- | --- | --- |
| produces | [GC-02 지침](GC-02-guidance.md) | 분류된 명확한 CEO 지침만 체크포인트로 전달한다. |
| reads | [GC-03 작업공간](GC-03-workspace.md) | 저장된 목표 맥락을 대화에 사용한다. |
| constrains | [보안](../security.md) | AI 답변 자체는 승인이나 파일 작업 권한이 아니다. |

## 검증 참조

[주제 수용 기준](acceptance.md), [수동 QA](../../qa/MANUAL-QA.md). 구현 증거와 미완료 항목은 [WORKLOG](../../WORKLOG.md)를 따른다.
