# External SDD import acceptance criteria

## Acceptance

- Previewing malformed or oversized input reports a reason and writes no active profile.
- Two projects can import different role/workflow specifications without changing prompts or the global team records.
- An `AGENTS.md` or custom harness document becomes a visible, source-attributed exception and is not rewritten or run.
- A stale draft cannot overwrite a newer applied version; apply is atomic and survives restart.
- Import text containing shell commands or approval instructions remains data and cannot change the engine's approval or file boundaries.
- A project with a portable file can be edited by another tool; refresh detects drift and preview/apply updates only that project's local projection.
- Initialization creates the file once and refuses replacement; malformed, oversized, or symlinked files are not silently used.
- An unrelated uploaded draft and a draft made before a file edit cannot be applied to SQLite. New imports create the project file first and are then previewed from that path.
