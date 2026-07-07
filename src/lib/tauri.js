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
