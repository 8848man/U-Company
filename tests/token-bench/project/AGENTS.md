# Working agreement

- Plain ES modules, no dependencies. Keep functions pure; never mutate the arrays you are given.
- Time is passed in as `now` (a `Date`); never call `Date.now()` inside rules.
- Every rule in `src/tickets.js` has tests in `test/tickets.test.js` using `node:test` and `node:assert/strict`.
