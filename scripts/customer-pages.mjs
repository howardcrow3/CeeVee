// Turns docs/customer/*.md into standalone HTML pages for turtlebox.studio/ceevee/{help,privacy,terms}.
//   node scripts/customer-pages.mjs [outDir]      (default: docs/customer/site)
// A tiny converter on purpose: headings, paragraphs, lists, tables, bold, code and links are all the
// customer docs use.
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const esc = (s) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
const inline = (s) =>
  esc(s)
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>');

export function render(md) {
  const out = [];
  const lines = md.split('\n');
  let i = 0;
  while (i < lines.length) {
    const l = lines[i];
    if (!l.trim()) { i++; continue; }
    const h = /^(#{1,3}) (.*)/.exec(l);
    if (h) { out.push(`<h${h[1].length}>${inline(h[2])}</h${h[1].length}>`); i++; continue; }
    if (l.startsWith('|')) {
      const rows = [];
      while (i < lines.length && lines[i].startsWith('|')) rows.push(lines[i++]);
      const cells = (r) => r.replace(/^\||\|$/g, '').split('|').map((c) => inline(c.trim()));
      const [head, , ...body] = rows;
      out.push('<table><thead><tr>' + cells(head).map((c) => `<th>${c}</th>`).join('') + '</tr></thead><tbody>' +
        body.map((r) => '<tr>' + cells(r).map((c) => `<td>${c}</td>`).join('') + '</tr>').join('') + '</tbody></table>');
      continue;
    }
    const ul = /^- /.test(l), ol = /^\d+\. /.test(l);
    if (ul || ol) {
      const items = [];
      const re = ul ? /^- / : /^\d+\. /;
      while (i < lines.length && re.test(lines[i])) items.push(`<li>${inline(lines[i++].replace(re, '').replace(/^\[[ x]\] /, ''))}</li>`);
      out.push(`<${ul ? 'ul' : 'ol'}>${items.join('')}</${ul ? 'ul' : 'ol'}>`);
      continue;
    }
    const para = [];
    while (i < lines.length && lines[i].trim() && !/^(#|\||- |\d+\. )/.test(lines[i])) para.push(lines[i++]);
    out.push(`<p>${inline(para.join(' '))}</p>`);
  }
  return out.join('\n');
}

export function page(title, body) {
  return `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>${esc(title)}</title>
<style>body{font:16px/1.6 system-ui,sans-serif;max-width:720px;margin:48px auto;padding:0 20px;color:#222}
table{border-collapse:collapse;width:100%}th,td{border:1px solid #ccc;padding:8px;text-align:left;vertical-align:top}
code{background:#f2f2f2;padding:1px 4px}</style></head>
<body>
${body}
</body></html>
`;
}

function main() {
  const root = join(dirname(fileURLToPath(import.meta.url)), '..');
  const outDir = process.argv[2] ?? join(root, 'docs/customer/site');
  mkdirSync(outDir, { recursive: true });
  for (const [file, name, title] of [['HELP', 'help', 'CeeVee help'], ['PRIVACY', 'privacy', 'CeeVee privacy notice'], ['TERMS', 'terms', 'CeeVee terms of use']]) {
    const md = readFileSync(join(root, 'docs/customer', `${file}.md`), 'utf8');
    writeFileSync(join(outDir, `${name}.html`), page(title, render(md)));
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();
