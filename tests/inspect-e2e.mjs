import { DatabaseSync } from 'node:sqlite';

const path = process.argv[2];
if (!path || !path.includes('.run-data')) throw new Error('Expected isolated .run-data database');
const db = new DatabaseSync(path, { readOnly: true });
const objects = db.prepare("SELECT kind,payload FROM objects WHERE kind IN ('project','goal','task','approval')").all();
for (const row of objects) {
  const value = JSON.parse(row.payload);
  if (row.kind === 'project') console.log(JSON.stringify({ kind: row.kind, path: value.path }));
  else if (row.kind === 'goal') console.log(JSON.stringify({ kind: row.kind, id: value.id, status: value.status, resumedFrom: value.resumedFrom, report: value.report.slice(0, 200) }));
  else if (row.kind === 'task') console.log(JSON.stringify({ kind: row.kind, id: value.id, goalId: value.goalId, title: value.title, status: value.status, files: value.files, dependsOn: value.dependsOn }));
  else console.log(JSON.stringify({ kind: row.kind, id: value.id, goalId: value.goalId, status: value.status, operation: value.operation }));
}
db.close();
