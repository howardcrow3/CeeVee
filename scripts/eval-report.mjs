// Builds the model comparison report from the JSON files `ceevee-eval` writes.
//
//   node scripts/eval-report.mjs <results-dir> <out-dir> [--thresholds evals/thresholds.json]
//
// Writes eval-report.md (for the job summary) and eval-report.html (for the artifact). It never
// exits non-zero because of a score; thresholds only change what the report says.
import { readFileSync, readdirSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

export function threshold(thresholds, model, task) {
  const v = thresholds?.models?.[model]?.[task] ?? thresholds?.default?.[task];
  return typeof v === 'number' ? v : null;
}

export function verdict(score, floor) {
  if (floor === null) return 'not set';
  return score >= floor ? 'pass' : 'fail';
}

const pct = (x) => (x === null || x === undefined ? 'n/a' : x.toFixed(2));

/** results: parsed JSON reports. Returns { tasks, models, cells, cases }. */
export function summarize(results, thresholds) {
  const tasks = [...new Set(results.map((r) => r.task))].sort();
  const models = [...new Set(results.map((r) => r.model))].sort();
  const cells = {};
  for (const r of results) {
    const floor = threshold(thresholds, r.model, r.task);
    cells[`${r.model}|${r.task}`] = {
      mean: r.mean_score, failed: r.failed, skipped: r.skipped, total: r.total,
      seconds: r.seconds, floor, verdict: verdict(r.mean_score, floor), version: r.version,
    };
  }
  const cases = {};
  for (const r of results) {
    for (const c of r.cases) {
      ((cases[r.task] ??= {})[c.name] ??= {})[r.model] = c.score;
    }
  }
  return { tasks, models, cells, cases };
}

export function toMarkdown(sum, meta = {}) {
  const out = ['# CeeVee model eval report', ''];
  if (meta.commit) out.push(`Commit \`${meta.commit}\`, ${meta.date ?? ''}`.trim(), '');
  out.push('Mean score per task, 0 to 1 (higher is better). Failed = no valid output after the retry. Skipped = cases the time budget cut off. Pass/fail uses `evals/thresholds.json`; this report never fails a build.', '');
  out.push(`| Model | ${sum.tasks.join(' | ')} |`, `|---|${sum.tasks.map(() => '---').join('|')}|`);
  for (const m of sum.models) {
    const row = sum.tasks.map((t) => {
      const c = sum.cells[`${m}|${t}`];
      if (!c) return 'not run';
      const extra = [c.failed ? `${c.failed} failed` : '', c.skipped ? `${c.skipped} skipped` : ''].filter(Boolean).join(', ');
      return `${pct(c.mean)} ${c.verdict === 'not set' ? '' : `(${c.verdict}, floor ${pct(c.floor)})`}${extra ? ` [${extra}]` : ''}`.replace(/\s+/g, ' ').trim();
    });
    out.push(`| ${m} | ${row.join(' | ')} |`);
  }
  for (const t of sum.tasks) {
    out.push('', `## ${t}`, '', `| Case | ${sum.models.join(' | ')} |`, `|---|${sum.models.map(() => '---').join('|')}|`);
    for (const [name, byModel] of Object.entries(sum.cases[t] ?? {})) {
      out.push(`| ${name} | ${sum.models.map((m) => (m in byModel ? (byModel[m] === null ? 'FAIL' : pct(byModel[m])) : '-')).join(' | ')} |`);
    }
  }
  return out.join('\n') + '\n';
}

const esc = (s) => String(s).replace(/[&<>]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' })[c]);

export function toHtml(sum, meta = {}) {
  const colour = (s) => (s === null ? '#b3261e' : s >= 0.9 ? '#1b7f3b' : s >= 0.5 ? '#8a6d00' : '#b3261e');
  const cell = (v) => `<td style="color:${colour(v)}">${v === null ? 'FAIL' : pct(v)}</td>`;
  const head = `<tr><th>Model</th>${sum.tasks.map((t) => `<th>${esc(t)}</th>`).join('')}</tr>`;
  const rows = sum.models.map((m) => `<tr><th>${esc(m)}</th>${sum.tasks.map((t) => {
    const c = sum.cells[`${m}|${t}`];
    if (!c) return '<td>not run</td>';
    return `${cell(c.mean).replace('</td>', ` <small>${c.verdict} ${c.failed ? `· ${c.failed} failed` : ''} ${c.skipped ? `· ${c.skipped} skipped` : ''}</small></td>`)}`;
  }).join('')}</tr>`).join('');
  const detail = sum.tasks.map((t) => `<h2>${esc(t)}</h2><table><tr><th>Case</th>${sum.models.map((m) => `<th>${esc(m)}</th>`).join('')}</tr>${Object.entries(sum.cases[t] ?? {}).map(([n, by]) => `<tr><th>${esc(n)}</th>${sum.models.map((m) => (m in by ? cell(by[m]) : '<td>-</td>')).join('')}</tr>`).join('')}</table>`).join('');
  return `<!doctype html><meta charset="utf-8"><title>CeeVee model eval report</title>
<style>body{font:15px system-ui;margin:2rem auto;max-width:60rem;padding:0 1rem}table{border-collapse:collapse;margin:1rem 0;width:100%}th,td{border:1px solid #8884;padding:.35rem .6rem;text-align:left}small{opacity:.7}</style>
<h1>CeeVee model eval report</h1><p>${esc(meta.commit ?? '')} ${esc(meta.date ?? '')}</p>
<p>Mean score per task (0 to 1). Pass/fail comes from <code>evals/thresholds.json</code>; this report never fails a build.</p>
<table>${head}${rows}</table>${detail}`;
}

export function loadResults(dir) {
  return readdirSync(dir, { recursive: true })
    .filter((f) => String(f).endsWith('.json'))
    .map((f) => JSON.parse(readFileSync(join(dir, String(f)), 'utf8')))
    .filter((r) => r && r.task && r.model && Array.isArray(r.cases));
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [dir, outDir] = process.argv.slice(2);
  const ti = process.argv.indexOf('--thresholds');
  const thresholds = ti > 0 && existsSync(process.argv[ti + 1]) ? JSON.parse(readFileSync(process.argv[ti + 1], 'utf8')) : {};
  const results = existsSync(dir) ? loadResults(dir) : [];
  mkdirSync(outDir, { recursive: true });
  const meta = { commit: (process.env.GITHUB_SHA ?? '').slice(0, 7), date: new Date().toISOString().slice(0, 10) };
  const sum = summarize(results, thresholds);
  writeFileSync(join(outDir, 'eval-report.md'), results.length ? toMarkdown(sum, meta) : '# CeeVee model eval report\n\nNo results were produced.\n');
  writeFileSync(join(outDir, 'eval-report.html'), toHtml(sum, meta));
  console.log(`report for ${sum.models.length} model(s), ${sum.tasks.length} task(s)`);
}
