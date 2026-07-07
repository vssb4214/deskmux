/** @typedef {import('../types.js').StatusResponse} StatusResponse */
/** @typedef {import('../types.js').ApplyPresetResponse} ApplyPresetResponse */
/** @typedef {import('../types.js').MonitorResult} MonitorResult */
/** @typedef {import('../types.js').PeerApplyOutcome} PeerApplyOutcome */
/** @typedef {import('../types.js').MonitorOutcome} MonitorOutcome */
/** @typedef {import('../types.js').PlanningError} PlanningError */
/** @typedef {import('../types.js').DeskMuxEvent} DeskMuxEvent */
/** @typedef {import('../types.js').DiscoveryDisplaysResponse} DiscoveryDisplaysResponse */

import {
  eventKindToBadgeClass,
  formatEventKindLabel,
  formatEventMeta,
  formatEventTimestamp,
} from '../lib/events.js';
import {
  DISCOVERY_EMPTY_MESSAGE,
  DISCOVERY_INSTRUCTIONS,
  DISCOVERY_UNAVAILABLE_MESSAGE,
  formatDisplayLabel,
} from '../lib/discovery.js';
import {
  classifyApplyResult,
  summaryBannerText,
} from '../lib/summary.js';
import { CONFIG_FILE_HINT } from '../lib/config-error.js';
import {
  CONFIG_DRAFT_DESKTOP_ONLY,
  CONFIG_DRAFT_SUCCESS_MESSAGE,
} from '../lib/config-draft.js';

/**
 * @param {MonitorOutcome} outcome
 */
export function outcomeLabel(outcome) {
  switch (outcome.type) {
    case 'dryRun':
      return 'Dry run';
    case 'success':
      return 'Success';
    case 'failed':
      return 'Failed';
    case 'spawnFailed':
      return 'Spawn failed';
    case 'resolutionFailed':
      return 'Resolution failed';
    default:
      return 'Unknown';
  }
}

/**
 * @param {MonitorOutcome} outcome
 */
function outcomeClass(outcome) {
  if (outcome.type === 'dryRun') {
    return 'badge badge-info';
  }
  if (outcome.type === 'success') {
    return 'badge badge-ok';
  }
  return 'badge badge-error';
}

/**
 * @param {HTMLElement} container
 * @param {string | null} message
 * @param {'error' | 'warning' | 'info'} kind
 */
export function renderBanner(container, message, kind) {
  container.hidden = !message;
  container.textContent = message ?? '';
  container.className = `banner banner-${kind}`;
}

/**
 * @param {HTMLElement} container
 * @param {string | undefined | null} configError
 */
export function renderConfigErrorBanner(container, configError) {
  container.hidden = false;
  container.className = 'banner banner-error';
  container.replaceChildren();

  const title = document.createElement('p');
  title.className = 'config-error-title';
  const titleStrong = document.createElement('strong');
  titleStrong.textContent = 'Config not loaded';
  title.appendChild(titleStrong);

  const detail = document.createElement('p');
  detail.className = 'config-error-detail';
  detail.textContent = configError?.trim() || 'Unknown configuration error.';

  const hint = document.createElement('p');
  hint.className = 'config-error-hint';
  hint.textContent = CONFIG_FILE_HINT;

  container.append(title, detail, hint);
}

/**
 * @param {HTMLElement} container
 * @param {StatusResponse} status
 */
export function renderStatus(container, status) {
  container.innerHTML = '';

  const device = document.createElement('p');
  device.className = 'meta-line';
  device.innerHTML = `<strong>Device</strong> ${escapeHtml(status.deviceName)}`;
  container.appendChild(device);

  const last = document.createElement('p');
  last.className = 'meta-line';
  const lastText = status.lastAppliedPreset ?? 'None';
  last.innerHTML = `<strong>Last applied preset</strong> ${escapeHtml(lastText)}`;
  container.appendChild(last);
}

/**
 * @param {HTMLElement} container
 * @param {StatusResponse['monitors']} monitors
 */
export function renderMonitors(container, monitors) {
  container.innerHTML = '';
  if (monitors.length === 0) {
    container.textContent = 'No monitors configured.';
    return;
  }

  const list = document.createElement('ul');
  list.className = 'item-list';
  for (const monitor of [...monitors].sort(
    (a, b) => a.order - b.order || a.id.localeCompare(b.id),
  )) {
    const item = document.createElement('li');
    item.textContent = `${monitor.label} (${monitor.id})`;
    list.appendChild(item);
  }
  container.appendChild(list);
}

/**
 * @param {HTMLSelectElement} select
 * @param {StatusResponse['presets']} presets
 */
export function renderPresetOptions(select, presets) {
  select.innerHTML = '';
  const sorted = [...presets].sort((a, b) => a.name.localeCompare(b.name));
  for (const preset of sorted) {
    const option = document.createElement('option');
    option.value = preset.name;
    option.textContent = `${preset.label} (${preset.name})`;
    select.appendChild(option);
  }
  select.disabled = sorted.length === 0;
}

/**
 * @param {HTMLElement} container
 * @param {ApplyPresetResponse} response
 */
export function renderApplyResult(container, response) {
  container.hidden = false;
  container.innerHTML = '';

  const summaryClass = classifyApplyResult(response);
  const banner = document.createElement('div');
  banner.className = `result-banner result-${summaryClass}`;
  banner.textContent = summaryBannerText(summaryClass);
  container.appendChild(banner);

  if (response.planningErrors.length > 0) {
    container.appendChild(
      renderSection('Planning errors', renderPlanningErrors(response.planningErrors)),
    );
  }

  if (response.localResults.length > 0) {
    container.appendChild(
      renderSection('Local results', renderMonitorResults(response.localResults)),
    );
  }

  if (response.peerResults.length > 0) {
    container.appendChild(
      renderSection('Peer results', renderPeerResults(response.peerResults, 0)),
    );
  }
}

/**
 * @param {string} title
 * @param {HTMLElement} body
 */
function renderSection(title, body) {
  const section = document.createElement('section');
  section.className = 'result-section';
  const heading = document.createElement('h3');
  heading.textContent = title;
  section.appendChild(heading);
  section.appendChild(body);
  return section;
}

/**
 * @param {PlanningError[]} errors
 */
function renderPlanningErrors(errors) {
  const list = document.createElement('ul');
  list.className = 'item-list';
  for (const error of errors) {
    const item = document.createElement('li');
    item.textContent = `Unknown monitor: ${error.monitorId}`;
    list.appendChild(item);
  }
  return list;
}

/**
 * @param {MonitorResult[]} results
 */
function renderMonitorResults(results) {
  const wrap = document.createElement('div');
  wrap.className = 'result-cards';
  for (const result of results) {
    wrap.appendChild(renderMonitorCard(result));
  }
  return wrap;
}

/**
 * @param {MonitorResult} result
 */
function renderMonitorCard(result) {
  const card = document.createElement('article');
  card.className = 'result-card';

  const header = document.createElement('div');
  header.className = 'result-card-header';
  const title = document.createElement('strong');
  title.textContent = `${result.monitorId} → ${result.deviceId}`;
  const badge = document.createElement('span');
  badge.className = outcomeClass(result.outcome);
  badge.textContent = outcomeLabel(result.outcome);
  header.appendChild(title);
  header.appendChild(badge);
  card.appendChild(header);

  if (result.command) {
    const command = document.createElement('p');
    command.className = 'command-line';
    command.textContent = result.command;
    card.appendChild(command);
  }

  appendOutcomeDetails(card, result.outcome);
  return card;
}

/**
 * @param {HTMLElement} card
 * @param {MonitorOutcome} outcome
 */
function appendOutcomeDetails(card, outcome) {
  if (outcome.type === 'success' || outcome.type === 'failed') {
    if (outcome.stdout) {
      card.appendChild(detailBlock('stdout', outcome.stdout));
    }
    if (outcome.stderr) {
      card.appendChild(detailBlock('stderr', outcome.stderr));
    }
    if (outcome.type === 'failed' && outcome.exitCode != null) {
      const code = document.createElement('p');
      code.className = 'meta-line';
      code.textContent = `Exit code: ${outcome.exitCode}`;
      card.appendChild(code);
    }
  } else if (outcome.type === 'spawnFailed') {
    card.appendChild(detailBlock('error', outcome.message));
  } else if (outcome.type === 'resolutionFailed') {
    card.appendChild(detailBlock('error', JSON.stringify(outcome.error)));
  }
}

/**
 * @param {string} label
 * @param {string} text
 */
function detailBlock(label, text) {
  const block = document.createElement('details');
  block.className = 'detail-block';
  const summary = document.createElement('summary');
  summary.textContent = label;
  const pre = document.createElement('pre');
  pre.textContent = text;
  block.appendChild(summary);
  block.appendChild(pre);
  return block;
}

/**
 * @param {PeerApplyOutcome[]} peers
 * @param {number} depth
 */
function renderPeerResults(peers, depth) {
  const wrap = document.createElement('div');
  wrap.className = 'peer-results';
  wrap.style.marginLeft = depth > 0 ? `${depth * 1.25}rem` : '0';

  for (const peer of peers) {
    const block = document.createElement('article');
    block.className = 'result-card peer-card';

    const header = document.createElement('div');
    header.className = 'result-card-header';
    const title = document.createElement('strong');
    title.textContent = `Peer ${peer.deviceId}`;
    header.appendChild(title);

    if (peer.outcome.type === 'failed') {
      const badge = document.createElement('span');
      badge.className = 'badge badge-error';
      badge.textContent = 'HTTP failed';
      header.appendChild(badge);
      block.appendChild(header);
      const err = document.createElement('p');
      err.textContent = peer.outcome.error;
      block.appendChild(err);
      if (peer.outcome.httpStatus != null) {
        const status = document.createElement('p');
        status.className = 'meta-line';
        status.textContent = `HTTP ${peer.outcome.httpStatus}`;
        block.appendChild(status);
      }
    } else {
      const badge = document.createElement('span');
      badge.className = 'badge badge-ok';
      badge.textContent = peer.outcome.localOnly
        ? 'Success (local only)'
        : 'Success';
      header.appendChild(badge);
      block.appendChild(header);

      if (peer.outcome.results.length > 0) {
        block.appendChild(renderMonitorResults(peer.outcome.results));
      }

      const nested = peer.outcome.peerResults ?? [];
      if (nested.length > 0) {
        const nestedTitle = document.createElement('p');
        nestedTitle.className = 'meta-line';
        nestedTitle.textContent = 'Nested peer fan-out';
        block.appendChild(nestedTitle);
        block.appendChild(renderPeerResults(nested, depth + 1));
      }
    }

    wrap.appendChild(block);
  }

  return wrap;
}

/**
 * @param {HTMLElement} container
 * @param {DeskMuxEvent[]} events
 */
export function renderEvents(container, events) {
  container.replaceChildren();

  if (events.length === 0) {
    const empty = document.createElement('p');
    empty.className = 'meta-line muted';
    empty.textContent = 'No recent events yet.';
    container.appendChild(empty);
    return;
  }

  const list = document.createElement('ul');
  list.className = 'event-list';

  for (const event of events) {
    const item = document.createElement('li');
    item.className = 'event-item';

    const header = document.createElement('div');
    header.className = 'event-header';

    const badge = document.createElement('span');
    badge.className = eventKindToBadgeClass(event.kind);
    badge.textContent = formatEventKindLabel(event.kind);
    header.appendChild(badge);

    const time = document.createElement('time');
    time.className = 'event-time';
    time.dateTime = String(event.timestampMs);
    time.textContent = formatEventTimestamp(event.timestampMs);
    header.appendChild(time);

    const message = document.createElement('p');
    message.className = 'event-message';
    message.textContent = event.message;

    const meta = formatEventMeta(event);
    item.appendChild(header);
    item.appendChild(message);
    if (meta) {
      const metaLine = document.createElement('p');
      metaLine.className = 'event-meta';
      metaLine.textContent = meta;
      item.appendChild(metaLine);
    }

    list.appendChild(item);
  }

  container.appendChild(list);
}

/**
 * Renders the monitor-discovery panel. `onReadInput(displayId, readingEl, buttonEl)` is called
 * when a display's read button is clicked; the caller owns the fetch and writes the outcome
 * into `readingEl` (textContent only — never markup).
 *
 * @param {HTMLElement} container
 * @param {DiscoveryDisplaysResponse} data
 * @param {(displayId: string, readingEl: HTMLElement, buttonEl: HTMLButtonElement) => void} onReadInput
 */
export function renderDiscoveryPanel(container, data, onReadInput) {
  container.replaceChildren();

  if (!data.nativeAvailable) {
    const message = document.createElement('p');
    message.className = 'meta-line muted';
    message.textContent = DISCOVERY_UNAVAILABLE_MESSAGE;
    container.appendChild(message);
    return;
  }

  if (data.displays.length === 0) {
    const empty = document.createElement('p');
    empty.className = 'meta-line muted';
    empty.textContent = DISCOVERY_EMPTY_MESSAGE;
    container.appendChild(empty);
    return;
  }

  const instructions = document.createElement('p');
  instructions.className = 'helper';
  instructions.textContent = DISCOVERY_INSTRUCTIONS;
  container.appendChild(instructions);

  const list = document.createElement('ul');
  list.className = 'discovery-list';

  data.displays.forEach((display, index) => {
    const item = document.createElement('li');
    item.className = 'discovery-item';

    const header = document.createElement('div');
    header.className = 'discovery-item-header';

    const label = document.createElement('span');
    label.className = 'discovery-label';
    label.textContent = formatDisplayLabel(index, display.displayId);
    header.appendChild(label);

    const readBtn = document.createElement('button');
    readBtn.type = 'button';
    readBtn.className = 'btn btn-secondary btn-small';
    readBtn.textContent = 'Read current input';
    header.appendChild(readBtn);

    const reading = document.createElement('p');
    reading.className = 'discovery-reading meta-line muted';
    reading.textContent = 'Not read yet.';

    readBtn.addEventListener('click', () => {
      onReadInput(display.displayId, reading, readBtn);
    });

    item.appendChild(header);
    item.appendChild(reading);
    list.appendChild(item);
  });

  container.appendChild(list);
}

/**
 * Temporary first-run plumbing — replaced by the guided setup wizard (phase 3).
 *
 * @param {HTMLElement} container
 */
export function renderConfigDraftDesktopOnly(container) {
  container.replaceChildren();

  const message = document.createElement('p');
  message.className = 'meta-line muted';
  message.textContent = CONFIG_DRAFT_DESKTOP_ONLY;
  container.appendChild(message);
}

/**
 * @param {HTMLElement} container
 * @param {string} draft
 */
export function renderConfigDraftEditor(container, draft) {
  container.replaceChildren();

  const helper = document.createElement('p');
  helper.className = 'helper';
  helper.textContent =
    'Edit draft JSON, validate, then save. Saving writes deskmux.config.json in the app directory.';

  const field = document.createElement('label');
  field.className = 'field config-draft-field';
  const label = document.createElement('span');
  label.textContent = 'Draft config JSON';
  const textarea = document.createElement('textarea');
  textarea.id = 'config-draft-textarea';
  textarea.className = 'config-draft-textarea';
  textarea.rows = 14;
  textarea.spellcheck = false;
  textarea.value = draft;
  field.append(label, textarea);

  const actions = document.createElement('div');
  actions.className = 'config-draft-actions';

  const validateBtn = document.createElement('button');
  validateBtn.type = 'button';
  validateBtn.id = 'config-draft-validate-btn';
  validateBtn.className = 'btn btn-secondary';
  validateBtn.textContent = 'Validate';

  const saveBtn = document.createElement('button');
  saveBtn.type = 'button';
  saveBtn.id = 'config-draft-save-btn';
  saveBtn.className = 'btn btn-primary';
  saveBtn.textContent = 'Save';

  actions.append(validateBtn, saveBtn);

  const feedback = document.createElement('div');
  feedback.id = 'config-draft-feedback';
  feedback.hidden = true;

  container.append(helper, field, actions, feedback);
}

/**
 * @param {HTMLElement} container
 * @param {string[]} messages
 */
export function renderConfigDraftErrorList(container, messages) {
  container.hidden = messages.length === 0;
  container.replaceChildren();
  if (messages.length === 0) {
    return;
  }

  container.className = 'config-draft-feedback config-draft-feedback-error';

  const title = document.createElement('p');
  title.className = 'config-draft-feedback-title';
  title.textContent = 'Fix these issues before saving:';

  const list = document.createElement('ul');
  list.className = 'item-list config-draft-error-list';
  for (const message of messages) {
    const item = document.createElement('li');
    item.textContent = message;
    list.appendChild(item);
  }

  container.append(title, list);
}

/**
 * @param {HTMLElement} container
 * @param {string} [message]
 */
export function renderConfigDraftSuccess(container, message = CONFIG_DRAFT_SUCCESS_MESSAGE) {
  container.hidden = false;
  container.className = 'config-draft-feedback config-draft-feedback-success';
  container.replaceChildren();

  const text = document.createElement('p');
  text.className = 'config-draft-success';
  text.textContent = message;
  container.appendChild(text);
}

/**
 * @param {string} value
 */
function escapeHtml(value) {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;');
}
