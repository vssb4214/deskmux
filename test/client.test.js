import assert from 'node:assert/strict';
import test from 'node:test';

import { createApiClient } from '../src/api/client.js';

test('fetchNativeDdcControls calls the encoded controls endpoint', async () => {
  const previousFetch = globalThis.fetch;
  /** @type {string[]} */
  const urls = [];
  try {
    globalThis.fetch = async (url) => {
      urls.push(String(url));
      return {
        ok: true,
        json: async () => ({
          displayId: 'K@P:d0e5:0',
          controls: {
            brightness: { available: true, current: 70, maximum: 100 },
            contrast: { available: true, current: 50, maximum: 100 },
            volume: { available: false, error: 'vcpReadFailed' },
          },
        }),
      };
    };

    const client = createApiClient('http://127.0.0.1:3737/');
    const result = await client.fetchNativeDdcControls('K@P:d0e5:0');

    assert.deepEqual(urls, [
      'http://127.0.0.1:3737/native-ddc/displays/K%40P%3Ad0e5%3A0/controls',
    ]);
    assert.equal(result.controls.brightness.current, 70);
  } finally {
    globalThis.fetch = previousFetch;
  }
});
