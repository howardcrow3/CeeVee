// Talks to the CeeVee desktop app on this computer. The page never does: it asks this script.
// Nothing here goes to the internet, and the only thing stored is the pairing token.
/* global chrome, browser */
const api = typeof browser !== 'undefined' ? browser : chrome;
const FIRST_PORT = 47831;
const PORT_COUNT = 10;

let port = null;
let pairing = null; // { id, started } while waiting for the person to approve in the app

async function token() {
  const v = await api.storage.local.get('token');
  return v.token || null;
}

async function call(method, path, body, withToken) {
  const headers = { 'Content-Type': 'application/json' };
  if (withToken) headers.Authorization = `Bearer ${await token()}`;
  const res = await fetch(`http://127.0.0.1:${port}${path}`, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body)
  });
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw Object.assign(new Error(data.error || `HTTP ${res.status}`), { status: res.status });
  return data;
}

/** Find the app. Remembers the port while it keeps answering. */
async function findApp() {
  if (port !== null) {
    try {
      await call('GET', '/ping');
      return true;
    } catch {
      port = null;
    }
  }
  for (let p = FIRST_PORT; p < FIRST_PORT + PORT_COUNT; p += 1) {
    try {
      const res = await fetch(`http://127.0.0.1:${p}/ping`, { signal: AbortSignal.timeout(800) });
      const data = await res.json();
      if (data.app === 'CeeVee') {
        port = p;
        return true;
      }
    } catch {
      /* not this one */
    }
  }
  return false;
}

function browserName() {
  const ua = navigator.userAgent;
  if (ua.includes('Edg/')) return 'Edge';
  if (ua.includes('Firefox/')) return 'Firefox';
  return 'Chrome';
}

async function status() {
  if (!(await findApp())) return { state: 'no_app' };
  if (pairing) return { state: 'waiting' };
  const t = await token();
  if (!t) return { state: 'not_paired' };
  try {
    // Any call that needs the token tells us whether it is still good.
    await call('GET', '/document/resume', undefined, true);
    return { state: 'connected' };
  } catch (e) {
    if (e.status === 401) {
      await api.storage.local.remove('token');
      return { state: 'not_paired' };
    }
    return { state: 'connected' }; // 404: no resume yet, but the token works
  }
}

async function pair() {
  if (!(await findApp())) return { state: 'no_app' };
  const { id } = await call('POST', '/pair', { browser: browserName() });
  pairing = { id };
  const give_up = Date.now() + 120000;
  while (Date.now() < give_up) {
    await new Promise((r) => setTimeout(r, 1000));
    const s = await call('GET', `/pair/${id}`);
    if (s.state === 'approved') {
      await api.storage.local.set({ token: s.token });
      pairing = null;
      return { state: 'connected' };
    }
    if (s.state === 'denied') break;
  }
  pairing = null;
  return { state: 'not_paired' };
}

const handlers = {
  status,
  pair,
  plan: (m) => call('POST', '/plan', m.request, true),
  document: (m) => call('GET', `/document/${m.kind}`, undefined, true)
};

api.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  const h = handlers[msg && msg.type];
  if (!h) return false;
  (async () => {
    if (msg.type !== 'status' && msg.type !== 'pair' && !(await findApp())) {
      sendResponse({ error: 'CeeVee is not running.' });
      return;
    }
    try {
      sendResponse({ ok: await h(msg) });
    } catch (e) {
      sendResponse({ error: String(e.message || e), status: e.status });
    }
  })();
  return true;
});
