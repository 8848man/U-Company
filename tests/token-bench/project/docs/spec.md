# Queue rules

## 1. Priorities

Priorities from most to least urgent: `urgent`, `high`, `normal`, `low`. An unknown priority is treated as `normal`.

## 2. Queue order

`sortQueue(tickets, now)` returns a new array of the tickets whose status is `open` or `pending`, ordered by:

1. overdue tickets first (see §3),
2. then priority (most urgent first),
3. then `createdAt` (oldest first),
4. then `id` (ascending) as a final tie-breaker.

Tickets with status `resolved` are not in the queue.

## 3. SLA

First-response targets, in hours from `createdAt`: `urgent` 1, `high` 4, `normal` 24, `low` 72.

`isOverdue(ticket, now)` is true when the ticket is `open` or `pending`, has no `firstResponseAt`, and `now` is strictly later than `createdAt` plus its target. A resolved ticket is never overdue.

## 4. Summary

`summarize(tickets, now)` returns `{ open, pending, resolved, overdue }` counts. `overdue` counts tickets for which `isOverdue` is true.
