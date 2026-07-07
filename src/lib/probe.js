export const PROBE_DESKTOP_ONLY =
  'Testing an input is only available from the DeskMux desktop app.';

/** How long an unconfirmed test switch is left in place before auto-reverting. */
export const PROBE_REVERT_DELAY_MS = 8000;

/**
 * @param {number} value
 * @returns {string}
 */
export function formatProbeCountdownMessage(value) {
  return `Testing input value ${value}. If the screen changed as expected, click Keep — otherwise it reverts automatically in a few seconds.`;
}

/**
 * @param {unknown} err
 * @returns {string}
 */
export function probeErrorMessage(err) {
  if (err && typeof err === 'object' && typeof (/** @type {any} */ (err).error) === 'string') {
    return /** @type {{ error: string }} */ (err).error;
  }
  if (err instanceof Error && err.message) {
    return err.message;
  }
  return 'Testing this input failed.';
}

/**
 * @typedef {{
 *   schedule: (fn: () => void, delayMs: number) => unknown,
 *   clear: (id: unknown) => void,
 *   onRevert: () => void,
 *   delayMs: number,
 * }} ProbeRevertOptions
 */

/**
 * Starts a revert-on-timeout window for a test switch: `onRevert` fires automatically after
 * `delayMs` unless `confirm()` runs first; `revertNow()` fires it immediately. Idempotent once
 * settled — a stray double-click on Keep/Revert can't double-fire or revert after confirm.
 *
 * No real timer lives here — the caller supplies `schedule`/`clear` (`setTimeout`/`clearTimeout`
 * in production), so the revert logic itself is testable without real time passing.
 *
 * @param {ProbeRevertOptions} options
 * @returns {{ confirm: () => void, revertNow: () => void }}
 */
export function startProbeRevertTimer({ schedule, clear, onRevert, delayMs }) {
  let settled = false;

  const timerId = schedule(() => {
    if (settled) {
      return;
    }
    settled = true;
    onRevert();
  }, delayMs);

  return {
    confirm() {
      if (settled) {
        return;
      }
      settled = true;
      clear(timerId);
    },
    revertNow() {
      if (settled) {
        return;
      }
      settled = true;
      clear(timerId);
      onRevert();
    },
  };
}
