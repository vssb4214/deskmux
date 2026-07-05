/**
 * @typedef {{
 *   status: string,
 *   configLoaded: boolean,
 *   configError?: string
 * }} HealthResponse
 * @typedef {{ error: string, configError?: string }} ApiErrorResponse
 * @typedef {{ name: string, label: string }} PresetSummary
 * @typedef {{ id: string, label: string, order: number }} MonitorSummary
 * @typedef {{
 *   deviceName: string,
 *   presets: PresetSummary[],
 *   monitors: MonitorSummary[],
 *   lastAppliedPreset: string | null
 * }} StatusResponse
 * @typedef {{ type: 'unknownMonitor', monitorId: string }} PlanningError
 * @typedef {{ type: 'dryRun' }} MonitorOutcomeDryRun
 * @typedef {{ type: 'success', stdout: string, stderr: string }} MonitorOutcomeSuccess
 * @typedef {{ type: 'failed', stdout: string, stderr: string, exitCode: number | null }} MonitorOutcomeFailed
 * @typedef {{ type: 'spawnFailed', message: string }} MonitorOutcomeSpawnFailed
 * @typedef {{ type: 'unknownMonitor', monitorId: string } | { type: 'unknownDevice', monitorId: string, deviceId: string }} ResolutionError
 * @typedef {{ type: 'resolutionFailed', error: ResolutionError }} MonitorOutcomeResolutionFailed
 * @typedef {MonitorOutcomeDryRun | MonitorOutcomeSuccess | MonitorOutcomeFailed | MonitorOutcomeSpawnFailed | MonitorOutcomeResolutionFailed} MonitorOutcome
 * @typedef {{
 *   monitorId: string,
 *   deviceId: string,
 *   command: string | null,
 *   executed: boolean,
 *   isNativeDdc: boolean,
 *   outcome: MonitorOutcome
 * }} MonitorResult
 * @typedef {{ host: string, port: number }} PeerRef
 * @typedef {{
 *   type: 'success',
 *   localOnly: boolean,
 *   results: MonitorResult[],
 *   peerResults?: PeerApplyOutcome[]
 * }} PeerOutcomeSuccess
 * @typedef {{ type: 'failed', error: string, httpStatus?: number }} PeerOutcomeFailed
 * @typedef {PeerOutcomeSuccess | PeerOutcomeFailed} PeerOutcome
 * @typedef {{
 *   deviceId: string,
 *   peer: PeerRef | null,
 *   outcome: PeerOutcome
 * }} PeerApplyOutcome
 * @typedef {{
 *   preset: string,
 *   dryRun: boolean,
 *   localOnly: boolean,
 *   planningErrors: PlanningError[],
 *   localResults: MonitorResult[],
 *   peerResults: PeerApplyOutcome[]
 * }} ApplyPresetResponse
 * @typedef {'info' | 'success' | 'error'} EventKind
 * @typedef {'api' | 'tray' | 'hotkey'} ApplySource
 * @typedef {{
 *   timestampMs: number,
 *   kind: EventKind,
 *   message: string,
 *   preset?: string,
 *   source?: ApplySource,
 *   monitorId?: string
 * }} DeskMuxEvent
 * @typedef {{ events: DeskMuxEvent[] }} EventsResponse
 * @typedef {'dry-run' | 'planning-failed' | 'success' | 'partial' | 'failed'} ApplySummaryClass
 */

export {};
