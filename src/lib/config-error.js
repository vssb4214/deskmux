export const CONFIG_FILE_HINT =
  'Check deskmux.config.json in the DeskMux app directory, fix the issue, and restart DeskMux.';

/**
 * @param {string | undefined | null} configError
 * @returns {string}
 */
export function formatConfigErrorBannerText(configError) {
  const detail = configError?.trim() || 'Unknown configuration error.';
  return `Config not loaded\n\n${detail}\n\n${CONFIG_FILE_HINT}`;
}

/**
 * @param {Record<string, unknown>} body
 * @param {number} status
 * @returns {Error & { status: number, configError?: string }}
 */
export function parseApiError(body, status) {
  const message =
    typeof body.error === 'string' ? body.error : 'Request failed';
  /** @type {Error & { status: number, configError?: string }} */
  const error = new Error(message);
  error.status = status;
  if (typeof body.configError === 'string') {
    error.configError = body.configError;
  }
  return error;
}

/**
 * @param {unknown} err
 * @returns {string | undefined}
 */
export function configErrorFromUnknown(err) {
  if (err && typeof err === 'object' && 'configError' in err) {
    const value = /** @type {{ configError?: unknown }} */ (err).configError;
    if (typeof value === 'string') {
      return value;
    }
  }
  return undefined;
}
