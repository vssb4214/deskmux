/** @typedef {import('../types.js').DeskMuxEvent} DeskMuxEvent */

/**
 * @param {number | undefined} timestampMs
 * @returns {string}
 */
export function formatEventTimestamp(timestampMs) {
  if (!timestampMs) {
    return '';
  }
  try {
    return new Date(timestampMs).toLocaleTimeString();
  } catch {
    return String(timestampMs);
  }
}

/**
 * @param {DeskMuxEvent['kind']} kind
 * @returns {string}
 */
export function eventKindToBadgeClass(kind) {
  switch (kind) {
    case 'success':
      return 'badge badge-ok';
    case 'error':
      return 'badge badge-error';
    default:
      return 'badge badge-info';
  }
}

/**
 * @param {DeskMuxEvent['kind']} kind
 * @returns {string}
 */
export function formatEventKindLabel(kind) {
  switch (kind) {
    case 'success':
      return 'Success';
    case 'error':
      return 'Error';
    default:
      return 'Info';
  }
}

/**
 * @param {DeskMuxEvent} event
 * @returns {string}
 */
export function formatEventMeta(event) {
  const parts = [];
  if (event.preset) {
    parts.push(`preset: ${event.preset}`);
  }
  if (event.source) {
    parts.push(`via ${event.source}`);
  }
  if (event.monitorId) {
    parts.push(`monitor: ${event.monitorId}`);
  }
  return parts.join(' · ');
}
