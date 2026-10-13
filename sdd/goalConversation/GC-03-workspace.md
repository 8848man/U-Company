# GC-03 — Shared conversation workspace

## Product purpose

The product's advantage is an inspectable AI organization: one CEO goal has an accountable team, a traceable plan, scoped decisions, live work evidence, and a goal-bound conversation. Every interface should answer **what is understood, who owns the next step, what changed, what needs my decision, and what proof supports completion**. A character or status animation is a view of saved facts, never a substitute for those facts.

## Interaction policy

For a deliberation Goal the same workspace opens from Home, Work, Office and open work. It shows one actionable meeting card at a time in the order exact transfer decision, stopped-meeting recovery, decision package. The conversation composer and execution-goal direction routing are hidden in the first meeting slice. The saved meeting record shows proposal → critique → revision → evaluation → CEO decision with source and dissent links; development QA cap, file tasks and CTO/PM graph are absent. Detailed stage, packet, participant, provider and reported/estimated usage history stays in progress disclosure. [ME-01](../meeting/ME-01-process.md) owns the meeting process and [Security](../security.md) owns approval.

The primary surface shows the saved goal, active owner, and one next action. The same goal workspace opens from CEO, Office, and board. Its first view contains the conversation, a short stage/owner/next-action line, and any critical decision with its exact approval scope or failure reason. The latest safe operational event, task assignees and statuses, QA count, and goal-specific token total with reported/estimated provenance are available in one progress disclosure. Use the [information-density policy](../content-density.md) for logs and turns; do not repeat full task/history lists across all tabs. A compact Office goal rail is specified in [Office goal visibility plan](../office-goal-visibility-plan.md); it navigates to the same workspace. A blocked control remains clickable and explains its reason and recovery step.

## Conversation-first workspace

The goal workspace opens as a compact conversation surface. Above the conversation, show the saved goal in a short title and one line of stage, current owner, and next action. The main area shows chronological CEO and actual teammate turns, with sender, status, time, and provider/model identity. When no turns exist, show a short deterministic welcome and editable example questions so a new user knows what to ask; do not imply a simulated answer is an AI response.

The conversation composer identifies the team member who interprets and receives the CEO message. A saved guidance status shows that target and the later CTO → PM review route. After planning, a linked revised goal shows which prior tasks were reused and which new tasks remain; `reuse proposed` and `reuse verified` must be visually distinct. The app never claims a model received a queued instruction before its invocation.

Show only the decision that currently needs attention as one in-conversation action card: exact pending approval scope, safe retry/resume, or version review. A blocked reason remains visible with full-text expansion. A focused task appears as a concise context message with its saved assignee and optional explicit evidence-to-draft action. The active composer stays reachable at the bottom with one natural-language draft and one send action. A read-only AI invocation interprets the CEO message against saved goal state as discussion, an actionable goal direction, guarded start/retry, or ambiguity. Discussion gets a teammate reply; ambiguity gets a clarifying reply without a state change. Guarded start/retry calls only the existing state-validating engine command. An actionable direction saves the CEO exact text as guidance, waits for the engine safe checkpoint, creates a linked revision using the previous intent, exact CEO instruction, and a bounded prior conversation excerpt marked as non-authoritative context, and starts CTO/PM re-planning automatically. This message itself authorizes goal routing, but neither AI interpretation nor message receipt approves file writes, changes projects, invokes arbitrary commands, or bypasses engine state checks. New file work still requires scoped CEO approval.

Render saved questions and CEO direction changes in one chronological conversation stream. A direction change appears as a CEO message and a separate state message (`접수/안전 지점 대기`, `새 목표 검토 필요`, or `새 버전에서 진행`), linked to the worker escalation and continuation record when available. The current action card may duplicate the next button but not the full command text or conversation. The helper copy must say that **questions** are read-only; it must not label the whole conversation read-only. A pending AI interpretation remains visible. Subsequent messages wait until that interpretation completes so two directions cannot be applied out of order.

Move the phase graph, full task list, QA evidence, token usage, and operational event detail behind one `진행 정보` disclosure. The summary retains the key counts and provenance; opening it reveals saved facts without a new AI call. Default conversation history follows the five-item density rule with a clear older-turn expansion. Critical pending approval and recovery actions stay visible. The dialog scrolls its conversation content while header and composer remain usable in a narrow window and with keyboard navigation.

## v3 대화 한 줄기 (ADR-0046)

v3에서는 실행 목표 하나가 대화 하나다([ADR-0046](../architecture/decisions/ADR-0046.md)). 위의 규칙은 그대로 두고, 표시 위치만 바꾼다.

- **결정 카드는 대화 안에 둔다.** 계획 카드(요약, 작업 수·담당·파일 수, 작업 구조 질문, 통제 한 줄, `승인`·`개선`·`거부`)와 복구 카드(`지금 할 일`)는 대화 흐름의 해당 시점에 놓인다. 지금 결정이 필요한 카드 하나만 버튼이 살아 있고, 끝난 카드는 한 줄 요약으로 접힌다. 승인 범위 전문, 산출물별 기준, 팀 구성과 바꾸기는 카드의 펼침 안에서 그대로 확인한다. v3에는 승인 모달이 없다.
- **단계가 바뀔 때만 한 줄을 남긴다.** "계획이 나왔어요", "작업 3개를 시작했어요", "검토를 통과했어요"처럼 저장된 단계 전환마다 한 줄. CTO 분석과 PM 계획 원문은 대화에 올리지 않고 `진행 정보`와 카드 펼침에 둔다.
- **진행 줄.** 입력칸 바로 위에 한 줄이 떠 있고 제자리에서 바뀐다. 문구 규칙은 [진행 표시 정책](../progress-visibility.md)의 "대화 진행 줄"을 따른다. 누르면 Office 창이 열린다.
- **개선 입력.** `개선`을 누르면 입력칸이 "어떻게 고칠까요?"로 바뀌고, 다음 메시지가 개선 방침으로 저장된다. 보낸 방침은 CEO 메시지로 대화에 남는다. 취소하면 일반 대화로 돌아간다.
- **보내면 시작.** 새 목표를 보내면 바로 팀 실행을 시작한다. 실행할 수 없으면 대화에 이유와 해결 버튼을 보인다.

## 관계와 소유 경계

| 관계 | 대상 | 의미 |
| --- | --- | --- |
| reads | [GC-01 대화 턴](GC-01-turns.md) | 실제 대화와 상태를 표시한다. |
| reads | [GC-02 지침](GC-02-guidance.md) | 접수·체크포인트·새 목표 버전을 표시한다. |
| reads | [GC-04 복구](GC-04-recovery.md) | 현재 허용되는 다음 행동을 표시한다. |

## 검증 참조

[주제 수용 기준](acceptance.md), [수동 QA](../../qa/MANUAL-QA.md). 구현 증거와 미완료 항목은 [WORKLOG](../../WORKLOG.md)를 따른다.
