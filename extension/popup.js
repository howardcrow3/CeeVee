/* global chrome, browser */
const api = typeof browser !== 'undefined' ? browser : chrome;
const msg = document.getElementById('msg');
const connect = document.getElementById('connect');

const TEXT = {
  no_app: 'CeeVee is not running. Open the CeeVee app on this computer, then reopen this.',
  not_paired: 'Not connected yet. Press the button, then approve this browser in the CeeVee app.',
  waiting: 'Waiting for you to approve this browser in the CeeVee app…',
  connected: 'Connected. On a job application page, press “Fill with CeeVee” at the bottom right.'
};

function ask(m) {
  return new Promise((resolve) => api.runtime.sendMessage(m, (r) => resolve(r || {})));
}

function render(state) {
  msg.textContent = TEXT[state] || TEXT.no_app;
  connect.hidden = state !== 'not_paired';
}

async function refresh() {
  const r = await ask({ type: 'status' });
  render((r.ok && r.ok.state) || 'no_app');
}

connect.onclick = async () => {
  render('waiting');
  const r = await ask({ type: 'pair' });
  render((r.ok && r.ok.state) || 'not_paired');
};

refresh();
