import assert from 'node:assert/strict';
import test from 'node:test';

import {
  defaultPresetLabel,
  ensureUniqueIds,
  makeDeviceId,
  makeMonitorId,
  makePresetId,
  monitorLabelFallback,
  slugifyId,
} from '../src/lib/setup-names.js';

test('slugifyId normalizes labels into underscore ids', () => {
  assert.equal(slugifyId('Gaming PC'), 'gaming_pc');
  assert.equal(slugifyId('Center 1440p'), 'center_1440p');
});

test('slugifyId uses fallback for blank labels', () => {
  assert.equal(slugifyId('   ', 'monitor_1'), 'monitor_1');
  assert.equal(slugifyId('!!!', 'safe_id'), 'safe_id');
});

test('slugifyId prefixes numeric-leading ids with underscore', () => {
  assert.equal(slugifyId('1440p center'), '_1440p_center');
});

test('makeDeviceId slugifies computer labels', () => {
  assert.equal(makeDeviceId('Gaming PC'), 'gaming_pc');
});

test('makeMonitorId uses index fallback when label is blank', () => {
  assert.equal(makeMonitorId('', 0), 'monitor_1');
  assert.equal(makeMonitorId('   ', 1), 'monitor_2');
});

test('makeMonitorId slugifies readable monitor names', () => {
  assert.equal(makeMonitorId('Left monitor', 0), 'left_monitor');
  assert.equal(makeMonitorId('Center 1440p', 0), 'center_1440p');
});

test('ensureUniqueIds adds suffixes for duplicate labels', () => {
  const ids = ensureUniqueIds(['monitor', 'monitor', 'monitor'], (label, index) =>
    makeMonitorId(label, index),
  );
  assert.deepEqual(ids, ['monitor', 'monitor_2', 'monitor_3']);
});

test('ensureUniqueIds handles weird punctuation without empty ids', () => {
  const ids = ensureUniqueIds(['!!!', '???'], (label, index) => slugifyId(label, `item_${index + 1}`));
  assert.equal(ids[0], 'item_1');
  assert.equal(ids[1], 'item_2');
});

test('defaultPresetLabel prefixes All', () => {
  assert.equal(defaultPresetLabel('Gaming PC'), 'All Gaming PC');
});

test('makePresetId slugifies preset labels', () => {
  assert.equal(makePresetId('All Gaming PC'), 'all_gaming_pc');
});

test('monitorLabelFallback numbers monitors from one', () => {
  assert.equal(monitorLabelFallback(0), 'Monitor 1');
  assert.equal(monitorLabelFallback(2), 'Monitor 3');
});
