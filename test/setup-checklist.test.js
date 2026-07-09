import assert from 'node:assert/strict';
import test from 'node:test';

import { buildSetupChecklist, getActiveSetupStepId } from '../src/lib/setup-checklist.js';

test('buildSetupChecklist marks name as current on a fresh session', () => {
  const steps = buildSetupChecklist({}, 'needsSetup', {
    isDesktop: true,
    nativeAvailable: true,
  });
  const nameStep = steps.find((step) => step.id === 'name');
  assert.equal(nameStep?.state, 'current');
});

test('buildSetupChecklist marks save current after draft generation', () => {
  const steps = buildSetupChecklist(
    {
      deviceName: 'Windows PC',
      displays: [{ displayId: 'K@P:d0e5:0', label: 'Display 1' }],
      readings: [
        {
          displayId: 'K@P:d0e5:0',
          label: 'Display 1',
          current: 4626,
          maximum: 4626,
        },
      ],
      generatedDraft: true,
    },
    'inProgress',
    { isDesktop: true, nativeAvailable: true },
  );

  const saveStep = steps.find((step) => step.id === 'save');
  assert.equal(saveStep?.state, 'current');
});

test('buildSetupChecklist marks restart current after save', () => {
  const steps = buildSetupChecklist(
    { saveSucceeded: true, generatedDraft: true },
    'restartRequired',
    { isDesktop: true, nativeAvailable: true },
  );

  const restartStep = steps.find((step) => step.id === 'restart');
  assert.equal(restartStep?.state, 'current');
});

test('getActiveSetupStepId returns the current step first', () => {
  const steps = buildSetupChecklist(
    {
      deviceName: 'Windows PC',
      displays: [{ displayId: 'K@P:d0e5:0', label: 'Display 1' }],
    },
    'inProgress',
    { isDesktop: true, nativeAvailable: true },
  );

  assert.equal(getActiveSetupStepId(steps), 'capture');
});

test('getActiveSetupStepId falls back to the first usable step', () => {
  assert.equal(
    getActiveSetupStepId([
      { id: 'name', label: 'Name', state: 'complete', helper: '' },
      { id: 'detect', label: 'Detect', state: 'blocked', helper: '' },
    ]),
    'name',
  );
});
