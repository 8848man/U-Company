/**
 * Ticket shape:
 * {
 *   id: string,               // "T-0001"
 *   title: string,
 *   priority: 'urgent' | 'high' | 'normal' | 'low',
 *   status: 'open' | 'pending' | 'resolved',
 *   createdAt: Date,
 *   firstResponseAt: Date | null,
 * }
 */

export function createStore(initial = []) {
  let tickets = initial.map(ticket => ({ ...ticket }));
  let next = tickets.length + 1;
  return {
    all: () => tickets.map(ticket => ({ ...ticket })),
    add(fields, now) {
      const ticket = { id: `T-${String(next++).padStart(4, '0')}`, priority: 'normal', status: 'open', firstResponseAt: null, ...fields, createdAt: now };
      tickets = [...tickets, ticket];
      return { ...ticket };
    },
    update(id, changes) {
      const index = tickets.findIndex(ticket => ticket.id === id);
      if (index < 0) throw new Error(`unknown ticket ${id}`);
      tickets = tickets.map((ticket, position) => (position === index ? { ...ticket, ...changes } : ticket));
      return { ...tickets[index] };
    },
  };
}
