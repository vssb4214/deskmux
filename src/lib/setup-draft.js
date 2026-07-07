/** @typedef {import('./setup-session.js').SetupSession} SetupSession */

/**
 * @param {string} name
 * @returns {string}
 */
export function slugifyDeviceId(name) {
  const slug = name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
  return slug || 'my-pc';
}

/**
 * @param {SetupSession} session
 * @returns {{ ok: true, json: string } | { ok: false, errors: string[] }}
 */
export function buildConfigDraftFromSetupState(session) {
  /** @type {string[]} */
  const errors = [];

  if (!session.deviceName?.trim()) {
    errors.push('Enter a name for this computer.');
  }
  if (!session.readings?.length) {
    errors.push('Capture at least one input value from a monitor.');
  }

  if (errors.length > 0) {
    return { ok: false, errors };
  }

  const deviceId = slugifyDeviceId(session.deviceName);
  const deviceLabel = session.deviceName.trim();
  const presetName = `all_${deviceId.replace(/-/g, '_')}`;

  const monitors = session.readings.map((reading, index) => ({
    id: `monitor${index + 1}`,
    label: reading.label || `Monitor ${index + 1}`,
    order: index,
    nativeDdc: { displayId: reading.displayId },
    inputs: {
      [deviceId]: {
        type: 'displayport',
        nativeDdc: { inputSourceValue: reading.current },
      },
    },
  }));

  /** @type {Record<string, string>} */
  const layout = {};
  for (const monitor of monitors) {
    layout[monitor.id] = deviceId;
  }

  const config = {
    deviceName: deviceId,
    apiPort: 3737,
    apiLanAccess: false,
    peers: [],
    devices: [{ id: deviceId, label: deviceLabel }],
    monitors,
    presets: {
      [presetName]: {
        label: `All ${deviceLabel}`,
        layout,
      },
    },
  };

  return { ok: true, json: JSON.stringify(config, null, 2) };
}
