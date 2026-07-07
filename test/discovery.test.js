import assert from 'node:assert/strict';
import test from 'node:test';

import {
  DISCOVERY_EMPTY_MESSAGE,
  DISCOVERY_INSTRUCTIONS,
  DISCOVERY_UNAVAILABLE_MESSAGE,
  discoveryErrorMessage,
  formatDisplayLabel,
  formatInputSourceReading,
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
