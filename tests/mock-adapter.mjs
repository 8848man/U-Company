// Test-only protocol v1 adapter. It never contacts a model or network.
import readline from 'node:readline';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';

for await (const line of readline.createInterface({ input: process.stdin })) {
  const request = JSON.parse(line);
  if (request.method === 'initialize') {
    process.stdout.write(JSON.stringify({ id: request.id, result: { protocolVersion: process.argv.includes('--invalid-version') ? 2 : 1 } }) + '\n');
  } else if (request.method === 'send_task') {
    if (request.params.writable) writeFileSync(join(request.params.cwd, 'adapter-result.txt'), 'mock task result\n');
    process.stdout.write(JSON.stringify({ id: request.id, result: { text: 'PASS: mock adapter task completed', sessionId: 'mock-session' } }) + '\n');
  }
}
