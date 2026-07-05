/** @typedef {import('../types.js').StatusResponse} StatusResponse */
/** @typedef {import('../types.js').ApplyPresetResponse} ApplyPresetResponse */

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
      const message =
        typeof body.error === 'string' ? body.error : response.statusText;
      const error = new Error(message);
      error.status = response.status;
      throw error;
    }
    return body;
  }

  return {
    baseUrl: normalized,

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
