/** @typedef {import('../types.js').StatusResponse} StatusResponse */
/** @typedef {import('../types.js').ApplyPresetResponse} ApplyPresetResponse */
/** @typedef {import('../types.js').HealthResponse} HealthResponse */

import { parseApiError } from '../lib/config-error.js';

/**
 * @param {string} baseUrl
 */
export function createApiClient(baseUrl) {
  const normalized = baseUrl.replace(/\/$/, '');

  /**
   * @param {string} path
   */
  async function requestJson(path, init) {
    const response = await fetch(`${normalized}${path}`, init);
    const body = await response.json().catch(() => ({}));
    if (!response.ok) {
      throw parseApiError(body, response.status);
    }
    return body;
  }

  return {
    baseUrl: normalized,

    /** @returns {Promise<HealthResponse>} */
    async fetchHealth() {
      return requestJson('/health');
    },

    /** @returns {Promise<StatusResponse>} */
    async fetchStatus() {
      return requestJson('/status');
    },

    /**
     * @param {string} preset
     * @param {boolean} dryRun
     * @returns {Promise<ApplyPresetResponse>}
     */
    async applyPreset(preset, dryRun) {
      return requestJson('/apply-preset', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ preset, dryRun, localOnly: false }),
      });
    },
  };
}
