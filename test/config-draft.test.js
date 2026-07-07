import assert from 'node:assert/strict';
import test from 'node:test';

import {
  CONFIG_DRAFT_DESKTOP_ONLY,
  CONFIG_DRAFT_SUCCESS_MESSAGE,
  defaultConfigDraftSkeleton,
  draftErrorsFromInvoke,
  formatConfigError,
  normalizeDraftError,
} from '../src/lib/config-draft.js';
import { isTauriDesktop } from '../src/lib/tauri.js';

test('isTauriDesktop is false when invoke is unavailable', () => {
  const previous = globalThis.__TAURI__;
  // @ts-expect-error test override
  globalThis.__TAURI__ = undefined;
  assert.equal(isTauriDesktop(), false);
  globalThis.__TAURI__ = previous;
});

test('desktop-only guard message is shown when invoke is unavailable', () => {
  assert.match(CONFIG_DRAFT_DESKTOP_ONLY, /desktop app/i);
});

test('draftErrorsFromInvoke formats parse errors', () => {
  const messages = draftErrorsFromInvoke({ type: 'parse', message: 'trailing comma at line 4' });
  assert.deepEqual(messages, ['trailing comma at line 4']);
});

test('draftErrorsFromInvoke formats semantic validation errors', () => {
  const messages = draftErrorsFromInvoke({
    type: 'invalid',
    errors: [{ type: 'deviceNameNotFound', deviceName: 'ghost' }],
  });
  assert.deepEqual(messages, [
    "deviceName 'ghost' does not match any entry in devices[]",
  ]);
});

test('formatConfigError includes monitor and device ids in plain text', () => {
  const message = formatConfigError({
    type: 'unknownDeviceInMonitorInput',
    monitorId: 'monitor1',
    deviceId: 'device-a',
  });
  assert.ok(message.includes('monitor1'));
  assert.ok(message.includes('device-a'));
});

test('draftErrorsFromInvoke returns plain-text messages suitable for textContent', () => {
  const messages = draftErrorsFromInvoke({
    type: 'invalid',
    errors: [{ type: 'duplicateDeviceId', deviceId: 'device-a' }],
  });
  assert.deepEqual(messages, ["duplicate device id 'device-a' in devices[]"]);
  assert.ok(!messages[0].includes('<'));
});

test('success message says restart is required', () => {
  assert.equal(
    CONFIG_DRAFT_SUCCESS_MESSAGE,
    'Saved to deskmux.config.json. Restart DeskMux for this configuration to take effect.',
  );
});

test('defaultConfigDraftSkeleton is valid JSON with required top-level keys', () => {
  const parsed = JSON.parse(defaultConfigDraftSkeleton());
  assert.equal(typeof parsed.deviceName, 'string');
  assert.ok(Array.isArray(parsed.devices));
  assert.ok(Array.isArray(parsed.monitors));
  assert.equal(typeof parsed.presets, 'object');
});

test('normalizeDraftError parses JSON error strings', () => {
  const payload = normalizeDraftError('{"type":"io","message":"denied"}');
  assert.deepEqual(payload, { type: 'io', message: 'denied' });
});
