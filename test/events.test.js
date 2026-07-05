import assert from 'node:assert/strict';
import test from 'node:test';

import {
  eventKindToBadgeClass,
  formatEventKindLabel,
  formatEventMeta,
  formatEventTimestamp,
} from '../src/lib/events.js';

test('formatEventTimestamp formats millis to locale time string', () => {
  const label = formatEventTimestamp(Date.UTC(2026, 6, 5, 12, 30, 0));
  assert.ok(typeof label === 'string');
  assert.ok(label.length > 0);
});

test('formatEventKindLabel maps kinds', () => {
  assert.equal(formatEventKindLabel('info'), 'Info');
  assert.equal(formatEventKindLabel('success'), 'Success');
  assert.equal(formatEventKindLabel('error'), 'Error');
});

test('eventKindToBadgeClass maps kinds to badge classes', () => {
  assert.equal(eventKindToBadgeClass('success'), 'badge badge-ok');
  assert.equal(eventKindToBadgeClass('error'), 'badge badge-error');
  assert.equal(eventKindToBadgeClass('info'), 'badge badge-info');
});

test('formatEventMeta joins preset source and monitor', () => {
  assert.equal(
    formatEventMeta({
      timestampMs: 1,
      kind: 'info',
      message: 'test',
      preset: 'all_windows',
      source: 'api',
      monitorId: 'monitor1',
    }),
    'preset: all_windows · via api · monitor: monitor1',
  );
});
