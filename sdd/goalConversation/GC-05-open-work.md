# GC-05 — Open-work quick access

## Open-work quick access

A fixed lower-right launcher is available from every main tab. It counts unfinished goals in the workspace and opens a compact, scrollable list grouped by project and goal. A goal without planned tasks remains selectable; an unfinished task row shows its saved status and assigned teammate. Completed goals and completed tasks do not crowd the list. A blocked goal superseded by a linked replacement leads to that replacement rather than appearing as another live job. Choosing a goal or task from a different project explicitly switches the selected project before opening its workspace; a failed switch leaves the current selection intact and explains the failure.

A deliberation Goal with `deliberating` or `awaiting_decision` remains in open work. A pending transfer decision, stopped meeting, or ready decision package is sorted as an actionable item and opens the same Goal workspace. The launcher never dispatches a meeting call ([ME-01](../meeting/ME-01-process.md)).

Choosing a task opens the existing goal conversation focused on that saved task and its assigned teammate. The teammate's actual configured AI interprets the CEO message via `send_goal_message`; the UI shows the connection/model, route, and pending or failed result. An unavailable teammate is identified by name and connection state, not silently replaced by another role. The CEO may explicitly select another teammate for advice. The question draft names the selected task but remains editable before transmission. A teammate's answer cannot directly mark a task resolved, edit files, or consume approval. An interpreted CEO request can enter only the guarded start/retry or guidance/revision commands. The same workspace exposes the persisted approval/retry/revision actions for actual issue resolution. The launcher itself does not start model calls.

Saved task results or failure reports are shown as evidence. The CEO may explicitly copy a bounded excerpt into the visible, editable question draft; it is never added to the provider payload invisibly. The provider receives that excerpt only when the CEO sends the question. The excerpt remains untrusted data under [Security](../security.md).

The panel, trigger, and toast must not cover each other at common desktop sizes. The trigger reports its open state and count to assistive technology; keyboard activation, Escape close, and narrow-window scrolling work without moving the fixed app bar or sidebar.

## 관계와 소유 경계

| 관계 | 대상 | 의미 |
| --- | --- | --- |
| routes_to | [GC-03 작업공간](GC-03-workspace.md) | 모든 탭에서 같은 목표 대화를 연다. |
| reads | [DM-03 업무·이벤트](../domain/DM-03-execution.md) | 담당자와 저장된 미완료 업무를 목록으로 만든다. |

## 검증 참조

[주제 수용 기준](acceptance.md), [수동 QA](../../qa/MANUAL-QA.md). 구현 증거와 미완료 항목은 [WORKLOG](../../WORKLOG.md)를 따른다.
