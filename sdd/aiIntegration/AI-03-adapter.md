# AI-03 — Adapter and transport contract

## Extension contract

### Prompt delivery refinement (MAM-014 D1)

The built-in CLI prompt transport has an explicit provider table. Literal argument delivery is the default, including Claude and Codex until each has passed its own synthetic Windows stdin provider-harness check. Verified stdin delivery writes the entire prompt and closes the stream while stdout and stderr drain concurrently. This choice is shared by meeting and execution-goal calls. An argument-mode provider checks quoted whole-command length before process creation; stdin mode uses the packet character budget. Neither mode silently truncates the prompt. Harness checks use synthetic prompts only and never replay stored CEO goals or project documents.

Execution read-only calls and meeting calls share one read-only execution profile check. A connection qualifies only when its provider applies an enforced read-only argument: today `cli` connections whose executable is `claude` (`--permission-mode plan`) or `codex` (`--sandbox read-only`). Gemini, OpenCode, custom CLIs, plugins and HTTP endpoints have no such contract yet and are excluded from meetings with that reason. Meeting calls go through the same provider entry point with `writable=false` and the selected project folder as working directory, as recorded immutable packets below a tested whole-command-line limit; oversized content blocks before a call and is never truncated. The profile name is recorded with each call. The real Windows effect of these flags is an acceptance check, not an assumption.

### Mock AI for token-free runs (ADR-0052)

`scripts/mock-ai/mock-adapter.mjs` is an external adapter speaking protocol v1 over stdio. It recognizes the engine stage from the fixed wording of the engine's prompts and replays a scenario (`scripts/mock-ai/scenarios/`): the CTO summary and `업무:` lines, a PM plan in the task schema, the declared files written into the isolated task folder (never outside it), and per-criterion QA verdicts that check the files exist. Optional failure modes replay recovery paths: `qa-fail-once`, `plan-form-once`, `needs-scope-once`. The v3 `AI 추가` dialog registers it with `add_mock_connection` as a `plugin` connection named `모의 AI (테스트용, 토큰 없음)` with the `files` capability, running the checkout's script with the Node found on `PATH`; like any new connection it fills only teammates without a connection. It is not eligible for meetings (no enforced read-only profile) and never presents itself as a real model. On Windows, adapter processes start without a console window.

### Provider and transport boundary

The engine calls one provider entry point with a prompt, working directory, model selection, and read/write scope. The provider chooses the CLI protocol or HTTP request and normalizes final text, native session ID, and token usage. The transport owns process creation, stdin mode, stdout/stderr draining, timeout, exit status, and termination, or the HTTP request lifecycle. Built-in one-shot CLI calls pass the prompt as a literal argument by default and close empty stdin. Only a provider with a passing synthetic Windows stdin harness may switch to full stdin delivery; the interactive external adapter retains its separate bidirectional protocol. Provider code must not manage OS process handles. A provider failure identifies provider ID, transport kind, operation, failure class, exit code when known, and a bounded, redacted diagnostic. An unsuccessful process result remains a failure even if stderr contains only a progress line; include safe structured error details from stdout when available.

Built-in Codex, Claude, Gemini, and OpenCode CLI providers share the process transport. The OpenAI-compatible endpoint remains an HTTP provider and the versioned external adapter remains a separate protocol over process stdio. The existing `AiConnection` record and engine call signature stay compatible; no new credential storage or UI protocol is required for this slice. Provider selection follows the saved connection identity, never a goal's prose or a character's display name.

Codex CLI invocations permit execution outside a Git repository with its documented `--skip-git-repo-check` option. The project's recorded execution mode, not provider output, determines whether the engine uses a Git worktree or an isolated folder copy. Both modes retain the same project-write approval.

Before a new goal starts or an early blocked goal retries, the CEO card shows the currently assigned analysis teammate and AI connection. It must not imply that changing one developer's connection changes the CTO's provider. Historical invocation records retain the actual connection used at call time.

A standalone developer harness invokes the same provider and transport code without Tauri, SQLite, the organization engine, or project approval state. Its default check is offline and synthetic. Real provider calls require an explicit provider choice and use a separate test folder; the harness reports initialization, process launch, stdin policy, output parsing, exit/timeout, and final response without printing credentials or entire prompts. The real Claude and Codex checks are acceptance gates before claiming those integrations healthy. The harness cannot grant or bypass Co-work project-write approval.

Developer commands from the project root: `npm run provider:test -- offline` tests the local process fixture; `npm run provider:test -- claude <executable-path>` and `npm run provider:test -- codex <executable-path>` make explicit read-only model calls in a dedicated temp Git folder. The equivalent command from `src-tauri` is `cargo run --locked --bin provider-harness -- ...` and works when npm is unavailable. The real commands use a fixed synthetic prompt and do not replay saved CEO goals or project documents. To add another CLI, extend the CLI provider's literal argument and response mapping, then run the offline and a provider-specific harness check. To add a compatible model server, use the HTTP provider and register its endpoint/model without changing process I/O. A different HTTP response schema needs its own provider implementation. An interactive process must first demonstrate a real protocol need and receive a separate design decision.

Every adapter exposes `discover`, `list_models`, `health`, `capabilities`, `start_session`, `send_task`, `stream_events`, `approve`, `cancel`, and `resume` where supported. Unsupported methods return an explicit capability error. Events carry adapter ID, native run/session ID, timestamp, goal/task/agent references, kind, payload, and error when present. Native events are preserved for debugging while a normalized event drives the engine and UI.

Built-in adapters use documented runtime interfaces. A user can add a custom CLI with executable path, argument vector and documented JSON/JSONL mapping; an OpenAI-compatible HTTP endpoint with base URL and model ID; or an external adapter process speaking the versioned JSON-RPC-over-stdio contract. Never interpolate task content into a shell command string. A new executable or remote endpoint requires an explicit connection action; the UI tests and records its capabilities before assignment. Task assignment ([DM-03](../domain/DM-03-execution.md)) reads only saved connection facts: a connection is usable when its status is not `missing`, `failed` or `needs_setup`, and it can take file-writing work when it is a built-in or custom CLI (which runs in the isolated workspace) or an external adapter whose recorded capabilities include `files`. An HTTP text endpoint never receives file-writing work. A role name or a model preference alone is not a capability.

The adapter interface is versioned. Unknown fields are ignored for forward compatibility; unsupported major versions are rejected with a clear error. A third-party adapter runs with the app's user privileges, so registration presents its executable, requested workspace/network scope, and trust warning before first launch.

## 관계와 소유 경계

| 관계 | 대상 | 의미 |
| --- | --- | --- |
| invokes | [AI-01 모델·런타임](AI-01-model-runtime.md) | 저장된 연결 정체성으로 호출한다. |
| emits | [AI-04 진행·토큰](AI-04-telemetry.md) | 정규화한 결과와 확인된 런타임 이벤트를 남긴다. |
| constrains | [보안](../security.md) | 공급자 출력으로 승인·셸 권한을 확대하지 않는다. |

## 검증 참조

[검증 기준](../validation.md), [수동 QA](../../qa/MANUAL-QA.md). 구현 증거와 미완료 항목은 [WORKLOG](../../WORKLOG.md)를 따른다.
