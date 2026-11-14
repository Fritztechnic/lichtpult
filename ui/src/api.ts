// Verbindung zur Engine: Tauri-Command in der Desktop-App, sonst HTTP zum Entwicklungsserver.
const tauri = '__TAURI_INTERNALS__' in window

export async function call<T = any>(method: string, body: unknown = null): Promise<T> {
  const b = JSON.stringify(body)
  if (tauri) {
    const { invoke } = await import('@tauri-apps/api/core')
    return JSON.parse(await invoke<string>('api', { method, body: b }))
  }
  const r = await fetch('/api/' + method, { method: 'POST', body: b })
  const t = await r.text()
  if (!r.ok) throw new Error(t)
  return JSON.parse(t)
}
