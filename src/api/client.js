/** @typedef {import('../types.js').StatusResponse} StatusResponse */
/** @typedef {import('../types.js').ApplyPresetResponse} ApplyPresetResponse */
/** @typedef {import('../types.js').EventsResponse} EventsResponse */
/** @typedef {import('../types.js').DeskMuxEvent} DeskMuxEvent */
/** @typedef {import('../types.js').DiscoveryDisplaysResponse} DiscoveryDisplaysResponse */
/** @typedef {import('../types.js').InputSourceResponse} InputSourceResponse */
/** @typedef {import('../types.js').ProbeInputResponse} ProbeInputResponse */

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

    /** @returns {Promise<EventsResponse>} */
    async fetchEvents() {
      return requestJson('/events');
    },

    /** @returns {Promise<DiscoveryDisplaysResponse>} */
    async fetchDiscoveryDisplays() {
      return requestJson('/native-ddc/displays');
    },

    /**
     * @param {string} displayId
     * @returns {Promise<InputSourceResponse>}
     */
    async fetchInputSource(displayId) {
      return requestJson(
        `/native-ddc/displays/${encodeURIComponent(displayId)}/input-source`,
      );
    },

    /**
     * Setup-time probe write for native DDC VCP 0x60.
     *
     * @param {string} displayId
     * @param {number} value
     * @returns {Promise<ProbeInputResponse>}
     */
    async probeInput(displayId, value) {
      return requestJson(
        `/native-ddc/displays/${encodeURIComponent(displayId)}/probe-input`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ value }),
        },
      );
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
