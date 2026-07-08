import assert from 'node:assert/strict';
import test from 'node:test';

import {
  DISCOVERY_EMPTY_MESSAGE,
  DISCOVERY_INSTRUCTIONS,
  DISCOVERY_UNAVAILABLE_MESSAGE,
  NATIVE_DDC_CONTROL_FEATURES,
  discoveryErrorMessage,
  formatDisplayLabel,
  formatInputSourceReading,
  formatNativeDdcControlValue,
  nativeDdcControlErrorMessage,
  nativeDdcControlLabel,
} from '../src/lib/discovery.js';

test('formatDisplayLabel numbers displays from 1 and shows the raw displayId', () => {
  assert.equal(formatDisplayLabel(0, 'K@P:d0e5:0'), 'Display 1 — K@P:d0e5:0');
  assert.equal(formatDisplayLabel(2, 'KJL:0e25:2'), 'Display 3 — KJL:0e25:2');
});

test('formatInputSourceReading shows current and reported max', () => {
  assert.equal(
    formatInputSourceReading({ current: 4626, maximum: 4626 }),
    'Current input value: 4626 (reported max 4626)',
  );
});

test('formatInputSourceReading handles values above 255', () => {
  const text = formatInputSourceReading({ current: 4623, maximum: 4626 });
  assert.ok(text.includes('4623'));
  assert.ok(text.includes('4626'));
});

test('discoveryErrorMessage uses the API error message when present', () => {
  const err = new Error("display 'GHOST:0000:0' not found");
  assert.equal(
    discoveryErrorMessage(err),
    "Read failed: display 'GHOST:0000:0' not found",
  );
});

test('discoveryErrorMessage falls back on non-Error values', () => {
  assert.equal(discoveryErrorMessage(undefined), 'Read failed: unknown error');
  assert.equal(discoveryErrorMessage('boom'), 'Read failed: unknown error');
});

test('static discovery copy is non-empty and mentions the key concepts', () => {
  assert.ok(DISCOVERY_UNAVAILABLE_MESSAGE.includes('Windows'));
  assert.ok(DISCOVERY_UNAVAILABLE_MESSAGE.includes('shell command'));
  assert.ok(DISCOVERY_INSTRUCTIONS.includes('read again'));
  assert.ok(DISCOVERY_EMPTY_MESSAGE.includes('DDC'));
});

test('native DDC control features are the supported live controls only', () => {
  assert.deepEqual(NATIVE_DDC_CONTROL_FEATURES, ['brightness', 'contrast', 'volume']);
});

test('nativeDdcControlLabel formats supported feature names', () => {
  assert.equal(nativeDdcControlLabel('brightness'), 'Brightness');
  assert.equal(nativeDdcControlLabel('contrast'), 'Contrast');
  assert.equal(nativeDdcControlLabel('volume'), 'Volume');
});

test('formatNativeDdcControlValue formats available and unavailable controls', () => {
  assert.equal(
    formatNativeDdcControlValue({ available: true, current: 70, maximum: 100 }),
    'Current 70 of 100',
  );
  assert.equal(
    formatNativeDdcControlValue({ available: false, error: 'vcpReadFailed' }),
    'Not supported by this monitor',
  );
});

test('nativeDdcControlErrorMessage extracts structured backend errors', () => {
  assert.equal(
    nativeDdcControlErrorMessage({ error: 'value 101 exceeds maximum 100' }),
    'value 101 exceeds maximum 100',
  );
  assert.equal(nativeDdcControlErrorMessage(new Error('boom')), 'boom');
  assert.equal(nativeDdcControlErrorMessage(undefined), 'Native DDC control failed.');
});
