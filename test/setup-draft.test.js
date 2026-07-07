import assert from 'node:assert/strict';
import test from 'node:test';

import {
  buildConfigDraftFromSetupState,
  slugifyDeviceId,
} from '../src/lib/setup-draft.js';

const sampleSession = {
  deviceName: 'Windows PC',
  readings: [
    {
      displayId: 'K@P:d0e5:0',
      label: 'Display 1 — K@P:d0e5:0',
      current: 4626,
      maximum: 4626,
    },
  ],
};

test('slugifyDeviceId normalizes labels into stable ids', () => {
  assert.equal(slugifyDeviceId('Windows PC'), 'windows-pc');
});

test('buildConfigDraftFromSetupState includes displayId and u16 value', () => {
  const result = buildConfigDraftFromSetupState(sampleSession);
  assert.equal(result.ok, true);
  if (!result.ok) {
    return;
  }

  const parsed = JSON.parse(result.json);
  assert.equal(parsed.monitors[0].nativeDdc.displayId, 'K@P:d0e5:0');
  assert.equal(parsed.monitors[0].inputs['windows-pc'].nativeDdc.inputSourceValue, 4626);
});

test('buildConfigDraftFromSetupState creates a default preset layout', () => {
  const result = buildConfigDraftFromSetupState(sampleSession);
  assert.equal(result.ok, true);
  if (!result.ok) {
    return;
  }

  const parsed = JSON.parse(result.json);
  assert.equal(parsed.presets.all_windows_pc.layout.monitor1, 'windows-pc');
});

test('buildConfigDraftFromSetupState reports missing device name', () => {
  const result = buildConfigDraftFromSetupState({
    readings: sampleSession.readings,
  });
  assert.equal(result.ok, false);
  if (result.ok) {
    return;
  }
  assert.match(result.errors.join(' '), /computer/i);
});

test('buildConfigDraftFromSetupState reports missing readings', () => {
  const result = buildConfigDraftFromSetupState({ deviceName: 'Windows PC' });
  assert.equal(result.ok, false);
  if (result.ok) {
    return;
  }
  assert.match(result.errors.join(' '), /capture/i);
});
