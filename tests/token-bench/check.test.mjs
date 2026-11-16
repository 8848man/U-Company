// Hidden acceptance check for the token measurement. The model never sees this file; the bench runs
// it against the finished project (BENCH_PROJECT) so "passed" does not depend on a QA model.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';
import { join } from 'node:path';

const project = process.env.BENCH_PROJECT;
if (!project) throw new Error('BENCH_PROJECT is not set');
const { sortQueue, isOverdue, summarize } = await import(pathToFileURL(join(project, 'src', 'tickets.js')).href);

const h = hours => new Date(Date.UTC(2026, 0, 5, 0, 0, 0) + hours * 3600_000);
const t = (id, priority, status, createdHours, firstResponse = null) => ({ id, title: id, priority, status, createdAt: h(createdHours), firstResponseAt: firstResponse === null ? null : h(firstResponse) });

test('SLA boundary is strict and per priority', () => {
  assert.equal(isOverdue(t('T-1', 'urgent', 'open', 0), h(1)), false);
  assert.equal(isOverdue(t('T-1', 'urgent', 'open', 0), h(1.001)), true);
  assert.equal(isOverdue(t('T-2', 'high', 'pending', 0), h(4.5)), true);
  assert.equal(isOverdue(t('T-3', 'normal', 'open', 0), h(24)), false);
  assert.equal(isOverdue(t('T-4', 'low', 'open', 0), h(72.5)), true);
  assert.equal(isOverdue(t('T-5', 'mystery', 'open', 0), h(25)), true, 'unknown priority is normal');
});

test('resolved or answered tickets are never overdue', () => {
  assert.equal(isOverdue(t('T-1', 'urgent', 'resolved', 0), h(100)), false);
  assert.equal(isOverdue(t('T-2', 'urgent', 'open', 0, 0.5), h(100)), false);
});

test('queue order: overdue, priority, createdAt, id; resolved excluded; input untouched', () => {
  const now = h(30);
  const tickets = [
    t('T-9', 'low', 'open', 29),
    t('T-3', 'normal', 'open', 2),      // overdue (28h > 24h)
    t('T-2', 'urgent', 'open', 29.5),
    t('T-8', 'urgent', 'open', 29.5),   // same createdAt as T-2 -> id order
    t('T-1', 'high', 'resolved', 0),
    t('T-4', 'high', 'pending', 28),
  ];
  const copy = structuredClone(tickets);
  assert.deepEqual(sortQueue(tickets, now).map(item => item.id), ['T-3', 'T-2', 'T-8', 'T-4', 'T-9']);
  assert.deepEqual(tickets, copy, 'input not mutated');
  assert.notEqual(sortQueue(tickets, now), tickets);
});

test('summary counts', () => {
  const now = h(30);
  const tickets = [t('A', 'normal', 'open', 2), t('B', 'urgent', 'pending', 29.5), t('C', 'low', 'resolved', 0)];
  assert.deepEqual({ ...summarize(tickets, now) }, { open: 1, pending: 1, resolved: 1, overdue: 1 });
});
