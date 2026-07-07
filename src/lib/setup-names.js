/**
 * @param {string} label
 * @param {string} [fallback]
 * @returns {string}
 */
export function slugifyId(label, fallback = 'item') {
  let slug = label
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '_')
    .replace(/^_+|_+$/g, '')
    .replace(/_+/g, '_');

  if (!slug) {
    slug = fallback;
  }

  if (/^[0-9]/.test(slug)) {
    slug = `_${slug}`;
  }

  return slug;
}

/**
 * @param {string[]} labels
 * @param {(label: string, index: number) => string} makeId
 * @returns {string[]}
 */
export function ensureUniqueIds(labels, makeId) {
  /** @type {Set<string>} */
  const used = new Set();
  /** @type {string[]} */
  const ids = [];

  for (let index = 0; index < labels.length; index += 1) {
    const label = labels[index];
    const baseId = makeId(label, index);
    let id = baseId;
    let suffix = 2;

    while (used.has(id)) {
      id = `${baseId}_${suffix}`;
      suffix += 1;
    }

    used.add(id);
    ids.push(id);
  }

  return ids;
}

/**
 * @param {string} label
 * @returns {string}
 */
export function makeDeviceId(label) {
  return slugifyId(label, 'my_pc');
}

/**
 * @param {number} index
 * @returns {string}
 */
export function monitorLabelFallback(index) {
  return `Monitor ${index + 1}`;
}

/**
 * @param {string} label
 * @param {number} index
 * @returns {string}
 */
export function makeMonitorId(label, index) {
  const effective = label?.trim() || monitorLabelFallback(index);
  return slugifyId(effective, `monitor_${index + 1}`);
}

/**
 * @param {string} deviceLabel
 * @returns {string}
 */
export function defaultPresetLabel(deviceLabel) {
  const trimmed = deviceLabel.trim();
  return trimmed ? `All ${trimmed}` : 'All monitors';
}

/**
 * @param {string} label
 * @returns {string}
 */
export function makePresetId(label) {
  return slugifyId(label, 'all_preset');
}
