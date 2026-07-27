// Promise-based, in-app replacements for the browser's blocking confirm()/prompt()
// so every yes/no or text-entry flow stays inside the app's keyboard model
// (focus-trapped, Escape-cancel, Enter-submit). A single <DialogHost/> mounted in
// App renders whatever is current; these helpers set it and resolve on close.

type Base = { title: string; message?: string; okLabel?: string; danger?: boolean }

export type DialogRequest =
  | (Base & { kind: 'confirm'; resolve: (ok: boolean) => void })
  | (Base & {
      kind: 'prompt'
      value: string
      placeholder?: string
      resolve: (value: string | null) => void
    })

export const dialogState = $state<{ current: DialogRequest | null }>({ current: null })

// Ask a yes/no question. Resolves true on OK, false on Cancel/Escape.
export function confirmDialog(o: Base): Promise<boolean> {
  return new Promise((resolve) => {
    dialogState.current = { kind: 'confirm', ...o, resolve }
  })
}

// Ask for a line of text. Resolves the trimmed string on OK, or null on cancel.
export function promptDialog(o: Base & { value?: string; placeholder?: string }): Promise<string | null> {
  return new Promise((resolve) => {
    dialogState.current = { kind: 'prompt', value: o.value ?? '', ...o, resolve }
  })
}

// Called by DialogHost when the user answers; clears and resolves.
export function settleDialog(result: boolean | string | null) {
  const req = dialogState.current
  dialogState.current = null
  if (!req) return
  if (req.kind === 'confirm') req.resolve(result === true)
  else req.resolve(typeof result === 'string' ? result : null)
}
