import assert from 'node:assert/strict';
import test from 'node:test';

import {
  NATIVE_CONTROLS_DESKTOP_ONLY,
  setNativeDdcControl,
} from '../src/api/native-controls.js';

test('setNativeDdcControl requires the desktop IPC surface', async () => {
  const previous = globalThis.__TAURI__;
  try {
    delete globalThis.__TAURI__;
    await assert.rejects(
      () => setNativeDdcControl('K@P:d0e5:0', 'brightness', 70),
      new RegExp(NATIVE_CONTROLS_DESKTOP_ONLY),
    );
  } finally {
    globalThis.__TAURI__ = previous;
  }
});

test('setNativeDdcControl invokes only the named control command', async () => {
  const previous = globalThis.__TAURI__;
  /** @type {unknown[]} */
  const calls = [];
  try {
    globalThis.__TAURI__ = {
      core: {
        invoke: async (cmd, args) => {
          calls.push({ cmd, args });
          return {
            accepted: true,
            displayId: args.displayId,
            feature: args.feature,
            value: args.value,
            maximum: 100,
          };
        },
      },
    };

    const result = await setNativeDdcControl('K@P:d0e5:0', 'volume', 12);

    assert.deepEqual(calls, [
      {
        cmd: 'set_native_ddc_control',
        args: { displayId: 'K@P:d0e5:0', feature: 'volume', value: 12 },
      },
    ]);
    assert.equal(result.feature, 'volume');
  } finally {
    globalThis.__TAURI__ = previous;
  }
});
