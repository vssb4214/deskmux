/** @typedef {import('../lib/setup-status.js').SetupSession} SetupSession */

/**
 * @returns {SetupSession}
 */
export function createSetupSession() {
  return {
    started: false,
    deviceName: '',
    displays: [],
    readings: [],
    saveSucceeded: false,
    generatedDraft: false,
  };
}

/**
 * @param {SetupSession} session
 * @returns {SetupSession}
 */
export function markSetupStarted(session) {
  return { ...session, started: true };
}

/**
 * @param {SetupSession} session
 * @param {string} name
 * @returns {SetupSession}
 */
export function setDeviceName(session, name) {
  return { ...session, started: true, deviceName: name.trim() };
}

/**
 * @param {SetupSession} session
 * @param {Array<{ displayId: string, label: string }>} displays
 * @returns {SetupSession}
 */
export function setDisplays(session, displays) {
  return { ...session, started: true, displays };
}

/**
 * @param {SetupSession} session
 * @param {string} displayId
 * @param {string} label
 * @param {{ current: number, maximum: number }} reading
 * @returns {SetupSession}
 */
export function recordReading(session, displayId, label, reading) {
  const readings = [
    ...(session.readings ?? []).filter((entry) => entry.displayId !== displayId),
    {
      displayId,
      label,
      current: reading.current,
      maximum: reading.maximum,
    },
  ];
  return { ...session, started: true, readings };
}

/**
 * @param {SetupSession} session
 * @returns {SetupSession}
 */
export function markDraftGenerated(session) {
  return { ...session, generatedDraft: true, started: true };
}

/**
 * @param {SetupSession} session
 * @returns {SetupSession}
 */
export function markSaveSucceeded(session) {
  return { ...session, saveSucceeded: true };
}
