/** @typedef {import('../lib/setup-status.js').SetupStatus} SetupStatus */

import {
  getSetupStatusCopy,
  setupStatusBadgeClass,
} from '../lib/setup-status.js';

/**
 * @param {HTMLElement} badgeEl
 * @param {HTMLElement} messageEl
 * @param {HTMLButtonElement | null} ctaEl
 * @param {SetupStatus} status
 * @param {{ configError?: string }} [health]
 */
export function renderSetupStatusBar(badgeEl, messageEl, ctaEl, status, health = {}) {
  const copy = getSetupStatusCopy(status, health);

  badgeEl.className = setupStatusBadgeClass(status);
  badgeEl.textContent = copy.badge;
  messageEl.textContent = copy.message;

  if (ctaEl) {
    const showCta = copy.cta && (status === 'needsSetup' || status === 'inProgress');
    ctaEl.hidden = !showCta;
    ctaEl.textContent = copy.cta ?? '';
  }
}

/**
 * @param {HTMLElement} container
 * @param {boolean} configLoaded
 */
export function renderPrimaryActionHeading(container, configLoaded) {
  container.replaceChildren();
  const heading = document.createElement('h2');
  heading.textContent = configLoaded ? 'Apply preset' : 'Get started';
  container.appendChild(heading);
}

/**
 * @param {HTMLElement} container
 */
export function renderSetupStartHint(container) {
  container.replaceChildren();
  const text = document.createElement('p');
  text.className = 'helper';
  text.textContent =
    'DeskMux is not configured yet. Work through the setup checklist below, or open Advanced options to edit JSON manually.';
  container.appendChild(text);
}

/**
 * @param {import('../lib/setup-checklist.js').SetupStepView[]} steps
 * @param {HTMLElement} listEl
 */
export function renderSetupChecklist(steps, listEl) {
  listEl.replaceChildren();

  const list = document.createElement('ol');
  list.className = 'setup-checklist';

  for (const step of steps) {
    const item = document.createElement('li');
    item.className = `setup-checklist-item setup-step-${step.state}`;
    item.dataset.stepId = step.id;

    const header = document.createElement('div');
    header.className = 'setup-checklist-header';

    const title = document.createElement('span');
    title.className = 'setup-checklist-title';
    title.textContent = step.label;

    const badge = document.createElement('span');
    badge.className = `badge setup-step-badge setup-step-badge-${step.state}`;
    badge.textContent =
      step.state === 'complete'
        ? 'Done'
        : step.state === 'current'
          ? 'Now'
          : step.state === 'blocked'
            ? 'Waiting'
            : 'Next';

    const helper = document.createElement('p');
    helper.className = 'setup-checklist-helper';
    helper.textContent = step.helper;

    header.append(title, badge);
    item.append(header, helper);
    list.appendChild(item);
  }

  listEl.appendChild(list);
}

/**
 * @param {HTMLElement} container
 * @param {import('../lib/setup-session.js').SetupSession} session
 * @param {{
 *   getMonitorName?: (displayId: string) => string,
 *   onInputLabelChange?: (displayId: string, label: string) => void,
 * } | undefined} [options]
 */
export function renderCapturedReadings(container, session, options) {
  container.replaceChildren();

  if (!session.readings?.length) {
    const empty = document.createElement('p');
    empty.className = 'meta-line muted';
    empty.textContent = 'No input values captured yet.';
    container.appendChild(empty);
    return;
  }

  const list = document.createElement('ul');
  list.className = 'captured-readings-list';

  for (const reading of session.readings) {
    const item = document.createElement('li');
    item.className = 'captured-reading-card';

    const monitorName =
      options?.getMonitorName?.(reading.displayId) ?? reading.label ?? reading.displayId;

    const title = document.createElement('strong');
    title.textContent = monitorName;

    const display = document.createElement('p');
    display.className = 'meta-line';
    display.textContent = `Display: ${reading.displayId}`;

    const value = document.createElement('p');
    value.className = 'meta-line';
    value.textContent = `Input value: ${reading.current}`;

    item.append(title, display, value);

    if (options?.onInputLabelChange) {
      const inputField = document.createElement('label');
      inputField.className = 'field setup-name-field';

      const inputLabel = document.createElement('span');
      inputLabel.textContent = 'Input label';

      const inputControl = document.createElement('input');
      inputControl.type = 'text';
      inputControl.className = 'text-input';
      inputControl.placeholder = 'e.g. Desktop or MacBook';
      inputControl.value = reading.inputLabel ?? session.deviceName ?? '';
      inputControl.addEventListener('input', () => {
        options.onInputLabelChange?.(reading.displayId, inputControl.value);
      });

      inputField.append(inputLabel, inputControl);
      item.appendChild(inputField);
    }

    list.appendChild(item);
  }

  container.appendChild(list);
}

/**
 * @param {HTMLElement} container
 * @param {string[]} errors
 */
export function renderSetupDraftErrors(container, errors) {
  container.replaceChildren();
  container.hidden = errors.length === 0;
  if (errors.length === 0) {
    return;
  }

  const title = document.createElement('p');
  title.className = 'config-draft-feedback-title';
  title.textContent = 'Cannot generate draft yet:';

  const list = document.createElement('ul');
  list.className = 'item-list';
  for (const error of errors) {
    const item = document.createElement('li');
    item.textContent = error;
    list.appendChild(item);
  }

  container.append(title, list);
}

/**
 * @param {HTMLElement} container
 * @param {{
 *   deviceLabel: string,
 *   monitorLabels: string[],
 *   presetLabel: string,
 * }} summary
 */
export function renderDraftSummary(container, summary) {
  container.replaceChildren();
  container.hidden = false;

  const title = document.createElement('p');
  title.className = 'setup-draft-summary-title';
  title.textContent = 'Generated draft summary';

  const list = document.createElement('ul');
  list.className = 'setup-draft-summary-list';

  const entries = [
    ['Device', summary.deviceLabel],
    ['Monitors', summary.monitorLabels.join(', ')],
    ['Preset', summary.presetLabel],
  ];

  for (const [label, value] of entries) {
    const item = document.createElement('li');
    item.className = 'meta-line';
    item.textContent = `${label}: ${value}`;
    list.appendChild(item);
  }

  container.append(title, list);
}

/**
 * @param {HTMLElement} container
 */
export function renderEventsEmptyState(container) {
  container.replaceChildren();
  const text = document.createElement('p');
  text.className = 'meta-line muted';
  text.textContent =
    'Activity from config loads, preset applies, and monitor switching will appear here.';
  container.appendChild(text);
}
