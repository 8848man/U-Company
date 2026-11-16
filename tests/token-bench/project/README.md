# helpdesk-lite

A tiny support-ticket queue for a small team. No dependencies; Node 20+.

- `src/store.js` keeps tickets in memory and defines the ticket shape.
- `src/format.js` renders tickets for the console.
- `src/tickets.js` holds queue rules (priority order, SLA). See `docs/spec.md`.
- `src/index.js` is a small demo.

Run the tests with `npm test` (`node --test`).
