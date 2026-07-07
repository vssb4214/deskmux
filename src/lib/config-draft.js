export const CONFIG_DRAFT_DESKTOP_ONLY =
  'Config save is only available from the DeskMux desktop app.';

export const CONFIG_DRAFT_SUCCESS_MESSAGE =
  'Saved to deskmux.config.json. Restart DeskMux for this configuration to take effect.';

/** Valid minimal skeleton when no live config JSON is available from the API. */
export function defaultConfigDraftSkeleton() {
  return JSON.stringify(
    {
      deviceName: 'my-pc',
      apiPort: 3737,
      apiLanAccess: false,
      peers: [],
      devices: [{ id: 'my-pc', label: 'My PC' }],
      monitors: [
        {
          id: 'monitor1',
          label: 'Monitor 1',
          order: 0,
          inputs: {
            'my-pc': { type: 'displayport', command: 'your-command-here' },
          },
        },
      ],
      presets: {
        default: { label: 'Default', layout: { monitor1: 'my-pc' } },
      },
    },
    null,
    2,
  );
}

/**
 * @param {unknown} err
 * @returns {Record<string, unknown> | null}
 */
export function normalizeDraftError(err) {
  if (err && typeof err === 'object' && 'type' in err) {
    return /** @type {Record<string, unknown>} */ (err);
  }

  const text = err instanceof Error ? err.message : typeof err === 'string' ? err : '';
  if (!text) {
    return null;
  }

  try {
    const parsed = JSON.parse(text);
    if (parsed && typeof parsed === 'object') {
      return /** @type {Record<string, unknown>} */ (parsed);
    }
  } catch {
    // fall through
  }

  return { type: 'parse', message: text };
}

/**
 * @param {unknown} err
 * @returns {string[]}
 */
export function draftErrorsFromInvoke(err) {
  const payload = normalizeDraftError(err);
  if (!payload) {
    return ['Unknown validation error.'];
  }

  switch (payload.type) {
    case 'io':
    case 'parse':
      return [typeof payload.message === 'string' ? payload.message : 'Request failed.'];
    case 'invalid': {
      const errors = Array.isArray(payload.errors) ? payload.errors : [];
      return errors.map((entry) => formatConfigError(entry));
    }
    default:
      return [
        typeof payload.message === 'string'
          ? payload.message
          : 'Config draft validation failed.',
      ];
  }
}

/**
 * @param {unknown} error
 * @returns {string}
 */
export function formatConfigError(error) {
  if (typeof error === 'string') {
    return error;
  }
  if (!error || typeof error !== 'object') {
    return 'Unknown config error.';
  }

  const record = /** @type {Record<string, unknown>} */ (error);
  switch (record.type) {
    case 'deviceNameNotFound':
      return `deviceName '${record.deviceName}' does not match any entry in devices[]`;
    case 'duplicateDeviceId':
      return `duplicate device id '${record.deviceId}' in devices[]`;
    case 'duplicateMonitorId':
      return `duplicate monitor id '${record.monitorId}' in monitors[]`;
    case 'unknownDeviceInMonitorInput':
      return `monitor '${record.monitorId}' declares an input for unknown device '${record.deviceId}'`;
    case 'unknownMonitorInPresetLayout':
      return `preset '${record.presetName}' routes unknown monitor '${record.monitorId}'`;
    case 'unknownDeviceInPresetLayout':
      return `preset '${record.presetName}' routes monitor '${record.monitorId}' to unknown device '${record.deviceId}'`;
    case 'deviceNotInputForMonitor':
      return `preset '${record.presetName}' routes monitor '${record.monitorId}' to '${record.deviceId}', but ${record.monitorId} has no input for that device`;
    case 'unknownControlledBy':
      return `monitor '${record.monitorId}' has unknown controlledBy '${record.controlledBy}'`;
    case 'locallyOwnedMonitorMissingInputs':
      return `monitor '${record.monitorId}' is owned by this machine but declares no inputs`;
    case 'peerNameNotFound':
      return `peer '${record.peerName}' does not match any entry in devices[]`;
    case 'peerNameIsLocalDevice':
      return `peer '${record.peerName}' must not name this machine (deviceName)`;
    case 'unknownHotkeyPreset':
      return `hotkeys references unknown preset '${record.presetName}'`;
    case 'invalidHotkeyShortcut':
      return `hotkeys['${record.presetName}'] has invalid shortcut '${record.shortcut}'`;
    case 'duplicateHotkey':
      return `hotkeys assigns the same shortcut '${record.shortcut}' to presets '${record.presetA}' and '${record.presetB}'`;
    case 'inputMissingBackend':
      return `monitor '${record.monitorId}' input for device '${record.deviceId}' has neither a command nor nativeDdc configured`;
    case 'nativeInputMissingDisplayId':
      return `monitor '${record.monitorId}' input for device '${record.deviceId}' sets nativeDdc, but monitor '${record.monitorId}' has no nativeDdc.displayId`;
    default: {
      const { type, ...rest } = record;
      const detail = Object.entries(rest)
        .map(([key, value]) => `${key}: ${String(value)}`)
        .join(', ');
      return detail ? `${String(type)} (${detail})` : String(type);
    }
  }
}
