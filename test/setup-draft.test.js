import assert from 'node:assert/strict';
import test from 'node:test';

import {
  buildConfigDraftFromSetupState,
  buildDraftSummary,
} from '../src/lib/setup-draft.js';

const namedSession = {
  deviceName: 'Gaming PC',
  presetLabel: 'All Gaming PC',
  displays: [
    {
      displayId: 'K@P:d0e5:0',
      label: 'Display 1 — K@P:d0e5:0',
      name: 'Center 1440p',
    },
    {
      displayId: 'K@P:a28a:0',
      label: 'Display 2 — K@P:a28a:0',
      name: 'Left monitor',
    },
  ],
  readings: [
    {
      displayId: 'K@P:d0e5:0',
      label: 'Display 1',
      current: 4626,
      maximum: 4626,
      inputLabel: 'Desktop',
    },
    {
      displayId: 'K@P:a28a:0',
      label: 'Display 2',
      current: 4623,
      maximum: 4626,
      inputLabel: 'Desktop',
    },
  ],
};

test('buildConfigDraftFromSetupState uses human labels and safe ids', () => {
  const result = buildConfigDraftFromSetupState(namedSession);
  assert.equal(result.ok, true);
  if (!result.ok) {
    return;
  }

  const parsed = JSON.parse(result.json);
  assert.equal(parsed.deviceName, 'gaming_pc');
  assert.equal(parsed.devices[0].label, 'Gaming PC');
  assert.equal(parsed.monitors[0].id, 'center_1440p');
  assert.equal(parsed.monitors[0].label, 'Center 1440p');
  assert.equal(parsed.monitors[1].id, 'left_monitor');
  assert.equal(parsed.presets.all_gaming_pc.label, 'All Gaming PC');
  assert.equal(parsed.presets.all_gaming_pc.layout.center_1440p, 'gaming_pc');
});

test('buildConfigDraftFromSetupState preserves displayId and u16 value', () => {
  const result = buildConfigDraftFromSetupState({
    deviceName: 'Gaming PC',
    readings: namedSession.readings.slice(0, 1),
    displays: namedSession.displays.slice(0, 1),
  });
  assert.equal(result.ok, true);
  if (!result.ok) {
    return;
  }

  const parsed = JSON.parse(result.json);
  assert.equal(parsed.monitors[0].nativeDdc.displayId, 'K@P:d0e5:0');
  assert.equal(parsed.monitors[0].inputs.gaming_pc.nativeDdc.inputSourceValue, 4626);
});

test('buildConfigDraftFromSetupState generates unique ids for duplicate monitor names', () => {
  const result = buildConfigDraftFromSetupState({
    deviceName: 'Gaming PC',
    displays: [
      { displayId: 'a', label: 'Display 1', name: 'Desk monitor' },
      { displayId: 'b', label: 'Display 2', name: 'Desk monitor' },
    ],
    readings: [
      { displayId: 'a', label: 'Display 1', current: 4626, maximum: 4626 },
      { displayId: 'b', label: 'Display 2', current: 4623, maximum: 4626 },
    ],
  });

  assert.equal(result.ok, true);
  if (!result.ok) {
    return;
  }

  const parsed = JSON.parse(result.json);
  assert.equal(parsed.monitors[0].id, 'desk_monitor');
  assert.equal(parsed.monitors[1].id, 'desk_monitor_2');
});

test('buildConfigDraftFromSetupState defaults preset label from device name', () => {
  const result = buildConfigDraftFromSetupState({
    deviceName: 'Gaming PC',
    readings: namedSession.readings.slice(0, 1),
    displays: namedSession.displays.slice(0, 1),
  });
  assert.equal(result.ok, true);
  if (!result.ok) {
    return;
  }

  const parsed = JSON.parse(result.json);
  assert.equal(parsed.presets.all_gaming_pc.label, 'All Gaming PC');
});

test('buildConfigDraftFromSetupState reports missing device name', () => {
  const result = buildConfigDraftFromSetupState({
    readings: namedSession.readings,
  });
  assert.equal(result.ok, false);
  if (result.ok) {
    return;
  }
  assert.match(result.errors.join(' '), /computer/i);
});

test('buildConfigDraftFromSetupState reports missing readings', () => {
  const result = buildConfigDraftFromSetupState({ deviceName: 'Gaming PC' });
  assert.equal(result.ok, false);
  if (result.ok) {
    return;
  }
  assert.match(result.errors.join(' '), /capture/i);
});

test('buildDraftSummary returns readable labels for the dashboard', () => {
  const summary = buildDraftSummary(namedSession);
  assert.equal(summary.deviceLabel, 'Gaming PC');
  assert.deepEqual(summary.monitorLabels, ['Center 1440p', 'Left monitor']);
  assert.equal(summary.presetLabel, 'All Gaming PC');
});

test('buildConfigDraftFromSetupState falls back to numbered monitor labels', () => {
  const result = buildConfigDraftFromSetupState({
    deviceName: 'Gaming PC',
    displays: [{ displayId: 'K@P:d0e5:0', label: 'Display 1' }],
    readings: [
      {
        displayId: 'K@P:d0e5:0',
        label: 'Display 1',
        current: 4626,
        maximum: 4626,
      },
    ],
  });

  assert.equal(result.ok, true);
  if (!result.ok) {
    return;
  }

  const parsed = JSON.parse(result.json);
  assert.equal(parsed.monitors[0].label, 'Monitor 1');
  assert.equal(parsed.monitors[0].id, 'monitor_1');
});
