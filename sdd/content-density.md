# 정보 밀도 UI 정책

Status: Accepted (2026-10-02); long-text rule amended 2026-10-11 by [ADR-0042](architecture/decisions/ADR-0042.md). Applies to the CEO desk, board, Office detail panels, project/AI lists, token usage, and later screens. This policy applies the [UI/UX 구현 철학](ux-principles.md) (UXP-1, UXP-2) to lists and long text.

## Repeated content

When a screen repeats similar records and its owning spec does not set another count, show at most **five** by default. Preserve the full data in memory/storage. Show the total and hidden count, plus a clear `모두 보기` / `접기` action. Restoring the collapsed state shows the same first five. Order by urgency when action is needed and otherwise by newest or the existing explicit sort. A user-selected sort operates on the full set before the five-item window, not only the visible subset. This is a display rule, not a data retention limit.

Exceptions require a user-facing reason: global navigation, choice menus, and the Office Canvas are controls/spatial views rather than repeated content; pending approvals and other time-critical actions stay fully visible so the CEO cannot miss a decision. Empty and at-most-five lists need no disclosure control. A focused search/filter may define its own count when it says so. Nested lists use the same five-item default independently.

## Tall content blocks

Read-only prose or logs inside a card get a compact preview when their rendered content exceeds **160 CSS px** at the current width. The preview uses a fixed maximum height and an explicit `전체 보기` / `접기` control; expanding reveals the original text unchanged. Where the owning spec provides a summary under the [summary rules](ux-principles.md#요약-규칙), the summary may be shown first instead of the height preview, labelled `AI 요약` or `원문 일부`, with `원문 보기` in the same place. A summary never replaces or rewrites the stored original. Re-measure on content and width changes. Avoid clipped focusable controls. Long titles use a short deterministic label only when the full text is recoverable by expansion in the same card. Do not truncate editable fields.

Exceptions: security or approval scope in its confirmation dialog stays fully inspectable; file paths needed to identify the selected project remain accessible in full via title or expansion; a dedicated report/code viewer may use its own labeled scroll region. An exception never silently drops the source text.

## Current component audit

| Surface | Repeated set | Treatment |
| --- | --- | --- |
| CEO desk | Goal cards | Five by default, ordered with the new-goal notice and pending decisions first, then newest; existing full-goal expansion remains. |
| Board | Per-goal graphs, delegated task cards, activity events | Five each by default; pending approvals remain all visible. Long task instructions/results use height preview. |
| Office | Roster, hidden teammate list, operational activity events | Five each by default; Canvas shows all enabled characters. Long activity descriptions use height preview. |
| Projects / AI connections | Project cards, confirmed AI, missing presets, saved AI | Five each by default. |
| Token usage | Goals, teammates, model rows | Five at each repeated level after the selected sort; detail remains available. |
| Goal conversation workspace | Saved question/answer exchanges, CEO instructions, task rows | Show five newest saved conversation items by default in chronological display order, and five tasks in the single progress disclosure; keep critical action cards visible. |
| 업무 tab (v2/v3) | Goal and meeting titles, pending decision, task rows | Titles are the display summary (`AI 요약`/`원문 일부`) with `원문 보기`; the pending decision shows a plain sentence and `요청 원문 보기`; task rows show the deliverable and plain status with `AI에게 준 지시 보기` / `결과 원문 보기`. |
| 설정 › 백업 삭제 (ADR-0043) | Archived goals and meetings | Settings shows one card with the count; the list opens only in a dialog with five rows (summary title, kind, date, size, actions) and `모두 보기`. No goal content is shown; permanent delete expands an inline confirmation. |
| Office Meeting dialog | Proposals, stage lines, issues, records | Three depths (ADR-0042): at most five proposals with one recommended; stage lines and grouped issues on 요약; every original under 자세히 / 모든 기록. |

The limit is a frontend projection over existing snapshot records. It must not change engine progress, approval policy, saved task/event counts, or token totals.
