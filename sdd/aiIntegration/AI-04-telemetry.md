# AI-04 — Runtime progress and token records

## Observable progress

The CLI runtime consumes stdout incrementally and maps supported structured events to short operational summaries. Persist at most a bounded number per runtime call and emit state changes for the UI. Summaries identify the agent, goal, task, and milestone; omit raw prompts, model reasoning, tool inputs, command strings, credentials, and arbitrary stdout. Providers without structured progress show stage and elapsed-time state; the UI does not fabricate runtime logs. Office's idle dialogue is labeled as illustrative and is distinct from the real event feed.

## Token accounting

For a meeting invocation, the record keeps the same project and Goal IDs but uses a namespaced Meeting slot ID as agent ID; the UI resolves its snapshot name, connection and model instead of presenting it as an Office teammate. A stopped or uncertain invocation keeps a conservative estimated count. Forced call/round limits and per-call timeout are separate from estimated cumulative usage that can only block the next call. No count is a subscription quota or billing hard cap ([ME-01](../meeting/ME-01-process.md)).

Each runtime invocation creates a project/goal/agent-scoped usage record that snapshots its connection ID and effective model ID at invocation time. Historical records without these fields show an unknown model, rather than inheriting a teammate's current assignment. Parse Codex `turn.completed.usage` and Claude final `result.usage` when present. If a provider does not supply counts, estimate input from prompt length and output from observed assistant text length at approximately four characters per token; mark the record `estimated`. Codex cached input is already part of its reported input count and must not be added twice. Claude cache creation/read fields are added to its base input count. A reported final count replaces the invocation's estimate rather than adding to it. Persist updates at most every five seconds during a run (and once at completion), with UI refresh coalescing. Aggregate records from SQLite; do not make extra model calls to obtain usage. Display counts as usage estimates, never account quota or monetary cost.

## 관계와 소유 경계

| 관계 | 대상 | 의미 |
| --- | --- | --- |
| reads | [AI-03 호출 결과](AI-03-adapter.md) | 실제 응답·사용량 근거를 보존하고 추정은 표시한다. |
| constrains | [진행 표시 정책](../progress-visibility.md) | 로그는 사적 추론이나 미확인 진행을 꾸며내지 않는다. |

## 검증 참조

[검증 기준](../validation.md), [수동 QA](../../qa/MANUAL-QA.md). 구현 증거와 미완료 항목은 [WORKLOG](../../WORKLOG.md)를 따른다.
