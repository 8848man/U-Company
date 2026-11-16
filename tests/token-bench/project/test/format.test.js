import { test } from 'node:test';
import assert from 'node:assert/strict';
import { formatList, formatTicket } from '../src/format.js';

test('formatTicket shows the priority tag and status', () => {
  assert.equal(formatTicket({ id: 'T-0001', title: 'x', priority: 'urgent', status: 'open' }), '[!!] T-0001 x (open)');
  assert.equal(formatTicket({ id: 'T-0002', title: 'y', priority: 'odd', status: 'pending' }), '[  ] T-0002 y (pending)');
});

test('formatList handles an empty list', () => {
  assert.equal(formatList([]), '(empty)');
});
