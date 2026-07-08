/** @typedef {import('../types.js').InputSourceResponse} InputSourceResponse */
/** @typedef {import('../types.js').NativeDdcControlFeature} NativeDdcControlFeature */
/** @typedef {import('../types.js').NativeDdcControlState} NativeDdcControlState */

export const DISCOVERY_UNAVAILABLE_MESSAGE =
  'Native display detection is Windows-only. On this platform, configure monitors with ' +
  'shell commands instead (ddcutil, BetterDisplay, Lunar…) — see docs/CONFIG.md in the ' +
  'DeskMux repository.';

export const DISCOVERY_INSTRUCTIONS =
  'Each physical input has its own value. Switch the monitor to another input using its ' +
  'buttons, then read again — note the value shown for each input you care about.';

export const DISCOVERY_EMPTY_MESSAGE =
  'No DDC-capable displays detected. Check the monitor is connected directly (not through ' +
  'some hubs/adapters) and supports DDC/CI.';

/**
 * @param {number} index
 * @param {string} displayId
 * @returns {string}
 */
export function formatDisplayLabel(index, displayId) {
  return `Display ${index + 1} — ${displayId}`;
}

/**
 * @param {InputSourceResponse} reading
 * @returns {string}
 */
export function formatInputSourceReading(reading) {
  return `Current input value: ${reading.current} (reported max ${reading.maximum})`;
}

/** @type {NativeDdcControlFeature[]} */
export const NATIVE_DDC_CONTROL_FEATURES = ['brightness', 'contrast', 'volume'];

/** @param {NativeDdcControlFeature} feature */
export function nativeDdcControlLabel(feature) {
  switch (feature) {
    case 'brightness':
      return 'Brightness';
    case 'contrast':
      return 'Contrast';
    case 'volume':
      return 'Volume';
    default:
      return 'Control';
  }
}

/**
 * @param {NativeDdcControlState} control
 * @returns {string}
 */
export function formatNativeDdcControlValue(control) {
  if (!control.available) {
    return 'Not supported by this monitor';
  }
  return `Current ${control.current} of ${control.maximum}`;
}

/**
 * @param {unknown} err
 * @returns {string}
 */
export function nativeDdcControlErrorMessage(err) {
  if (err && typeof err === 'object' && 'error' in err) {
    const message = /** @type {{ error?: unknown }} */ (err).error;
    if (typeof message === 'string' && message) {
      return message;
    }
  }
  if (err instanceof Error && err.message) {
    return err.message;
  }
  return 'Native DDC control failed.';
}

/**
 * @param {unknown} err
 * @returns {string}
 */
export function discoveryErrorMessage(err) {
  if (err instanceof Error && err.message) {
    return `Read failed: ${err.message}`;
  }
  return 'Read failed: unknown error';
}
