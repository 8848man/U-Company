// Token measurement (WLG-091): reads the JSON lines written by `cargo run --bin token-bench -- run ...`,
// finds each run's own CLI session log, and counts internal responses, fresh input, cache and output.
//
// Input is counted the same way for both CLIs, as total input the model processed:
//   Claude = input_tokens + cache_creation_input_tokens + cache_read_input_tokens (per API response)
//   Codex  = input_tokens (already includes cached_input_tokens)
// The multiplier is Claude input / Codex input per condition, from medians across repeated runs.
//
// Usage: node scripts/token-bench/collect.mjs <runs.jsonl> [--out enriched.jsonl] [--claude-home DIR] [--codex-home DIR]
import { existsSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { zstdDecompressSync } from 'node:zlib';

function jsonLines(text) {
  return text.split(/\r?\n/).filter((line) => line.trim()).flatMap((line) => {
    try { return [JSON.parse(line)]; } catch { return []; }
  });
}

function walk(dir, match, found = []) {
  if (!existsSync(dir)) return found;
  for (const name of readdirSync(dir)) {
    const full = path.join(dir, name);
    if (statSync(full).isDirectory()) walk(full, match, found);
    else if (match(name, full)) found.push(full);
  }
  return found;
}

/** Claude Code keeps `<projects>/<encoded cwd>/<session>.jsonl` and subagent logs under `<session>/subagents/`. */
export function claudeLogs(home, sessionId) {
  const projects = path.join(home, 'projects');
  if (!existsSync(projects)) return [];
  const logs = [];
  for (const dir of readdirSync(projects)) {
    const main = path.join(projects, dir, `${sessionId}.jsonl`);
    if (existsSync(main)) logs.push(main);
    logs.push(...walk(path.join(projects, dir, sessionId), (name) => name.endsWith('.jsonl')));
  }
  return logs;
}

/** One API response can be split over several log lines with the same message id; count it once. */
export function claudeUsage(texts) {
  const responses = new Map();
  const versions = new Set();
  for (const text of texts) {
    for (const entry of jsonLines(text)) {
      if (entry.version) versions.add(entry.version);
      const message = entry.type === 'assistant' ? entry.message : null;
      if (!message?.usage) continue;
      responses.set(message.id ?? `${entry.uuid}`, { usage: message.usage, model: message.model });
    }
  }
  const sum = { responses: responses.size, freshInput: 0, cacheWrite: 0, cacheRead: 0, output: 0, models: [], cliVersions: [...versions] };
  for (const { usage, model } of responses.values()) {
    sum.freshInput += usage.input_tokens ?? 0;
    sum.cacheWrite += usage.cache_creation_input_tokens ?? 0;
    sum.cacheRead += usage.cache_read_input_tokens ?? 0;
    sum.output += usage.output_tokens ?? 0;
    if (model && model !== '<synthetic>' && !sum.models.includes(model)) sum.models.push(model);
  }
  sum.input = sum.freshInput + sum.cacheWrite + sum.cacheRead;
  sum.cacheShare = sum.input ? sum.cacheRead / sum.input : 0;
  return sum;
}

/** Codex keeps `<home>/sessions/YYYY/MM/DD/rollout-<time>-<session>.jsonl`, and compresses older ones to `.jsonl.zst`. */
export function codexLogs(home, sessionId) {
  return walk(path.join(home, 'sessions'), (name) => name.endsWith(`${sessionId}.jsonl`) || name.endsWith(`${sessionId}.jsonl.zst`));
}

export function readLog(file) {
  return file.endsWith('.zst') ? zstdDecompressSync(readFileSync(file)).toString('utf8') : readFileSync(file, 'utf8');
}

/** Codex writes one `token_usage_record` per model response; older logs only have cumulative `token_count`. */
export function codexUsage(texts) {
  const records = new Map();
  let total = null;
  let model = null;
  const versions = new Set();
  for (const text of texts) {
    for (const entry of jsonLines(text)) {
      const payload = entry.payload ?? {};
      if (entry.type === 'session_meta' && payload.cli_version) versions.add(payload.cli_version);
      if (entry.type === 'token_usage_record' && payload.usage) records.set(payload.response_id ?? `${entry.ordinal}`, payload.usage);
      if (entry.type === 'event_msg' && payload.type === 'token_count' && payload.info?.total_token_usage) total = payload.info.total_token_usage;
      if (entry.type === 'turn_context' && payload.model) model = payload.model;
    }
  }
  const usages = records.size ? [...records.values()] : total ? [total] : [];
  const sum = { responses: records.size || null, freshInput: 0, cacheWrite: 0, cacheRead: 0, output: 0, reasoning: 0, models: model ? [model] : [], cliVersions: [...versions] };
  for (const usage of usages) {
    sum.cacheRead += usage.cached_input_tokens ?? 0;
    sum.cacheWrite += usage.cache_write_input_tokens ?? 0;
    sum.freshInput += (usage.input_tokens ?? 0) - (usage.cached_input_tokens ?? 0);
    sum.output += usage.output_tokens ?? 0;
    sum.reasoning += usage.reasoning_output_tokens ?? 0;
  }
  sum.input = sum.freshInput + sum.cacheRead;
  sum.cacheShare = sum.input ? sum.cacheRead / sum.input : 0;
  return sum;
}

export function collectRun(run, homes) {
  if (!run.sessionId || !['claude', 'codex'].includes(run.provider)) return { ...run, log: null };
  const files = run.provider === 'claude' ? claudeLogs(homes.claude, run.sessionId) : codexLogs(homes.codex, run.sessionId);
  const texts = files.map(readLog);
  const log = files.length ? (run.provider === 'claude' ? claudeUsage(texts) : codexUsage(texts)) : null;
  return { ...run, log: log && { ...log, files: files.length } };
}

const median = (values) => {
  const sorted = values.filter((value) => Number.isFinite(value)).sort((a, b) => a - b);
  if (!sorted.length) return null;
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
};

/** Medians per provider x condition, and Claude/Codex multipliers per condition. */
export function summarize(runs) {
  const groups = {};
  for (const run of runs) {
    const key = `${run.provider}/${run.condition}`;
    (groups[key] ??= []).push(run);
  }
  const cells = {};
  for (const [key, list] of Object.entries(groups)) {
    const pick = (fn) => median(list.map(fn));
    cells[key] = {
      runs: list.length,
      passed: list.filter((run) => run.check?.fail === 0 && run.check?.pass > 0 && !run.undeclaredChanges?.length && !run.error).length,
      input: pick((run) => run.log?.input ?? run.appInputTokens),
      output: pick((run) => run.log?.output ?? run.appOutputTokens),
      responses: pick((run) => run.log?.responses),
      inputPerResponse: pick((run) => (run.log?.responses ? run.log.input / run.log.responses : NaN)),
      cacheShare: pick((run) => run.log?.cacheShare),
      seconds: pick((run) => run.seconds),
    };
  }
  const multiplier = {};
  for (const condition of ['before', 'after']) {
    const claude = cells[`claude/${condition}`]?.input;
    const codex = cells[`codex/${condition}`]?.input;
    multiplier[condition] = claude && codex ? claude / codex : null;
  }
  const change = multiplier.before && multiplier.after ? multiplier.after / multiplier.before - 1 : null;
  return { cells, multiplier, change };
}

function main(argv) {
  const flag = (name) => { const index = argv.indexOf(name); return index >= 0 ? argv[index + 1] : undefined; };
  const input = argv.find((arg, index) => !arg.startsWith('--') && !argv[index - 1]?.startsWith('--'));
  if (!input) { console.error('Usage: node scripts/token-bench/collect.mjs <runs.jsonl> [--out FILE] [--claude-home DIR] [--codex-home DIR]'); process.exit(1); }
  const homes = {
    claude: flag('--claude-home') ?? process.env.CLAUDE_CONFIG_DIR ?? path.join(os.homedir(), '.claude'),
    codex: flag('--codex-home') ?? process.env.CODEX_HOME ?? path.join(os.homedir(), '.codex'),
  };
  const runs = jsonLines(readFileSync(input, 'utf8')).map((run) => collectRun(run, homes));
  const out = flag('--out');
  if (out) writeFileSync(out, runs.map((run) => JSON.stringify(run)).join('\n') + '\n');
  const missing = runs.filter((run) => !run.log && run.provider !== 'mock').length;
  console.log(JSON.stringify({ runs: runs.length, missingLogs: missing, ...summarize(runs) }, null, 2));
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main(process.argv.slice(2));
