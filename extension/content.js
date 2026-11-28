// Runs on job application pages. It reads the form, asks the CeeVee app what to fill, fills it,
// and shows the blank-field check. It never clicks Submit: that is always the person's.
/* global chrome, browser, CeeVeeLib */
(function () {
  const api = typeof browser !== 'undefined' ? browser : chrome;
  const L = CeeVeeLib;
  const ask = (msg) =>
    new Promise((resolve) =>
      api.runtime.sendMessage(msg, (r) => resolve(r || { error: 'No answer from CeeVee.' }))
    );

  function visible(el) {
    const r = el.getBoundingClientRect();
    const s = getComputedStyle(el);
    return (r.width > 0 || el.type === 'file') && s.visibility !== 'hidden' && s.display !== 'none';
  }

  function labelFor(el) {
    const byFor = el.id ? document.querySelector(`label[for="${CSS.escape(el.id)}"]`) : null;
    const wrap = el.closest('label');
    const labelledBy = (el.getAttribute('aria-labelledby') || '')
      .split(/\s+/)
      .map((id) => (document.getElementById(id) || {}).textContent || '')
      .join(' ');
    const group = el.closest('fieldset');
    return L.pickLabel({
      labelText: (byFor && byFor.textContent) || (wrap && wrap.textContent) || '',
      ariaLabelledBy: labelledBy,
      ariaLabel: el.getAttribute('aria-label'),
      legend: group && group.querySelector('legend') ? group.querySelector('legend').textContent : '',
      placeholder: el.getAttribute('placeholder'),
      name: el.getAttribute('name')
    });
  }

  let counter = 0;
  const byId = new Map();

  function readFields() {
    byId.clear();
    const out = [];
    for (const el of document.querySelectorAll('input, textarea, select')) {
      const kind = L.controlKind(el.tagName, el.getAttribute('type'));
      if (!kind || !visible(el)) continue;
      const id = `c${(counter += 1)}`;
      byId.set(id, el);
      out.push({
        id,
        label: labelFor(el),
        kind,
        options: kind === 'select' ? Array.from(el.options).map((o) => o.text.trim()) : [],
        required: el.required || el.getAttribute('aria-required') === 'true'
      });
    }
    return out;
  }

  /** Set a value the way a person typing would, so the page's own scripts notice. */
  function setValue(el, value) {
    if (el.tagName === 'SELECT') {
      const opt = Array.from(el.options).find((o) => o.text.trim() === value);
      if (!opt) return;
      el.value = opt.value;
    } else {
      const proto = el.tagName === 'TEXTAREA' ? HTMLTextAreaElement : HTMLInputElement;
      Object.getOwnPropertyDescriptor(proto.prototype, 'value').set.call(el, value);
    }
    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
  }

  async function attach(el, kind) {
    const r = await ask({ type: 'document', kind });
    if (r.error || !r.ok) return false;
    const bytes = Uint8Array.from(atob(r.ok.data_base64), (c) => c.charCodeAt(0));
    const file = new File([bytes], r.ok.file_name, { type: 'application/pdf' });
    const dt = new DataTransfer();
    dt.items.add(file);
    el.files = dt.files;
    el.dispatchEvent(new Event('change', { bubbles: true }));
    return true;
  }

  async function fill() {
    const reply = await ask({
      type: 'plan',
      request: {
        url: location.href,
        page_text: (document.body.innerText || '').slice(0, 20000),
        fields: readFields()
      }
    });
    if (reply.error) return { error: reply.error };
    const plan = reply.ok;
    if (plan.refused) return { refused: plan.refused };
    for (const step of plan.steps) {
      const el = byId.get(step.field_id);
      if (!el) continue;
      if (step.action === 'fill') setValue(el, step.value);
      else await attach(el, step.value);
    }
    return { plan };
  }

  // --- A small panel in a shadow root, so the page's styles cannot touch it. ---
  const host = document.createElement('div');
  host.style.cssText = 'position:fixed;right:16px;bottom:16px;z-index:2147483647;font:13px system-ui,sans-serif';
  const root = host.attachShadow({ mode: 'closed' });
  const css = `
    .btn{background:#1f2937;color:#fff;border:0;border-radius:8px;padding:10px 14px;font:inherit;cursor:pointer}
    .btn:disabled{opacity:.6;cursor:default}
    .panel{background:#fff;color:#111;border:1px solid #d1d5db;border-radius:10px;padding:12px;width:320px;max-height:60vh;overflow:auto;box-shadow:0 8px 24px rgba(0,0,0,.18);margin-bottom:8px}
    .panel h3{margin:0 0 6px;font-size:13px}
    ul{list-style:none;margin:8px 0 0;padding:0}
    li{padding:4px 0;border-top:1px solid #eee}
    .tag{display:inline-block;min-width:74px;font-weight:600}
    .needs_you{color:#b91c1c}.check_this{color:#92400e}.left_blank{color:#6b7280}.filled{color:#166534}
    .detail{color:#6b7280;display:block}
    .row{display:flex;gap:8px;margin-top:8px}
    .ghost{background:transparent;color:#111;border:1px solid #d1d5db}
  `;
  const style = document.createElement('style');
  style.textContent = css;
  const box = document.createElement('div');
  const panel = document.createElement('div');
  panel.className = 'panel';
  panel.hidden = true;
  const button = document.createElement('button');
  button.className = 'btn';
  button.textContent = 'Fill with CeeVee';
  box.append(panel, button);
  root.append(style, box);

  const TAG = { filled: 'Filled', check_this: 'Check this', needs_you: 'Needs you', left_blank: 'Left blank' };

  function show(title, bodyNodes) {
    panel.replaceChildren();
    const h = document.createElement('h3');
    h.textContent = title;
    panel.append(h, ...bodyNodes);
    const row = document.createElement('div');
    row.className = 'row';
    const close = document.createElement('button');
    close.className = 'btn ghost';
    close.textContent = 'Close';
    close.onclick = () => (panel.hidden = true);
    row.append(close);
    panel.append(row);
    panel.hidden = false;
  }

  function line(text) {
    const p = document.createElement('p');
    p.textContent = text;
    p.style.margin = '0';
    return p;
  }

  button.onclick = async () => {
    button.disabled = true;
    try {
      const r = await fill();
      if (r.error) {
        show('CeeVee could not fill this page', [line(r.error), line('Open the CeeVee app and connect this browser from the CeeVee icon in the toolbar.')]);
      } else if (r.refused) {
        show('CeeVee leaves this page to you', [line(r.refused)]);
      } else {
        const list = document.createElement('ul');
        for (const i of r.plan.checklist) {
          const li = document.createElement('li');
          const tag = document.createElement('span');
          tag.className = `tag ${i.state}`;
          tag.textContent = TAG[i.state] || i.state;
          const label = document.createElement('span');
          label.textContent = ` ${i.label}`;
          const detail = document.createElement('span');
          detail.className = 'detail';
          detail.textContent = i.detail;
          li.append(tag, label, detail);
          list.append(li);
        }
        show(L.summaryText(r.plan.checklist, r.plan.ready_to_submit), [list]);
      }
    } finally {
      button.disabled = false;
    }
  };

  // Offer the button only on pages that look like a form worth filling.
  function offer() {
    if (host.isConnected) return;
    const inputs = document.querySelectorAll('input, textarea, select');
    if (inputs.length >= 3) document.documentElement.append(host);
  }
  offer();
  new MutationObserver(offer).observe(document.documentElement, { childList: true, subtree: true });
})();
