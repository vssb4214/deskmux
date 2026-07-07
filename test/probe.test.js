import assert from 'node:assert/strict';
import test from 'node:test';

import {
  PROBE_DESKTOP_ONLY,
  PROBE_REVERT_DELAY_MS,
  formatProbeCountdownMessage,
  probeErrorMessage,
  startProbeRevertTimer,
} from '../src/lib/probe.js';

/** Fake scheduler: records scheduled calls, lets tests fire or cancel them manually. */
function fakeScheduler() {
  /** @type {{ id: number, fn: () => void, delayMs: number, cleared: boolean }[]} */
  const scheduled = [];
  let nextId = 0;

  return {
    schedule(fn, delayMs) {
      const id = nextId++;
      scheduled.push({ id, fn, delayMs, cleared: false });
      return id;
    },
    clear(id) {
      const entry = scheduled.find((e) => e.id === id);
      if (entry) {
        entry.cleared = true;
      }
    },
    fire(id) {
      const entry = scheduled.find((e) => e.id === id);
      if (entry && !entry.cleared) {
        entry.fn();
      }
    },
    entries: scheduled,
  };
}

test('startProbeRevertTimer schedules onRevert with the given delay', () => {
  const scheduler = fakeScheduler();
  let reverted = false;
  startProbeRevertTimer({
    schedule: scheduler.schedule,
    clear: scheduler.clear,
    onRevert: () => {
      reverted = true;
    },
    delayMs: PROBE_REVERT_DELAY_MS,
  });

  assert.equal(scheduler.entries.length, 1);
  assert.equal(scheduler.entries[0].delayMs, PROBE_REVERT_DELAY_MS);
  assert.equal(reverted, false);
});

test('confirm cancels the timer without calling onRevert', () => {
  const scheduler = fakeScheduler();
  let reverted = false;
  const timer = startProbeRevertTimer({
    schedule: scheduler.schedule,
    clear: scheduler.clear,
    onRevert: () => {
      reverted = true;
    },
    delayMs: 1000,
  });

  timer.confirm();
  scheduler.fire(scheduler.entries[0].id);

  assert.equal(reverted, false);
  assert.equal(scheduler.entries[0].cleared, true);
});

test('revertNow calls onRevert immediately and clears the timer', () => {
  const scheduler = fakeScheduler();
  let revertCount = 0;
  const timer = startProbeRevertTimer({
    schedule: scheduler.schedule,
    clear: scheduler.clear,
    onRevert: () => {
      revertCount += 1;
    },
    delayMs: 1000,
  });

  timer.revertNow();

  assert.equal(revertCount, 1);
  assert.equal(scheduler.entries[0].cleared, true);
});

test('the scheduled timeout firing calls onRevert exactly once', () => {
  const scheduler = fakeScheduler();
  let revertCount = 0;
  startProbeRevertTimer({
    schedule: scheduler.schedule,
    clear: scheduler.clear,
    onRevert: () => {
      revertCount += 1;
    },
    delayMs: 1000,
  });

  scheduler.fire(scheduler.entries[0].id);

  assert.equal(revertCount, 1);
});

test('confirm after revertNow is a no-op — no double clear, no throw', () => {
  const scheduler = fakeScheduler();
  let revertCount = 0;
  const timer = startProbeRevertTimer({
    schedule: scheduler.schedule,
    clear: scheduler.clear,
    onRevert: () => {
      revertCount += 1;
    },
    delayMs: 1000,
  });

  timer.revertNow();
  timer.confirm();

  assert.equal(revertCount, 1);
});

test('revertNow after confirm is a no-op — onRevert never fires', () => {
  const scheduler = fakeScheduler();
  let revertCount = 0;
  const timer = startProbeRevertTimer({
    schedule: scheduler.schedule,
    clear: scheduler.clear,
    onRevert: () => {
      revertCount += 1;
    },
    delayMs: 1000,
  });

  timer.confirm();
  timer.revertNow();

  assert.equal(revertCount, 0);
});

test('the timeout firing after confirm is a no-op', () => {
  const scheduler = fakeScheduler();
  let revertCount = 0;
  const timer = startProbeRevertTimer({
    schedule: scheduler.schedule,
    clear: scheduler.clear,
    onRevert: () => {
      revertCount += 1;
    },
    delayMs: 1000,
  });

  timer.confirm();
  // Simulate a timer that fired before clearTimeout took effect (fake scheduler skips this
  // race by design, so drive it explicitly here).
  scheduler.entries[0].cleared = false;
  scheduler.fire(scheduler.entries[0].id);

  assert.equal(revertCount, 0);
});

test('formatProbeCountdownMessage includes the value being tested', () => {
  const message = formatProbeCountdownMessage(4626);
  assert.ok(message.includes('4626'));
  assert.match(message, /Keep/);
});

test('probeErrorMessage extracts the error field from a structured backend error', () => {
  const message = probeErrorMessage({ error: "display 'GHOST:0000:0' not found", code: 'displayNotFound' });
  assert.equal(message, "display 'GHOST:0000:0' not found");
});

test('probeErrorMessage falls back for plain Error instances', () => {
  assert.equal(probeErrorMessage(new Error('boom')), 'boom');
});

test('probeErrorMessage falls back for unrecognized shapes', () => {
  assert.equal(probeErrorMessage(undefined), 'Testing this input failed.');
  assert.equal(probeErrorMessage('a string'), 'Testing this input failed.');
});

test('desktop-only guard message mentions the desktop app', () => {
  assert.match(PROBE_DESKTOP_ONLY, /desktop app/i);
});
