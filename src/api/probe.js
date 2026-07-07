import { tauriInvoke } from '../lib/tauri.js';
import { PROBE_DESKTOP_ONLY } from '../lib/probe.js';

/**
 * Setup-time test switch: writes `value` to VCP 0x60 on `displayId`. IPC-only — the backend
 * only allows a value that a prior read has already returned as this display's current input;
 * see docs/NATIVE_DDC_DISCOVERY.md.
 *
 * @param {string} displayId
 * @param {number} value
 * @returns {Promise<{ accepted: boolean, displayId: string, value: number, current?: number }>}
 */
export async function probeInput(displayId, value) {
  const invoke = tauriInvoke();
  if (!invoke) {
    throw new Error(PROBE_DESKTOP_ONLY);
  }
  return /** @type {Promise<{ accepted: boolean, displayId: string, value: number, current?: number }>} */ (
    invoke('probe_input', { displayId, value })
  );
}
