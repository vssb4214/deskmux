import { tauriInvoke } from '../lib/tauri.js';

export const NATIVE_CONTROLS_DESKTOP_ONLY =
  'Live native DDC control writes are available only in the DeskMux desktop app.';

/**
 * @param {string} displayId
 * @param {import('../types.js').NativeDdcControlFeature} feature
 * @param {number} value
 * @returns {Promise<import('../types.js').SetNativeDdcControlResponse>}
 */
export async function setNativeDdcControl(displayId, feature, value) {
  const invoke = tauriInvoke();
  if (!invoke) {
    throw new Error(NATIVE_CONTROLS_DESKTOP_ONLY);
  }
  return /** @type {Promise<import('../types.js').SetNativeDdcControlResponse>} */ (
    invoke('set_native_ddc_control', { displayId, feature, value })
  );
}
