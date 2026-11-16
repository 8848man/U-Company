import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createStore } from '../src/store.js';

test('add assigns padded ids and defaults', () => {
  const store = createStore();
  const at = new Date('2026-01-01T00:00:00Z');
  const ticket = store.add({ title: 'a' }, at);
  assert.equal(ticket.id, 'T-0001');
  assert.equal(ticket.priority, 'normal');
  assert.equal(ticket.status, 'open');
  assert.equal(ticket.firstResponseAt, null);
  assert.equal(ticket.createdAt, at);
});

test('update returns a copy and leaves earlier snapshots untouched', () => {
  const store = createStore();
  store.add({ title: 'a' }, new Date());
  const before = store.all();
  store.update('T-0001', { status: 'resolved' });
  assert.equal(before[0].status, 'open');
  assert.equal(store.all()[0].status, 'resolved');
});
