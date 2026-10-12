# Provider/runtime investigation — 2026-10-04

This is observed diagnostic evidence, not the integration contract. The intended boundary is in [Integrations](aiIntegration/00_index.md).

## Actual failing path

`src/App.tsx` calls `run_goal` → `src-tauri/src/lib.rs` calls `engine::start` → `engine::run` chooses the enabled CTO connection → `execute_tracked` calls `runtime::execute_with_observer` → `run_cli` builds the Codex/Claude CLI arguments → `capture_with_timeout` starts the process. The selected goal's saved invocation ledger identifies `agentId=cto`, `connectionId=codex`, `connectionName=Codex CLI`, twice with `status=failed`. The goal stopped in `analyzing`. The current CTO assignment in the app's default SQLite is Codex. A registered Claude connection exists but is not the connection used for this failure.

The actual command path for that saved Codex connection is the npm `codex.cmd`, safely resolved by `discovery::runtime_command` to Node plus `@openai/codex/bin/codex.js`. `run_cli` passes `exec --json --sandbox read-only --cd <project> <prompt>` as literal arguments. `capture_with_timeout` sets stdin to `Stdio::null()`, drains stdout line by line and stderr concurrently, waits with a 30-minute deadline, and turns a nonzero status into `Runtime exited ...` with up to 1000 stderr characters. Therefore the `Reading additional input from stdin...` text came from the Codex child process's stderr; Co-work added the `Runtime exited exit code: 1:` prefix. This is not evidence of a failed Claude login.

## Reproduction boundary

The locally installed Claude Code CLI is 2.1.289; `auth status` reported `loggedIn=true`. The current app uses its npm-native `claude.exe` for the saved `claude.cmd`. A synthetic read-only Claude `-p` call with closed stdin, stream JSON, verbose mode, and plan permission exited 0 and returned a result. The local Codex CLI is 0.159.3. Synthetic read-only Codex calls with closed stdin in isolated Git folders exited 0, including an extended Windows path and a Korean folder. One successful Codex run also printed the stdin notice on stderr, so that notice alone is not a failure criterion. The failed app invocation's full stdout was discarded when the process returned 1, so its additional structured error and exact cause were not preserved.

After the change, the shared process transport creates and immediately closes an empty stdin pipe instead of attaching the null device. The standalone Rust harness then returned successful synthetic responses from both installed CLIs. This verifies the new EOF lifecycle but does not retroactively identify the historical failure's additional cause.

The automatic approval reviewer rejected replaying the saved CEO goal, role instructions, and project documents to the external Codex service because that exact data transmission lacked approval. Do not work around the rejection. The synthetic checks did not transmit those documents. The exact condition behind the two historical exit-code-1 runs remains unverified; process I/O, child failure, and permission/auth context must be distinguished by the new harness and bounded diagnostics. Reports in the [Codex issue tracker](https://github.com/openai/codex/issues/20919) describe non-TTY stdin behavior, but they do not establish that behavior as the cause of these two local exits.

## Change scope

Move CLI argument and result parsing into provider code; keep process I/O/timeout in the common transport. Preserve the current connection schema and engine call contract. Build an independent harness for synthetic and explicit real-provider checks. On failure, retain safe JSONL error metadata as well as bounded stderr and identify the provider/transport/operation. Interactive/PTY execution is not supported by this evidence and is not introduced.

No user decision is needed to retain the existing CLI connections, close stdin explicitly, or add this isolated diagnostic boundary. A switch to Claude API/SDK, new credential storage, or a PTY would need a separate decision. The user's historical goal was not replayed because its exact transmission was rejected by automatic approval review.
