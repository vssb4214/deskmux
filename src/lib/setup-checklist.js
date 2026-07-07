/** @typedef {import('./setup-status.js').SetupStatus} SetupStatus */
/** @typedef {import('./setup-session.js').SetupSession} SetupSession */

/** @typedef {'complete' | 'current' | 'pending' | 'blocked'} StepState */

/**
 * @typedef {{
 *   id: string,
 *   label: string,
 *   state: StepState,
 *   helper: string,
 * }} SetupStepView
 */

export const SETUP_STEP_IDS = [
  'name',
  'detect',
  'capture',
  'generate',
  'save',
  'restart',
  'test',
];

/**
 * @param {SetupSession} session
 * @param {SetupStatus} status
 * @param {{ isDesktop: boolean, nativeAvailable: boolean }} options
 * @returns {SetupStepView[]}
 */
export function buildSetupChecklist(session, status, options) {
  const hasName = Boolean(session.deviceName?.trim());
  const hasDisplays = (session.displays?.length ?? 0) > 0;
  const hasReadings = (session.readings?.length ?? 0) > 0;
  const hasDraft = Boolean(session.generatedDraft);
  const saved = Boolean(session.saveSucceeded);
  const ready = status === 'ready';

  const states = {
    name: hasName ? 'complete' : 'current',
    detect: !hasName
      ? 'blocked'
      : hasDisplays
        ? 'complete'
        : hasName
          ? 'current'
          : 'pending',
    capture: !hasDisplays
      ? 'blocked'
      : hasReadings
        ? 'complete'
        : hasDisplays
          ? 'current'
          : 'pending',
    generate: !hasReadings
      ? 'blocked'
      : hasDraft
        ? 'complete'
        : hasReadings
          ? 'current'
          : 'pending',
    save: !hasDraft
      ? 'blocked'
      : saved
        ? 'complete'
        : hasDraft
          ? 'current'
          : 'pending',
    restart: !saved ? 'blocked' : ready ? 'complete' : 'current',
    test: !ready ? 'blocked' : 'current',
  };

  const helpers = {
    name: 'Choose a short label for this computer in your desk setup.',
    detect: options.nativeAvailable
      ? 'Find monitors DeskMux can control with native DDC on this PC.'
      : 'Native display detection is Windows-only. Use Advanced JSON for shell-command setup.',
    capture:
      'Switch each monitor to the input you want to capture, then click Read current input.',
    generate: 'Build a config draft from the values you captured.',
    save: options.isDesktop
      ? 'Validate the draft, then save deskmux.config.json from the desktop app.'
      : 'Saving is only available in the DeskMux desktop app.',
    restart: 'Quit DeskMux completely and open it again so the new config loads.',
    test: 'Run a dry-run preset to confirm everything looks right.',
  };

  const labels = {
    name: 'Name this computer',
    detect: 'Detect monitors',
    capture: 'Capture current input values',
    generate: 'Generate config draft',
    save: 'Validate and save',
    restart: 'Restart DeskMux',
    test: 'Test a preset',
  };

  return SETUP_STEP_IDS.map((id) => ({
    id,
    label: labels[id],
    state: /** @type {StepState} */ (states[id]),
    helper: helpers[id],
  }));
}

/**
 * @param {StepState} state
 * @returns {string}
 */
export function stepStateLabel(state) {
  switch (state) {
    case 'complete':
      return 'Done';
    case 'current':
      return 'Current step';
    case 'blocked':
      return 'Waiting';
    default:
      return 'Pending';
  }
}
