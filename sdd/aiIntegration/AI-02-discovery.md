# AI-02 — Discovery, health, and login

## Discovery

At app start and on request, check known executables on PATH and known loopback endpoints with bounded timeouts. Initial probes: Codex, Claude Code, Gemini CLI, OpenCode, Ollama, and LM Studio. Existence is distinct from authentication and a successful health check. Never search the entire disk or read credentials during discovery. Missing, needs setup, ready, and failed statuses are distinct. The UI separates confirmed local executable or model-server results from supported-tool presets that were not found. Each confirmed result shows its actual path or endpoint, probe result, and readiness reason. Registration records a real connection; it never stores a missing preset as if installed. A failed registration reports the exact failed condition and a next action.

On Windows, executable resolution prefers runnable `.exe`, `.cmd`, and `.bat` entries over extensionless POSIX shell shims (for example npm's `claude` next to `claude.cmd`). A failed launch does not prove the AI itself is broken; show the selected path and a plain-language probe result. If an already registered built-in AI has a different path from automatic discovery, the UI identifies the difference and requires an explicit path-change confirmation instead of silently replacing the saved executable. Registering a connection assigns it to currently unassigned members in one persisted operation, without changing deliberate existing assignments. A provider model catalog is queried when available. The UI offers only discovered IDs plus a provider-default choice; for an HTTP endpoint that requires a model, registration waits for a successful `/models` response and a selection.

Known CLI connections saved with an extensionless Windows npm shim are repaired to a verified sibling wrapper during registration or app startup. Custom connection paths remain user controlled. Because a live frontend can refresh while the Rust process is older, the frontend checks a backend protocol revision and explains that a restart is required before offering new connection actions.

Windows npm batch wrappers must not receive natural-language goals as command-line arguments: they can reject complex text or interpret shell metacharacters. For supported built-in CLIs, resolve a verified native executable or Node entry point and pass the goal directly to that process. If safe resolution is unavailable, report an actionable unsupported-wrapper error before starting work. Claude readiness comes from its `auth status` JSON `loggedIn` field, not from a successful `--version` probe. The app checks the exact executable stored on a registered Claude connection, even when automatic PATH discovery finds a different path. A false `loggedIn` value is shown as the CLI's report for that executable and app process; an unparseable or timed-out check is shown as unverified, not as proof of logout. Rechecking a registered AI refreshes its saved state and shows the checked path, result, and next action without revealing identity fields or credentials.

For a registered built-in CLI, AI Connections offers a user-initiated login action. It starts the saved executable in a visible terminal with a fixed provider command: Claude `auth login`, Codex `login`, OpenCode `auth login`, or Gemini's interactive start. The app never supplies credentials, injects `/login` into a running session, or marks the connection ready merely because a login window opened. The user finishes the provider's browser/terminal flow and explicitly rechecks the same saved path. Missing or custom providers show instructions instead of running an inferred command. On Windows a new PowerShell console executes a safely quoted literal executable path and fixed arguments; Ubuntu uses an installed terminal emulator with an argument vector. Failure to open a terminal is reported without changing connection state.

Connection and runtime failures use fixed provider-aware recovery guidance. Authentication, executable/launcher failure, network failure, rate/quota exhaustion, and missing/unsupported model are distinct. A diagnostic may inform classification, but untrusted provider text never supplies an executable, argument, URL, credential, or approval decision. Unknown errors retain a bounded diagnostic and suggest an explicit recheck. Recovery guidance is visible next to saved connections and blocked goals where a provider can be identified.

Known provider details have narrower guidance: Claude's `organization disabled` can indicate an overriding `ANTHROPIC_API_KEY`; Gemini organization entitlement errors can depend on `GOOGLE_CLOUD_PROJECT`; OpenCode's missing provider requires provider configuration. These checks are informational only and never alter environment variables or credentials.

## 관계와 소유 경계

| 관계 | 대상 | 의미 |
| --- | --- | --- |
| selects | [AI-01 모델·런타임](AI-01-model-runtime.md) | 발견·인증 상태와 능력을 구분해 배정한다. |
| constrains | [보안](../security.md) | 저장된 경로의 고정 로그인 동작만 사용자 클릭으로 시작한다. |

## 검증 참조

[검증 기준](../validation.md), [수동 QA](../../qa/MANUAL-QA.md). 구현 증거와 미완료 항목은 [WORKLOG](../../WORKLOG.md)를 따른다.
