import assert from 'node:assert/strict';
import test from 'node:test';

import {
  classifyApplyResult,
  monitorOutcomeOk,
  peerApplyOutcomeOk,
  summaryBannerText,
} from '../src/lib/summary.js';

const successOutcome = { type: 'success', stdout: '', stderr: '' };
const failedOutcome = {
  type: 'failed',
  stdout: '',
  stderr: 'nope',
  exitCode: 1,
};

test('monitorOutcomeOk accepts dryRun and success', () => {
  assert.equal(monitorOutcomeOk({ type: 'dryRun' }), true);
  assert.equal(monitorOutcomeOk(successOutcome), true);
  assert.equal(monitorOutcomeOk(failedOutcome), false);
});

test('classifyApplyResult returns dry-run', () => {
  assert.equal(
    classifyApplyResult({
      dryRun: true,
      planningErrors: [],
      localResults: [],
      peerResults: [],
    }),
    'dry-run',
  );
});

test('classifyApplyResult returns planning-failed', () => {
  assert.equal(
    classifyApplyResult({
      dryRun: false,
      planningErrors: [{ type: 'unknownMonitor', monitorId: 'm1' }],
      localResults: [],
      peerResults: [],
    }),
    'planning-failed',
  );
});

test('classifyApplyResult returns success when all outcomes ok', () => {
  assert.equal(
    classifyApplyResult({
      dryRun: false,
      planningErrors: [],
      localResults: [
        {
          monitorId: 'm1',
          deviceId: 'd1',
          command: 'exit 0',
          executed: true,
          outcome: successOutcome,
        },
      ],
      peerResults: [
        {
          deviceId: 'peer',
          peer: null,
          outcome: {
            type: 'success',
            localOnly: true,
            results: [
              {
                monitorId: 'm2',
                deviceId: 'd2',
                command: 'exit 0',
                executed: true,
                outcome: successOutcome,
              },
            ],
            peerResults: [],
          },
        },
      ],
    }),
    'success',
  );
});

test('classifyApplyResult returns partial on mixed monitor results', () => {
  assert.equal(
    classifyApplyResult({
      dryRun: false,
      planningErrors: [],
      localResults: [
        {
          monitorId: 'm1',
          deviceId: 'd1',
          command: 'exit 0',
          executed: true,
          outcome: successOutcome,
        },
        {
          monitorId: 'm2',
          deviceId: 'd2',
          command: 'exit 1',
          executed: true,
          outcome: failedOutcome,
        },
      ],
      peerResults: [],
    }),
    'partial',
  );
});

test('classifyApplyResult returns failed when peer http fails', () => {
  assert.equal(
    classifyApplyResult({
      dryRun: false,
      planningErrors: [],
      localResults: [],
      peerResults: [
        {
          deviceId: 'peer',
          peer: { host: '127.0.0.1', port: 3737 },
          outcome: { type: 'failed', error: 'boom', httpStatus: 404 },
        },
      ],
    }),
    'failed',
  );
});

test('peerApplyOutcomeOk fails on nested monitor failure', () => {
  assert.equal(
    peerApplyOutcomeOk({
      deviceId: 'peer',
      peer: null,
      outcome: {
        type: 'success',
        localOnly: true,
        results: [
          {
            monitorId: 'm2',
            deviceId: 'd2',
            command: 'exit 1',
            executed: true,
            outcome: failedOutcome,
          },
        ],
      },
    }),
    false,
  );
});

test('summaryBannerText mentions dry run and last applied preset', () => {
  assert.match(summaryBannerText('dry-run'), /not updated/i);
  assert.match(summaryBannerText('partial'), /not updated/i);
});
