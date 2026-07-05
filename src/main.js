import { resolveApiBaseUrl } from './api/bootstrap.js';
import { createApiClient } from './api/client.js';
import {
  renderApplyResult,
  renderBanner,
  renderMonitors,
  renderPresetOptions,
  renderStatus,
} from './ui/render.js';

/** @typedef {import('./types.js').StatusResponse} StatusResponse */

/** @type {ReturnType<typeof createApiClient> | null} */
let client = null;

const els = {
  bootstrapBanner: document.getElementById('bootstrap-banner'),
  statusBanner: document.getElementById('status-banner'),
  statusPanel: document.getElementById('status-panel'),
  monitorList: document.getElementById('monitor-list'),
  presetSelect: /** @type {HTMLSelectElement} */ (document.getElementById('preset-select')),
  dryRun: /** @type {HTMLInputElement} */ (document.getElementById('dry-run')),
  applyBtn: /** @type {HTMLButtonElement} */ (document.getElementById('apply-btn')),
  refreshBtn: /** @type {HTMLButtonElement} */ (document.getElementById('refresh-btn')),
  applyPanel: document.getElementById('apply-panel'),
  loading: document.getElementById('loading'),
};

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

async function loadStatus() {
  if (!client) {
    return;
  }

  setLoading(true);
  renderBanner(els.statusBanner, null, 'info');
  setControlsDisabled(true);

  try {
    /** @type {StatusResponse} */
    const status = await client.fetchStatus();
    renderStatus(els.statusPanel, status);
    renderMonitors(els.monitorList, status.monitors);
    renderPresetOptions(els.presetSelect, status.presets);
    setControlsDisabled(false);
  } catch (err) {
    const status = /** @type {{ status?: number }} */ (err).status;
    const message =
      status === 503
        ? 'Config not loaded — fix deskmux.config.json and restart DeskMux.'
        : `Could not load status from ${client.baseUrl}: ${errorMessage(err)}`;
    renderBanner(els.statusBanner, message, 'error');
    renderStatus(els.statusPanel, {
      deviceName: '—',
      presets: [],
      monitors: [],
      lastAppliedPreset: null,
    });
    renderMonitors(els.monitorList, []);
    renderPresetOptions(els.presetSelect, []);
    setControlsDisabled(true);
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
    await loadStatus();
  } catch (err) {
    renderBanner(
      els.statusBanner,
      `Apply failed: ${errorMessage(err)}`,
      'error',
    );
    setControlsDisabled(false);
    setLoading(false);
  }
}

async function main() {
  setControlsDisabled(true);
  setLoading(true, 'Starting…');

  const { baseUrl, bootstrapWarning } = await resolveApiBaseUrl();
  client = createApiClient(baseUrl);
  renderBanner(els.bootstrapBanner, bootstrapWarning, 'warning');

  await loadStatus();

  els.refreshBtn.addEventListener('click', () => {
    void loadStatus();
  });
  els.applyBtn.addEventListener('click', () => {
    void applySelectedPreset();
  });
}

void main();
