const DEFAULT_API_BASE = 'http://127.0.0.1:3737';

/**
 * @returns {Promise<{ baseUrl: string, bootstrapWarning: string | null }>}
 */
export async function resolveApiBaseUrl() {
  const invoke = globalThis.__TAURI__?.core?.invoke;
  if (typeof invoke === 'function') {
    try {
      /** @type {string} */
      const baseUrl = await invoke('get_api_base_url');
      return { baseUrl, bootstrapWarning: null };
    } catch {
      return {
        baseUrl: DEFAULT_API_BASE,
        bootstrapWarning:
          'Could not read API URL from DeskMux — using default http://127.0.0.1:3737.',
      };
    }
  }

  return {
    baseUrl: DEFAULT_API_BASE,
    bootstrapWarning:
      'DeskMux API URL unavailable outside the app — using default http://127.0.0.1:3737.',
  };
}

export { DEFAULT_API_BASE };
