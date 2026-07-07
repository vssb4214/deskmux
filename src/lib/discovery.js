/** @typedef {import('../types.js').InputSourceResponse} InputSourceResponse */

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
