import { tauriInvoke } from '../lib/tauri.js';

/**
 * @param {string} json
 */
export async function validateConfigDraft(json) {
  const invoke = tauriInvoke();
  if (!invoke) {
    throw new Error('Config draft validation is only available in the DeskMux desktop app.');
  }
  return invoke('validate_config_draft', { json });
}

/**
 * @param {string} json
 * @returns {Promise<{ filename: string, backupCreated: boolean, restartRequired: boolean }>}
 */
export async function saveConfigDraft(json) {
  const invoke = tauriInvoke();
  if (!invoke) {
    throw new Error('Config save is only available in the DeskMux desktop app.');
  }
  return /** @type {Promise<{ filename: string, backupCreated: boolean, restartRequired: boolean }>} */ (
    invoke('save_config_draft', { json })
  );
}
