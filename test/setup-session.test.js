import assert from 'node:assert/strict';
import test from 'node:test';

import {
  createSetupSession,
  getEffectiveMonitorName,
  recordReading,
  setDeviceName,
  setDisplays,
  setMonitorName,
  setPresetLabel,
  setReadingInputLabel,
} from '../src/lib/setup-session.js';

test('setDisplays preserves existing monitor names across re-detect', () => {
  let session = createSetupSession();
  session = setDisplays(session, [{ displayId: 'K@P:d0e5:0', label: 'Display 1' }]);
  session = setMonitorName(session, 'K@P:d0e5:0', 'Center 1440p');
  session = setDisplays(session, [{ displayId: 'K@P:d0e5:0', label: 'Display 1' }]);

  assert.equal(session.displays?.[0]?.name, 'Center 1440p');
});

test('recordReading stores input label defaulting to device name', () => {
  let session = createSetupSession();
  session = setDeviceName(session, 'Gaming PC');
  session = recordReading(session, 'K@P:d0e5:0', 'Display 1', {
    current: 4626,
    maximum: 4626,
  });

  assert.equal(session.readings?.[0]?.inputLabel, 'Gaming PC');
});

test('setReadingInputLabel updates captured input label', () => {
  let session = createSetupSession();
  session = setDeviceName(session, 'Gaming PC');
  session = recordReading(session, 'K@P:d0e5:0', 'Display 1', {
    current: 4626,
    maximum: 4626,
  });
  session = setReadingInputLabel(session, 'K@P:d0e5:0', 'Desktop');

  assert.equal(session.readings?.[0]?.inputLabel, 'Desktop');
});

test('setPresetLabel stores preset label', () => {
  let session = createSetupSession();
  session = setPresetLabel(session, 'All Gaming PC');
  assert.equal(session.presetLabel, 'All Gaming PC');
});

test('getEffectiveMonitorName falls back to numbered label', () => {
  assert.equal(getEffectiveMonitorName(undefined, 1), 'Monitor 2');
  assert.equal(getEffectiveMonitorName({ displayId: 'x', label: 'y', name: 'Left monitor' }, 0), 'Left monitor');
});
