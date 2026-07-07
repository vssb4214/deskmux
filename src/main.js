import { resolveApiBaseUrl } from './api/bootstrap.js';
import { saveConfigDraft, validateConfigDraft } from './api/config-draft.js';
import { createApiClient } from './api/client.js';
import { configErrorFromUnknown } from './lib/config-error.js';
import { defaultConfigDraftSkeleton, draftErrorsFromInvoke } from './lib/config-draft.js';
import {
  discoveryErrorMessage,
  formatDisplayLabel,
  formatInputSourceReading,
} from './lib/discovery.js';
import { buildSetupChecklist } from './lib/setup-checklist.js';
import { buildConfigDraftFromSetupState } from './lib/setup-draft.js';
import {
  defaultPresetLabel,
  makeDeviceId,
  makeMonitorId,
  makePresetId,
} from './lib/setup-names.js';
import {
  createSetupSession,
  getEffectiveMonitorName,
  markDraftGenerated,
  markSaveSucceeded,
  markSetupStarted,
  recordReading,
  setDeviceName,
  setDisplays,
  setMonitorName,
  setPresetLabel,
  setReadingInputLabel,
} from './lib/setup-session.js';
import { deriveSetupStatus } from './lib/setup-status.js';
import { isTauriDesktop } from './lib/tauri.js';
import {
  renderApplyResult,
  renderBanner,
  renderConfigDraftDesktopOnly,
  renderConfigDraftEditor,
  renderConfigDraftErrorList,
  renderConfigDraftSuccess,
  renderConfigErrorBanner,
  renderDiscoveryPanel,
  renderMonitors,
  renderPresetOptions,
  renderStatus,
  renderEvents,
} from './ui/render.js';
import {
  renderCapturedReadings,
  renderDraftSummary,
  renderEventsEmptyState,
  renderPrimaryActionHeading,
  renderSetupChecklist,
  renderSetupDraftErrors,
  renderSetupStartHint,
  renderSetupStatusBar,
} from './ui/setup.js';

/** @typedef {import('./types.js').StatusResponse} StatusResponse */
/** @typedef {import('./lib/setup-session.js').SetupSession} SetupSession */

/** @type {ReturnType<typeof createApiClient> | null} */
let client = null;

/** @type {import('./types.js').HealthResponse | null} */
let currentHealth = null;

/** @type {StatusResponse | null} */
let currentStatus = null;

/** @type {SetupSession} */
let setupSession = createSetupSession();

/** @type {boolean} */
let nativeAvailable = false;

const els = {
  bootstrapBanner: document.getElementById('bootstrap-banner'),
  setupStatusBadge: document.getElementById('setup-status-badge'),
  setupStatusMessage: document.getElementById('setup-status-message'),
  setupPrimaryCta: /** @type {HTMLButtonElement} */ (document.getElementById('setup-primary-cta')),
  statusBanner: document.getElementById('status-banner'),
  primaryActionHeading: document.getElementById('primary-action-heading'),
  setupStartSection: document.getElementById('setup-start-section'),
  applySection: document.getElementById('apply-section'),
  presetSelect: /** @type {HTMLSelectElement} */ (document.getElementById('preset-select')),
  dryRun: /** @type {HTMLInputElement} */ (document.getElementById('dry-run')),
  applyBtn: /** @type {HTMLButtonElement} */ (document.getElementById('apply-btn')),
  refreshBtn: /** @type {HTMLButtonElement} */ (document.getElementById('refresh-btn')),
  applyPanel: document.getElementById('apply-panel'),
  eventsPanel: document.getElementById('events-panel'),
  eventsDetails: /** @type {HTMLDetailsElement} */ (document.getElementById('events-details')),
  setupChecklistCard: document.getElementById('setup-checklist-card'),
  setupChecklist: document.getElementById('setup-checklist'),
  deviceNameInput: /** @type {HTMLInputElement} */ (document.getElementById('device-name-input')),
  deviceIdPreview: document.getElementById('device-id-preview'),
  discoveryPanel: document.getElementById('discovery-panel'),
  detectBtn: /** @type {HTMLButtonElement} */ (document.getElementById('detect-btn')),
  capturedReadingsPanel: document.getElementById('captured-readings-panel'),
  generateDraftBtn: /** @type {HTMLButtonElement} */ (document.getElementById('generate-draft-btn')),
  generateDraftErrors: document.getElementById('generate-draft-errors'),
  presetLabelInput: /** @type {HTMLInputElement} */ (document.getElementById('preset-label-input')),
  presetIdPreview: document.getElementById('preset-id-preview'),
  draftSummaryPanel: document.getElementById('draft-summary-panel'),
  configDraftPanel: document.getElementById('config-draft-panel'),
  statusPanel: document.getElementById('status-panel'),
  monitorList: document.getElementById('monitor-list'),
  loading: document.getElementById('loading'),
};

const emptyStatus = {
  deviceName: '—',
  presets: [],
  monitors: [],
  lastAppliedPreset: null,
};

function getSetupStatus() {
  return deriveSetupStatus(currentHealth ?? { configLoaded: false }, setupSession);
}

function scrollToChecklist() {
  els.setupChecklistCard?.scrollIntoView({ behavior: 'smooth', block: 'start' });
}

function setControlsDisabled(disabled) {
  els.presetSelect.disabled = disabled || els.presetSelect.options.length === 0;
  els.dryRun.disabled = disabled;
  els.applyBtn.disabled = disabled || els.presetSelect.options.length === 0;
  els.refreshBtn.disabled = disabled;
}

function setLoading(visible, message = 'Loading status…') {
  els.loading.hidden = !visible;
  els.loading.textContent = message;
}

/**
 * @param {unknown} err
 */
function errorMessage(err) {
  if (err instanceof Error) {
    return err.message;
  }
  return String(err);
}

function getConfigDraftTextarea() {
  return /** @type {HTMLTextAreaElement | null} */ (
    document.getElementById('config-draft-textarea')
  );
}

function getEffectivePresetLabel() {
  if (setupSession.presetLabel?.trim()) {
    return setupSession.presetLabel.trim();
  }
  return defaultPresetLabel(setupSession.deviceName ?? '');
}

function updateNamingPreviews() {
  const deviceName = setupSession.deviceName?.trim() ?? '';
  if (els.deviceIdPreview) {
    els.deviceIdPreview.textContent = deviceName
      ? `Config id: ${makeDeviceId(deviceName)}`
      : '';
  }

  const presetLabel = getEffectivePresetLabel();
  if (els.presetIdPreview) {
    els.presetIdPreview.textContent = presetLabel
      ? `Config id: ${makePresetId(presetLabel)}`
      : '';
  }
}

function getMonitorNameForDisplay(displayId) {
  const index = (setupSession.displays ?? []).findIndex(
    (display) => display.displayId === displayId,
  );
  const display = index >= 0 ? setupSession.displays?.[index] : undefined;
  return getEffectiveMonitorName(display, Math.max(index, 0));
}

function renderDashboardChrome() {
  const setupStatus = getSetupStatus();
  const configLoaded = Boolean(currentHealth?.configLoaded);

  renderSetupStatusBar(
    els.setupStatusBadge,
    els.setupStatusMessage,
    els.setupPrimaryCta,
    setupStatus,
    currentHealth ?? {},
  );

  renderPrimaryActionHeading(els.primaryActionHeading, configLoaded);
  els.applySection.hidden = !configLoaded;
  els.setupStartSection.hidden = configLoaded;
  if (!configLoaded) {
    renderSetupStartHint(els.setupStartSection);
  }

  els.setupChecklistCard.hidden = configLoaded;
  if (!configLoaded) {
    const steps = buildSetupChecklist(setupSession, setupStatus, {
      isDesktop: isTauriDesktop(),
      nativeAvailable,
    });
    renderSetupChecklist(steps, els.setupChecklist);
    renderCapturedReadings(els.capturedReadingsPanel, setupSession, {
      getMonitorName: getMonitorNameForDisplay,
      onInputLabelChange: (displayId, label) => {
        setupSession = setReadingInputLabel(setupSession, displayId, label);
      },
    });
    if (els.deviceNameInput.value !== (setupSession.deviceName ?? '')) {
      els.deviceNameInput.value = setupSession.deviceName ?? '';
    }
    const effectivePreset = getEffectivePresetLabel();
    if (
      els.presetLabelInput &&
      els.presetLabelInput.value !== effectivePreset &&
      document.activeElement !== els.presetLabelInput
    ) {
      els.presetLabelInput.value = effectivePreset;
    }
    updateNamingPreviews();
  }

  if (els.eventsDetails) {
    els.eventsDetails.open = configLoaded;
  }
}

async function loadEvents() {
  if (!client || !els.eventsPanel) {
    return;
  }

  try {
    const { events } = await client.fetchEvents();
    if (events.length === 0) {
      renderEventsEmptyState(els.eventsPanel);
    } else {
      renderEvents(els.eventsPanel, events);
    }
  } catch {
    renderEventsEmptyState(els.eventsPanel);
  }
}

/**
 * @param {string} displayId
 * @param {string} label
 * @param {HTMLElement} readingEl
 * @param {HTMLButtonElement} buttonEl
 */
async function readDisplayInput(displayId, label, readingEl, buttonEl) {
  if (!client) {
    return;
  }

  buttonEl.disabled = true;
  readingEl.textContent = 'Reading…';

  try {
    const reading = await client.fetchInputSource(displayId);
    readingEl.textContent = formatInputSourceReading(reading);
    readingEl.classList.remove('muted');
    setupSession = recordReading(setupSession, displayId, label, reading);
    renderDashboardChrome();
  } catch (err) {
    readingEl.textContent = discoveryErrorMessage(err);
  } finally {
    buttonEl.disabled = false;
  }
}

async function detectDisplays() {
  if (!client || !els.discoveryPanel) {
    return;
  }

  els.detectBtn.disabled = true;

  try {
    const data = await client.fetchDiscoveryDisplays();
    nativeAvailable = data.nativeAvailable;

    const displays = data.displays.map((display, index) => ({
      displayId: display.displayId,
      label: formatDisplayLabel(index, display.displayId),
    }));
    setupSession = setDisplays(setupSession, displays);

    renderDiscoveryPanel(
      els.discoveryPanel,
      data,
      (displayId, readingEl, buttonEl) => {
        const label = getMonitorNameForDisplay(displayId);
        void readDisplayInput(displayId, label, readingEl, buttonEl);
      },
      {
        getMonitorName: (displayId, index) => {
          const display = setupSession.displays?.[index];
          return getEffectiveMonitorName(display, index);
        },
        onMonitorNameChange: (displayId, name) => {
          setupSession = setMonitorName(setupSession, displayId, name);
          renderDashboardChrome();
        },
        previewMonitorId: (_displayId, index, name) => makeMonitorId(name, index),
      },
    );
    renderDashboardChrome();
  } catch (err) {
    els.discoveryPanel.replaceChildren();
    const message = document.createElement('p');
    message.className = 'meta-line muted';
    message.textContent = `Detection failed: ${errorMessage(err)}`;
    els.discoveryPanel.appendChild(message);
  } finally {
    els.detectBtn.disabled = false;
  }
}

function populateConfigDraftTextarea(json) {
  const textarea = getConfigDraftTextarea();
  if (textarea) {
    textarea.value = json;
    return;
  }

  if (!els.configDraftPanel) {
    return;
  }

  if (!isTauriDesktop()) {
    renderConfigDraftDesktopOnly(els.configDraftPanel);
    return;
  }

  renderConfigDraftEditor(els.configDraftPanel, json, { advanced: true });
  wireConfigDraftPanel();
}

function wireConfigDraftPanel() {
  const textarea = getConfigDraftTextarea();
  const feedback = document.getElementById('config-draft-feedback');
  const validateBtn = /** @type {HTMLButtonElement | null} */ (
    document.getElementById('config-draft-validate-btn')
  );
  const saveBtn = /** @type {HTMLButtonElement | null} */ (
    document.getElementById('config-draft-save-btn')
  );

  if (!textarea || !feedback || !validateBtn || !saveBtn) {
    return;
  }

  validateBtn.replaceWith(validateBtn.cloneNode(true));
  saveBtn.replaceWith(saveBtn.cloneNode(true));

  const freshValidateBtn = /** @type {HTMLButtonElement} */ (
    document.getElementById('config-draft-validate-btn')
  );
  const freshSaveBtn = /** @type {HTMLButtonElement} */ (
    document.getElementById('config-draft-save-btn')
  );

  freshValidateBtn.addEventListener('click', () => {
    void (async () => {
      freshValidateBtn.disabled = true;
      freshSaveBtn.disabled = true;
      try {
        await validateConfigDraft(textarea.value);
        renderConfigDraftSuccess(
          feedback,
          'Draft is valid. Click Save to write deskmux.config.json.',
        );
      } catch (err) {
        renderConfigDraftErrorList(feedback, draftErrorsFromInvoke(err));
      } finally {
        freshValidateBtn.disabled = false;
        freshSaveBtn.disabled = false;
      }
    })();
  });

  freshSaveBtn.addEventListener('click', () => {
    void (async () => {
      freshValidateBtn.disabled = true;
      freshSaveBtn.disabled = true;
      try {
        const result = await saveConfigDraft(textarea.value);
        setupSession = markSaveSucceeded(setupSession);
        renderDashboardChrome();
        if (result.restartRequired) {
          renderConfigDraftSuccess(feedback);
        } else {
          renderConfigDraftSuccess(feedback, 'Saved to deskmux.config.json.');
        }
      } catch (err) {
        renderConfigDraftErrorList(feedback, draftErrorsFromInvoke(err));
      } finally {
        freshValidateBtn.disabled = false;
        freshSaveBtn.disabled = false;
      }
    })();
  });
}

function initConfigDraftPanel() {
  if (!els.configDraftPanel) {
    return;
  }

  if (!isTauriDesktop()) {
    renderConfigDraftDesktopOnly(els.configDraftPanel);
    return;
  }

  renderConfigDraftEditor(els.configDraftPanel, defaultConfigDraftSkeleton(), {
    advanced: true,
  });
  wireConfigDraftPanel();
}

function generateDraftFromSetup() {
  const result = buildConfigDraftFromSetupState(setupSession);
  if (!result.ok) {
    renderSetupDraftErrors(els.generateDraftErrors, result.errors);
    els.draftSummaryPanel.hidden = true;
    return;
  }

  els.generateDraftErrors.hidden = true;
  setupSession = markDraftGenerated(setupSession);
  populateConfigDraftTextarea(result.json);

  renderDraftSummary(els.draftSummaryPanel, {
    deviceLabel: setupSession.deviceName?.trim() ?? 'this computer',
    monitorLabels: (setupSession.readings ?? []).map((reading) =>
      getMonitorNameForDisplay(reading.displayId),
    ),
    presetLabel: getEffectivePresetLabel(),
  });
  renderDashboardChrome();

  document.getElementById('advanced-details')?.scrollIntoView({
    behavior: 'smooth',
    block: 'start',
  });
}

/**
 * @param {string | undefined} configError
 */
async function showConfigError(configError) {
  renderConfigErrorBanner(els.statusBanner, configError);
  renderStatus(els.statusPanel, emptyStatus);
  renderMonitors(els.monitorList, []);
  renderPresetOptions(els.presetSelect, []);
  setControlsDisabled(true);
  renderDashboardChrome();
  await loadEvents();
}

async function loadStatus() {
  if (!client) {
    return;
  }

  setLoading(true);
  renderBanner(els.statusBanner, null, 'info');
  setControlsDisabled(true);

  try {
    const health = await client.fetchHealth();
    currentHealth = health;

    if (!health.configLoaded) {
      await showConfigError(health.configError);
      return;
    }

    /** @type {StatusResponse} */
    const status = await client.fetchStatus();
    currentStatus = status;
    renderBanner(els.statusBanner, null, 'info');
    renderStatus(els.statusPanel, status);
    renderMonitors(els.monitorList, status.monitors);
    renderPresetOptions(els.presetSelect, status.presets);
    setControlsDisabled(false);
    renderDashboardChrome();
    await loadEvents();
  } catch (err) {
    const configError = configErrorFromUnknown(err);
    const status = /** @type {{ status?: number }} */ (err).status;
    if (status === 503 && configError) {
      currentHealth = { status: 'ok', configLoaded: false, configError };
      await showConfigError(configError);
    } else {
      const message = `Could not load status from ${client.baseUrl}: ${errorMessage(err)}`;
      renderBanner(els.statusBanner, message, 'error');
      currentHealth = { status: 'ok', configLoaded: false };
      renderStatus(els.statusPanel, emptyStatus);
      renderMonitors(els.monitorList, []);
      renderPresetOptions(els.presetSelect, []);
      setControlsDisabled(true);
      renderDashboardChrome();
    }
  } finally {
    setLoading(false);
  }
}

async function applySelectedPreset() {
  if (!client) {
    return;
  }

  const preset = els.presetSelect.value;
  if (!preset) {
    return;
  }

  setLoading(true, 'Applying preset…');
  setControlsDisabled(true);
  renderBanner(els.statusBanner, null, 'info');

  try {
    const response = await client.applyPreset(preset, els.dryRun.checked);
    renderApplyResult(els.applyPanel, response);
    els.applyPanel.hidden = false;
    await loadStatus();
  } catch (err) {
    const configError = configErrorFromUnknown(err);
    const status = /** @type {{ status?: number }} */ (err).status;
    if (status === 503 && configError) {
      await showConfigError(configError);
    } else {
      renderBanner(els.statusBanner, `Apply failed: ${errorMessage(err)}`, 'error');
      setControlsDisabled(false);
    }
    setLoading(false);
  }
}

async function main() {
  setControlsDisabled(true);
  setLoading(true, 'Starting…');

  const { baseUrl, bootstrapWarning } = await resolveApiBaseUrl();
  client = createApiClient(baseUrl);
  renderBanner(els.bootstrapBanner, bootstrapWarning, 'warning');

  initConfigDraftPanel();

  els.setupPrimaryCta.addEventListener('click', () => {
    setupSession = markSetupStarted(setupSession);
    renderDashboardChrome();
    scrollToChecklist();
    els.deviceNameInput.focus();
  });

  els.deviceNameInput.addEventListener('input', () => {
    setupSession = setDeviceName(setupSession, els.deviceNameInput.value);
    renderDashboardChrome();
  });

  els.detectBtn.addEventListener('click', () => {
    void detectDisplays();
  });

  els.presetLabelInput?.addEventListener('input', () => {
    setupSession = setPresetLabel(setupSession, els.presetLabelInput.value);
    updateNamingPreviews();
  });

  els.generateDraftBtn.addEventListener('click', () => {
    generateDraftFromSetup();
  });

  els.refreshBtn.addEventListener('click', () => {
    void loadStatus();
  });

  els.applyBtn.addEventListener('click', () => {
    void applySelectedPreset();
  });

  await loadStatus();
}

void main();
