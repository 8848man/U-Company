// Test-only data setup. Never point this at a user's normal app database.
import { DatabaseSync } from 'node:sqlite';

const path = process.argv[2];
if (!path || !path.includes('.run-data')) throw new Error('Expected isolated .run-data database');
const db = new DatabaseSync(path);
const find = db.prepare("SELECT id,payload FROM objects WHERE kind='agent'");
const update = db.prepare("UPDATE objects SET payload=? WHERE kind='agent' AND id=?");
for (const row of find.all()) {
  const agent = JSON.parse(row.payload);
  if (['cto', 'pm', 'developer-a', 'developer-b', 'qa'].includes(row.id)) {
    agent.connectionId = 'codex';
    update.run(JSON.stringify(agent), row.id);
  }
}
db.close();
