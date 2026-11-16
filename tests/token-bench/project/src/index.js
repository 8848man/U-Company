import { createStore } from './store.js';
import { formatList } from './format.js';

const now = new Date('2026-01-05T09:00:00Z');
const store = createStore();
store.add({ title: 'Login page is blank', priority: 'urgent' }, new Date('2026-01-05T07:30:00Z'));
store.add({ title: 'Invoice PDF typo', priority: 'low' }, new Date('2026-01-02T10:00:00Z'));
store.add({ title: 'Export to CSV', priority: 'normal' }, new Date('2026-01-04T12:00:00Z'));

console.log(`Queue at ${now.toISOString()}`);
console.log(formatList(store.all()));
