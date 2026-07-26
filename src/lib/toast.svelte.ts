// Lightweight toast notifications. Call toast('Saved') after any user action
// that completes silently. Auto-dismiss; no dependency, no config.

export type Toast = { id: number; msg: string; kind: 'ok' | 'err' }

let seq = 0
export const toasts = $state<{ list: Toast[] }>({ list: [] })

export function toast(msg: string, kind: 'ok' | 'err' = 'ok') {
  const id = ++seq
  toasts.list.push({ id, msg, kind })
  // ponytail: fixed 2.6s dismiss; add hover-to-persist only if users ask.
  setTimeout(() => {
    toasts.list = toasts.list.filter((t) => t.id !== id)
  }, 2600)
}
