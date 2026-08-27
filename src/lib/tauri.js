/**
 * @returns {((cmd: string, args?: Record<string, unknown>) => Promise<unknown>) | undefined}
 */
export function tauriInvoke() {
  const invoke = globalThis.__TAURI__?.core?.invoke;
  return typeof invoke === 'function' ? invoke : undefined;
}

export function isTauriDesktop() {
  return tauriInvoke() !== undefined;
}

/**
 * Mirrors `EVENT_OPEN_SETUP_WIZARD` in src-tauri/src/tray.rs.
 */
export const EVENT_OPEN_SETUP_WIZARD = 'deskmux://open-setup-wizard';

/**
 * @returns {((event: string, handler: (payload: unknown) => void) => Promise<unknown>) | undefined}
 */
export function tauriListen() {
  const events = globalThis.__TAURI__?.event;
  const listen = events?.listen;
  return typeof listen === 'function' ? listen.bind(events) : undefined;
}
