// Builds the interactive eval dashboard.
//
//   node scripts/eval-dashboard.mjs <results-dir> <out-dir> [--history evals/history.json]
//        [--thresholds evals/thresholds.json] [--models evals/models.json]
//
// Writes eval-dashboard.html (self-contained) and eval-data.json (this run, one history entry).
// Like the markdown report it never exits non-zero because of a score.
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadResults } from './eval-report.mjs';

export const TASKS = ['extract_posting', 'import_resume', 'score_match', 'tailor_resume', 'write_cover_letter'];

/** One history entry from the JSON files `ceevee-eval` writes. */
export function buildRun(results, meta = {}) {
  return {
    id: meta.id ?? meta.commit ?? 'run',
    commit: meta.commit ?? '',
    date: meta.date ?? '',
    url: meta.url ?? '',
    results: results.map((r) => ({
      model: r.model, task: r.task, version: r.version,
      mean: r.mean_score, failed: r.failed ?? 0, skipped: r.skipped ?? 0, total: r.total ?? r.cases.length,
      seconds: r.seconds ?? null,
      cases: r.cases.map((c) => ({
        name: c.name, score: c.error ? null : (c.score ?? null), seconds: c.seconds ?? null,
        attempts: c.attempts ?? null, outputChars: c.output_chars ?? null, error: c.error ?? null, fields: c.fields ?? null,
      })),
    })),
  };
}

/** Replaces an entry with the same id, otherwise appends. */
export function mergeHistory(history, run) {
  const runs = (history?.runs ?? []).filter((r) => r.id !== run.id);
  runs.push(run);
  return { runs };
}

export function buildData({ history, models, thresholds }) {
  return {
    models: models.map((m) => ({ id: m.id, label: m.label ?? m.id })),
    tasks: TASKS,
    thresholds: thresholds ?? {},
    runs: history.runs,
  };
}

export function render(template, data) {
  // `<` is escaped so no text in the data can close the script tag.
  const json = JSON.stringify(data).replace(/</g, '\\u003c');
  return template.replace('/*__DATA__*/null', () => json);
}

const read = (p, fallback) => (p && existsSync(p) ? JSON.parse(readFileSync(p, 'utf8')) : fallback);

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [dir, outDir] = process.argv.slice(2);
  const opt = (name, def) => { const i = process.argv.indexOf(name); return i > 0 ? process.argv[i + 1] : def; };
  const results = existsSync(dir) ? loadResults(dir) : [];
  const here = dirname(fileURLToPath(import.meta.url));
  const sha = (process.env.GITHUB_SHA ?? '').slice(0, 7);
  const runId = process.env.GITHUB_RUN_ID;
  const meta = {
    id: runId ? `run-${runId}` : sha || 'local', commit: sha || 'local', date: new Date().toISOString().slice(0, 10),
    url: runId ? `${process.env.GITHUB_SERVER_URL}/${process.env.GITHUB_REPOSITORY}/actions/runs/${runId}` : '',
  };
  let history = read(opt('--history', 'evals/history.json'), { runs: [] });
  if (results.length) history = mergeHistory(history, buildRun(results, meta));
  const models = read(opt('--models', 'evals/models.json'), { models: [] }).models;
  const data = buildData({ history, models, thresholds: read(opt('--thresholds', 'evals/thresholds.json'), {}) });
  mkdirSync(outDir, { recursive: true });
  writeFileSync(join(outDir, 'eval-dashboard.html'), render(readFileSync(join(here, 'eval-dashboard.template.html'), 'utf8'), data));
  writeFileSync(join(outDir, 'eval-data.json'), JSON.stringify(history));
  console.log(`dashboard for ${data.runs.length} run(s), ${models.length} model(s)`);
}
