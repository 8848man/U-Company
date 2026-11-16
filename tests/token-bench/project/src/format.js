const TAGS = { urgent: '[!!]', high: '[! ]', normal: '[  ]', low: '[ .]' };

export function formatTicket(ticket) {
  const tag = TAGS[ticket.priority] ?? TAGS.normal;
  return `${tag} ${ticket.id} ${ticket.title} (${ticket.status})`;
}

export function formatList(tickets) {
  return tickets.length ? tickets.map(formatTicket).join('\n') : '(empty)';
}
