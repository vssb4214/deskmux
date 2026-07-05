import assert from 'node:assert/strict';
import test from 'node:test';

import {
  CONFIG_FILE_HINT,
  configErrorFromUnknown,
  formatConfigErrorBannerText,
  parseApiError,
} from '../src/lib/config-error.js';

test('formatConfigErrorBannerText includes title, detail, and hint', () => {
  const text = formatConfigErrorBannerText('deviceName missing from devices[]');
  assert.match(text, /^Config not loaded/);
  assert.match(text, /deviceName missing from devices\[\]/);
  assert.match(text, /deskmux\.config\.json/);
  assert.match(text, new RegExp(CONFIG_FILE_HINT.replaceAll('.', '\\.')));
});

test('formatConfigErrorBannerText preserves multiline validation errors', () => {
  const text = formatConfigErrorBannerText(
    'config is invalid:\n  - duplicate device id',
  );
  assert.match(text, /config is invalid:\n  - duplicate device id/);
});

test('parseApiError attaches configError from 503 body', () => {
  const err = parseApiError(
    { error: 'config not loaded', configError: 'failed to read config file' },
    503,
  );
  assert.equal(err.message, 'config not loaded');
  assert.equal(err.status, 503);
  assert.equal(err.configError, 'failed to read config file');
});

test('parseApiError omits configError when absent', () => {
  const err = parseApiError({ error: 'preset name is required' }, 400);
  assert.equal(err.message, 'preset name is required');
  assert.equal(err.configError, undefined);
});

test('configErrorFromUnknown reads configError property', () => {
  const err = parseApiError(
    { error: 'config not loaded', configError: 'parse failed' },
    503,
  );
  assert.equal(configErrorFromUnknown(err), 'parse failed');
  assert.equal(configErrorFromUnknown(new Error('nope')), undefined);
});
