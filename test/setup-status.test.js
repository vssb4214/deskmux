import assert from 'node:assert/strict';
import test from 'node:test';

import {
  deriveSetupStatus,
  getSetupStatusCopy,
  setupStatusBadgeClass,
} from '../src/lib/setup-status.js';

const healthMissing = { configLoaded: false };
const healthLoaded = { configLoaded: true };
const emptySession = {};

test('deriveSetupStatus returns needsSetup when config missing and session empty', () => {
  assert.equal(deriveSetupStatus(healthMissing, emptySession), 'needsSetup');
});

test('deriveSetupStatus returns inProgress when device name entered', () => {
  assert.equal(
    deriveSetupStatus(healthMissing, { deviceName: 'Windows PC' }),
    'inProgress',
  );
});

test('deriveSetupStatus returns restartRequired after save in session', () => {
  assert.equal(
    deriveSetupStatus(healthMissing, { saveSucceeded: true }),
    'restartRequired',
  );
});

test('deriveSetupStatus returns ready when config is loaded', () => {
  assert.equal(
    deriveSetupStatus(healthLoaded, { saveSucceeded: true }),
    'ready',
  );
});

test('getSetupStatusCopy for needsSetup mentions setup', () => {
  const copy = getSetupStatusCopy('needsSetup', {
    configError: 'failed to read config file',
  });
  assert.equal(copy.badge, 'Setup required');
  assert.equal(copy.cta, 'Start setup');
  assert.match(copy.message, /setup checklist/i);
});

test('getSetupStatusCopy for restartRequired mentions restart', () => {
  const copy = getSetupStatusCopy('restartRequired');
  assert.match(copy.message, /reopen DeskMux/i);
});

test('setupStatusBadgeClass maps statuses to badge classes', () => {
  assert.match(setupStatusBadgeClass('ready'), /badge-ok/);
  assert.match(setupStatusBadgeClass('needsSetup'), /badge-error/);
});
