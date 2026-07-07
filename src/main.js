import { resolveApiBaseUrl } from './api/bootstrap.js';

import { createApiClient } from './api/client.js';

import { configErrorFromUnknown } from './lib/config-error.js';

import {

  discoveryErrorMessage,

  formatInputSourceReading,

} from './lib/discovery.js';

import {

  renderApplyResult,

  renderBanner,

  renderConfigErrorBanner,

  renderDiscoveryPanel,

  renderMonitors,

  renderPresetOptions,

  renderStatus,

  renderEvents,

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

  eventsPanel: document.getElementById('events-panel'),

  discoveryPanel: document.getElementById('discovery-panel'),

  detectBtn: /** @type {HTMLButtonElement} */ (document.getElementById('detect-btn')),

  loading: document.getElementById('loading'),

};



const emptyStatus = {

  deviceName: '—',

  presets: [],

  monitors: [],

  lastAppliedPreset: null,

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



/**

 * @param {string | undefined} configError

 */

async function showConfigError(configError) {

  renderConfigErrorBanner(els.statusBanner, configError);

  renderStatus(els.statusPanel, emptyStatus);

  renderMonitors(els.monitorList, []);

  renderPresetOptions(els.presetSelect, []);

  setControlsDisabled(true);

  await loadEvents();

}



async function loadEvents() {

  if (!client || !els.eventsPanel) {

    return;

  }



  try {

    const { events } = await client.fetchEvents();

    renderEvents(els.eventsPanel, events);

  } catch {

    renderEvents(els.eventsPanel, []);

  }

}



async function readDisplayInput(displayId, readingEl, buttonEl) {

  if (!client) {

    return;

  }



  buttonEl.disabled = true;

  readingEl.textContent = 'Reading…';



  try {

    const reading = await client.fetchInputSource(displayId);

    readingEl.textContent = formatInputSourceReading(reading);

    readingEl.classList.remove('muted');

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

    renderDiscoveryPanel(els.discoveryPanel, data, (displayId, readingEl, buttonEl) => {

      void readDisplayInput(displayId, readingEl, buttonEl);

    });

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



async function loadStatus() {

  if (!client) {

    return;

  }



  setLoading(true);

  renderBanner(els.statusBanner, null, 'info');

  setControlsDisabled(true);



  try {

    const health = await client.fetchHealth();

    if (!health.configLoaded) {

      await showConfigError(health.configError);

      return;

    }



    /** @type {StatusResponse} */

    const status = await client.fetchStatus();

    renderBanner(els.statusBanner, null, 'info');

    renderStatus(els.statusPanel, status);

    renderMonitors(els.monitorList, status.monitors);

    renderPresetOptions(els.presetSelect, status.presets);

    setControlsDisabled(false);

    await loadEvents();

  } catch (err) {

    const configError = configErrorFromUnknown(err);

    const status = /** @type {{ status?: number }} */ (err).status;

    if (status === 503 && configError) {

      await showConfigError(configError);

    } else {

      const message = `Could not load status from ${client.baseUrl}: ${errorMessage(err)}`;

      renderBanner(els.statusBanner, message, 'error');

      renderStatus(els.statusPanel, emptyStatus);

      renderMonitors(els.monitorList, []);

      renderPresetOptions(els.presetSelect, []);

      setControlsDisabled(true);

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

    await loadStatus();

  } catch (err) {

    const configError = configErrorFromUnknown(err);

    const status = /** @type {{ status?: number }} */ (err).status;

    if (status === 503 && configError) {

      await showConfigError(configError);

    } else {

      renderBanner(

        els.statusBanner,

        `Apply failed: ${errorMessage(err)}`,

        'error',

      );

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



  await loadStatus();



  els.refreshBtn.addEventListener('click', () => {

    void loadStatus();

  });

  els.applyBtn.addEventListener('click', () => {

    void applySelectedPreset();

  });

  // Deliberately outside setControlsDisabled: discovery works without a loaded config —

  // first run is exactly when it matters.

  els.detectBtn.addEventListener('click', () => {

    void detectDisplays();

  });

}



void main();

