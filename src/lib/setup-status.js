/** @typedef {'needsSetup' | 'inProgress' | 'restartRequired' | 'ready'} SetupStatus */

/**
 * @typedef {{
 *   started?: boolean,
 *   deviceName?: string,
 *   displays?: Array<{ displayId: string, label: string, name?: string }>,
 *   readings?: Array<{
 *     displayId: string,
 *     label: string,
 *     current: number,
 *     maximum: number,
 *     inputLabel?: string,
 *   }>,
 *   presetLabel?: string,
 *   saveSucceeded?: boolean,
 *   generatedDraft?: boolean,
 * }} SetupSession
 */

/**
 * @param {{ configLoaded: boolean, configError?: string }} health
 * @param {SetupSession} session
 * @returns {SetupStatus}
 */
export function deriveSetupStatus(health, session) {
  if (health.configLoaded) {
    return 'ready';
  }
  if (session.saveSucceeded) {
    return 'restartRequired';
  }
  if (
    session.started ||
    session.deviceName?.trim() ||
    (session.displays?.length ?? 0) > 0 ||
    (session.readings?.length ?? 0) > 0
  ) {
    return 'inProgress';
  }
  return 'needsSetup';
}

/**
 * @param {SetupStatus} status
 * @param {{ configError?: string }} [health]
 * @returns {{ badge: string, message: string, cta: string | null }}
 */
export function getSetupStatusCopy(status, health = {}) {
  switch (status) {
    case 'ready':
      return {
        badge: 'Ready',
        message: 'DeskMux is configured. Choose a preset and apply it below.',
        cta: null,
      };
    case 'restartRequired':
      return {
        badge: 'Restart required',
        message:
          'Configuration saved. Quit and reopen DeskMux so the new config loads.',
        cta: null,
      };
    case 'inProgress':
      return {
        badge: 'Setup in progress',
        message: 'Continue the setup checklist below.',
        cta: null,
      };
    case 'needsSetup':
    default:
      return {
        badge: 'Setup required',
        message: health.configError
          ? 'DeskMux could not load your configuration. Use the setup checklist to create one.'
          : 'DeskMux is not configured yet. Use the setup checklist to get started.',
        cta: 'Start setup',
      };
  }
}

/**
 * @param {SetupStatus} status
 * @param {boolean} configLoaded
 * @returns {{ mode: 'expanded' | 'collapsed' | 'hidden', summary: string }}
 */
export function getSetupChecklistPresentation(status, configLoaded) {
  if (configLoaded && status === 'ready') {
    return { mode: 'collapsed', summary: 'Run setup again' };
  }
  if (!configLoaded) {
    return { mode: 'expanded', summary: 'Set up DeskMux' };
  }
  return { mode: 'hidden', summary: '' };
}

/**
 * @param {SetupStatus} status
 * @returns {string}
 */
export function setupStatusBadgeClass(status) {
  switch (status) {
    case 'ready':
      return 'badge badge-ok setup-status-badge';
    case 'restartRequired':
      return 'badge badge-info setup-status-badge';
    case 'inProgress':
      return 'badge badge-info setup-status-badge';
    case 'needsSetup':
    default:
      return 'badge badge-error setup-status-badge';
  }
}
