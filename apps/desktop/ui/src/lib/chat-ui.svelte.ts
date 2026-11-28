const KEY = 'ceevee.chatPane';

function read(): boolean {
  try {
    return localStorage.getItem(KEY) === 'open';
  } catch {
    return false;
  }
}

/** Whether the chat pane is open beside Jobs and Applications. Remembered between visits. */
export const chatPane = $state({ open: read() });

export function toggleChatPane() {
  chatPane.open = !chatPane.open;
  try {
    localStorage.setItem(KEY, chatPane.open ? 'open' : 'closed');
  } catch {
    // Storage can be blocked; the pane still works for this visit.
  }
}
