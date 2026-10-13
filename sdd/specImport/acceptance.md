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
- Detecting the SDD origin (SI-03) reads files only: registration, detection and plan preview leave every project file byte-identical, make no AI call and send nothing.
- Each fixture is classified as its harness manifest expects (`tests/sdd-origin`): empty folder and empty `AGENTS.md` → bootstrap; guidance with a spec workflow, other-tool rules or an external SDD folder → existing; rules only in a subfolder → existing for that subfolder with no root action; guidance without a workflow or with only a title → ask; `.ucompany/sdd/` → continue. With `AGENTS.md` and `CLAUDE.md` both present, `AGENTS.md` is named as the rule followed.
- `bootstrap`, `add_section` and `convert` refuse without `confirmed = true`, with a stale digest, for a disallowed mode, when `.ucompany/sdd/` already exists, through a symbolic link, or outside the canonical root. A refused action changes no file.
- `add_section` keeps the existing `AGENTS.md` bytes as an exact prefix. `convert` writes a byte-identical backup of every root guidance file under `.ucompany/legacy/` before any change, keeps the original `AGENTS.md` text outside the managed block, and leaves other-tool rules and external SDD folders unchanged. Rerunning only replaces the managed block.
- After `bootstrap`, the offline judge (`python judge.py check <repo> --mode bootstrap`) passes structure, entry point, Claude bridge and graph integrity.
