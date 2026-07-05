/** @typedef {import('../types.js').MonitorOutcome} MonitorOutcome */
/** @typedef {import('../types.js').PeerApplyOutcome} PeerApplyOutcome */
/** @typedef {import('../types.js').PeerOutcome} PeerOutcome */
/** @typedef {import('../types.js').ApplyPresetResponse} ApplyPresetResponse */
/** @typedef {import('../types.js').ApplySummaryClass} ApplySummaryClass */

/**
 * @param {MonitorOutcome} outcome
 */
export function monitorOutcomeOk(outcome) {
  return outcome?.type === 'dryRun' || outcome?.type === 'success';
}

/**
 * @param {PeerOutcome} outcome
 */
export function peerOutcomeOk(outcome) {
  if (outcome.type === 'failed') {
    return false;
  }
  const resultsOk = outcome.results.every((result) =>
    monitorOutcomeOk(result.outcome),
  );
  const nested = outcome.peerResults ?? [];
  const nestedOk = nested.every((peer) => peerApplyOutcomeOk(peer));
  return resultsOk && nestedOk;
}

/**
 * @param {PeerApplyOutcome} peer
 */
export function peerApplyOutcomeOk(peer) {
  return peerOutcomeOk(peer.outcome);
}

/**
 * @param {ApplyPresetResponse} response
 * @returns {ApplySummaryClass}
 */
export function classifyApplyResult(response) {
  if (response.dryRun) {
    return 'dry-run';
  }
  if (response.planningErrors?.length > 0) {
    return 'planning-failed';
  }

  const localResults = response.localResults ?? [];
  const peerResults = response.peerResults ?? [];
  const localStatuses = localResults.map((result) =>
    monitorOutcomeOk(result.outcome),
  );
  const peerStatuses = peerResults.map((peer) => peerApplyOutcomeOk(peer));
  const allOk =
    localStatuses.every(Boolean) && peerStatuses.every(Boolean);
  const anyOk =
    localStatuses.some(Boolean) || peerStatuses.some(Boolean);

  if (allOk) {
    return 'success';
  }
  if (anyOk) {
    return 'partial';
  }
  return 'failed';
}

/**
 * @param {ApplySummaryClass} summaryClass
 */
export function summaryBannerText(summaryClass) {
  switch (summaryClass) {
    case 'dry-run':
      return 'Dry run — commands were not executed. Last applied preset was not updated.';
    case 'success':
      return 'Preset applied successfully.';
    case 'partial':
      return 'Partial failure — last applied preset was not updated.';
    case 'planning-failed':
      return 'Planning failed — preset was not applied.';
    case 'failed':
      return 'Apply failed — last applied preset was not updated.';
    default:
      return '';
  }
}
