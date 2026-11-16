# Architecture notes

helpdesk-lite started as a weekend script and stays small on purpose.

- Storage is an in-memory array behind `createStore()`. A file-backed store was discussed and postponed.
- Formatting is separate from rules so the console view can change without touching the queue logic.
- There is no web server. A future HTTP layer would call the same pure functions.

## History

- 0.1: store and console formatting.
- 0.2: ticket ids became zero-padded strings (`T-0001`).
- 0.3: queue rules moved to `src/tickets.js` and are being specified in `docs/spec.md`.
