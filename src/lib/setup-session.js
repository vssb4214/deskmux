/** @typedef {import('./setup-status.js').SetupSession} SetupSession */

/**
 * @returns {SetupSession}
 */
export function createSetupSession() {
  return {
    started: false,
    deviceName: '',
    displays: [],
    readings: [],
    presetLabel: '',
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
 * @param {Array<{ displayId: string, label: string, name?: string }>} displays
 * @returns {SetupSession}
 */
export function setDisplays(session, displays) {
  const previousNames = new Map(
    (session.displays ?? []).map((display) => [display.displayId, display.name ?? '']),
  );

  const merged = displays.map((display) => ({
    ...display,
    name: previousNames.get(display.displayId) ?? display.name ?? '',
  }));

  return { ...session, started: true, displays: merged };
}

/**
 * @param {SetupSession} session
 * @param {string} displayId
 * @param {string} name
 * @returns {SetupSession}
 */
export function setMonitorName(session, displayId, name) {
  const displays = (session.displays ?? []).map((display) =>
    display.displayId === displayId ? { ...display, name: name.trim() } : display,
  );
  return { ...session, started: true, displays };
}

/**
 * @param {SetupSession} session
 * @param {string} displayId
 * @param {string} label
 * @param {{ current: number, maximum: number }} reading
 * @param {string} [inputLabel]
 * @returns {SetupSession}
 */
export function recordReading(session, displayId, label, reading, inputLabel) {
  const defaultInputLabel = session.deviceName?.trim() ?? '';
  const existing = (session.readings ?? []).find((entry) => entry.displayId === displayId);
  const readings = [
    ...(session.readings ?? []).filter((entry) => entry.displayId !== displayId),
    {
      displayId,
      label,
      current: reading.current,
      maximum: reading.maximum,
      inputLabel: inputLabel ?? existing?.inputLabel ?? defaultInputLabel,
    },
  ];
  return { ...session, started: true, readings };
}

/**
 * @param {SetupSession} session
 * @param {string} displayId
 * @param {string} inputLabel
 * @returns {SetupSession}
 */
export function setReadingInputLabel(session, displayId, inputLabel) {
  const readings = (session.readings ?? []).map((entry) =>
    entry.displayId === displayId ? { ...entry, inputLabel: inputLabel.trim() } : entry,
  );
  return { ...session, started: true, readings };
}

/**
 * @param {SetupSession} session
 * @param {string} label
 * @returns {SetupSession}
 */
export function setPresetLabel(session, label) {
  return { ...session, started: true, presetLabel: label.trim() };
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

/**
 * @param {SetupSession['displays'][number] | undefined} display
 * @param {number} index
 * @returns {string}
 */
export function getEffectiveMonitorName(display, index) {
  if (display?.name?.trim()) {
    return display.name.trim();
  }
  return `Monitor ${index + 1}`;
}
