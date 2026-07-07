/** @typedef {import('./setup-session.js').SetupSession} SetupSession */

import {
  defaultPresetLabel,
  ensureUniqueIds,
  makeDeviceId,
  makeMonitorId,
  makePresetId,
} from './setup-names.js';
import { getEffectiveMonitorName } from './setup-session.js';

/**
 * @param {SetupSession} session
 * @returns {{ deviceLabel: string, monitorLabels: string[], presetLabel: string }}
 */
export function buildDraftSummary(session) {
  const deviceLabel = session.deviceName?.trim() || 'this computer';
  const presetLabel = session.presetLabel?.trim() || defaultPresetLabel(deviceLabel);

  const monitorLabels = (session.readings ?? []).map((reading, index) => {
    const displayIndex =
      session.displays?.findIndex((display) => display.displayId === reading.displayId) ?? index;
    const display =
      displayIndex >= 0 ? session.displays?.[displayIndex] : session.displays?.[index];
    return getEffectiveMonitorName(display, displayIndex >= 0 ? displayIndex : index);
  });

  return { deviceLabel, monitorLabels, presetLabel };
}

/**
 * @param {SetupSession} session
 * @returns {{ ok: true, json: string, summary: ReturnType<typeof buildDraftSummary> } | { ok: false, errors: string[] }}
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

  const deviceLabel = session.deviceName.trim();
  const deviceId = makeDeviceId(deviceLabel);
  const presetLabel = session.presetLabel?.trim() || defaultPresetLabel(deviceLabel);
  const presetId = makePresetId(presetLabel);
  const summary = buildDraftSummary(session);

  const monitorLabels = summary.monitorLabels;
  const monitorIds = ensureUniqueIds(monitorLabels, (label, index) => makeMonitorId(label, index));

  const monitors = session.readings.map((reading, index) => ({
    id: monitorIds[index],
    label: monitorLabels[index],
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
      [presetId]: {
        label: presetLabel,
        layout,
      },
    },
  };

  return {
    ok: true,
    json: JSON.stringify(config, null, 2),
    summary,
  };
}
