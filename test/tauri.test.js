import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { EVENT_OPEN_SETUP_WIZARD, tauriListen } from '../src/lib/tauri.js';

test('tauriListen returns undefined outside the desktop shell', () => {
  const previous = globalThis.__TAURI__;
  delete globalThis.__TAURI__;
  try {
    assert.equal(tauriListen(), undefined);
  } finally {
    if (previous !== undefined) {
      globalThis.__TAURI__ = previous;
    }
  }
});

test('tauriListen stays bound to the Tauri event namespace', async () => {
  const previous = globalThis.__TAURI__;
  /** @type {unknown[]} */
  const seen = [];
  const events = {
    listen(/** @type {string} */ name) {
      seen.push([name, this === events]);
      return Promise.resolve(() => {});
    },
  };
  globalThis.__TAURI__ = { event: events };

  try {
    const listen = tauriListen();
    assert.equal(typeof listen, 'function');
    await listen(EVENT_OPEN_SETUP_WIZARD, () => {});
    assert.deepEqual(seen, [[EVENT_OPEN_SETUP_WIZARD, true]]);
  } finally {
    if (previous === undefined) {
      delete globalThis.__TAURI__;
    } else {
      globalThis.__TAURI__ = previous;
    }
  }
});

test('EVENT_OPEN_SETUP_WIZARD matches the Rust tray constant', () => {
  const trayRs = readFileSync(new URL('../src-tauri/src/tray.rs', import.meta.url), 'utf8');
  const match = trayRs.match(/pub const EVENT_OPEN_SETUP_WIZARD: &str = "([^"]+)";/);
  assert.ok(match, 'src-tauri/src/tray.rs should declare EVENT_OPEN_SETUP_WIZARD');
  assert.equal(match[1], EVENT_OPEN_SETUP_WIZARD);
});
