<script lang="ts">
  import { Download, Upload, Copy, Check } from '@lucide/svelte'
  import {
    importSshConfig,
    exportSshConfig,
    writeSshConfig,
    saveHost,
    type Host,
  } from './state.svelte'

  let { mode, onclose }: { mode: 'import' | 'export'; onclose: () => void } = $props()

  let busy = $state(false)
  let err = $state('')

  // import
  let path = $state('~/.ssh/config')
  let parsed = $state<Host[] | null>(null)
  let selected = $state<Set<string>>(new Set())

  // export
  let text = $state('')
  let outPath = $state('~/ssh_config.exported')
  let copied = $state(false)
  let saved = $state(false)

  async function scan() {
    busy = true
    err = ''
    try {
      const hosts = await importSshConfig(path)
      parsed = hosts
      selected = new Set(hosts.map((h) => h.id))
    } catch (e) {
      err = String(e)
      parsed = null
    } finally {
      busy = false
    }
  }

  function toggle(id: string) {
    const s = new Set(selected)
    s.has(id) ? s.delete(id) : s.add(id)
    selected = s
  }

  async function doImport() {
    if (!parsed) return
    busy = true
    err = ''
    try {
      for (const h of parsed) if (selected.has(h.id)) await saveHost(h)
      onclose()
    } catch (e) {
      err = String(e)
    } finally {
      busy = false
    }
  }

  async function loadExport() {
    busy = true
    err = ''
    try {
      text = await exportSshConfig()
    } catch (e) {
      err = String(e)
    } finally {
      busy = false
    }
  }
  if (mode === 'export') loadExport()

  function copy() {
    navigator.clipboard?.writeText(text)
    copied = true
    setTimeout(() => (copied = false), 1500)
  }

  async function save() {
    busy = true
    err = ''
    try {
      await writeSshConfig(outPath, text)
      saved = true
      setTimeout(() => (saved = false), 1800)
    } catch (e) {
      err = String(e)
    } finally {
      busy = false
    }
  }
</script>

<div class="backdrop" onclick={onclose} role="presentation">
  <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
    {#if mode === 'import'}
      <div class="mh"><Upload size={15} /> Import from ssh_config</div>
      <div class="mbody">
        <div class="pathrow">
          <input class="mono" bind:value={path} placeholder="~/.ssh/config" />
          <button class="btn" onclick={scan} disabled={busy}>Scan</button>
        </div>
        {#if err}<div class="err">{err}</div>{/if}
        {#if parsed}
          {#if parsed.length === 0}
            <p class="muted small">No concrete hosts found in that file.</p>
          {:else}
            <p class="muted small">{selected.size} of {parsed.length} selected</p>
            <ul class="list">
              {#each parsed as h (h.id)}
                <li>
                  <label class="chk">
                    <input type="checkbox" checked={selected.has(h.id)} onchange={() => toggle(h.id)} />
                    <span class="name">{h.name}</span>
                    <span class="fp mono">{h.user ? h.user + '@' : ''}{h.hostname}{h.port !== 22 ? ':' + h.port : ''}</span>
                    {#if h.auth === 'key'}<span class="badge">key</span>{/if}
                    {#if h.jumps.length}<span class="badge">jump: {h.jumps.join(',')}</span>{/if}
                  </label>
                </li>
              {/each}
            </ul>
          {/if}
        {/if}
      </div>
      <div class="mfoot">
        <button class="btn ghost" onclick={onclose}>Cancel</button>
        <button class="btn primary" onclick={doImport} disabled={busy || !parsed || selected.size === 0}>
          Import {selected.size || ''} selected
        </button>
      </div>
    {:else}
      <div class="mh"><Download size={15} /> Export to ssh_config</div>
      <div class="mbody">
        {#if err}<div class="err">{err}</div>{/if}
        <textarea class="mono" readonly rows="12" value={text}></textarea>
        <div class="pathrow">
          <input class="mono" bind:value={outPath} placeholder="~/ssh_config.exported" />
          <button class="btn" onclick={save} disabled={busy || !text}>
            {#if saved}<Check size={14} /> Saved{:else}Save to path{/if}
          </button>
        </div>
      </div>
      <div class="mfoot">
        <button class="btn ghost" onclick={onclose}>Close</button>
        <button class="btn primary" onclick={copy} disabled={!text}>
          {#if copied}<Check size={14} /> Copied{:else}<Copy size={14} /> Copy{/if}
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: grid;
    place-items: start center;
    padding-top: 10vh;
    z-index: 60;
  }
  .modal {
    width: 560px;
    max-width: 92vw;
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border));
    border-radius: 12px;
    overflow: hidden;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.55);
  }
  .mh {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 14px 18px;
    border-bottom: 1px solid hsl(var(--border));
    font-size: 14px;
    font-weight: 600;
  }
  .mbody {
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-height: 60vh;
    overflow: auto;
  }
  .pathrow {
    display: flex;
    gap: 8px;
  }
  .pathrow input {
    flex: 1;
  }
  input,
  textarea {
    background: hsl(var(--muted));
    border: 1px solid hsl(var(--border));
    border-radius: 7px;
    padding: 8px 10px;
    color: inherit;
    outline: none;
    font-size: 13px;
    font-family: inherit;
    resize: vertical;
  }
  input:focus,
  textarea:focus {
    border-color: hsl(var(--ring) / 0.6);
  }
  .list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .chk {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 9px;
    border: 1px solid hsl(var(--border));
    border-radius: 7px;
    background: hsl(var(--background));
    cursor: pointer;
    font-size: 12.5px;
  }
  .chk input {
    width: auto;
    padding: 0;
  }
  .name {
    font-weight: 600;
  }
  .fp {
    color: hsl(var(--muted-foreground));
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .badge {
    padding: 2px 7px;
    border-radius: 5px;
    background: hsl(var(--muted));
    border: 1px solid hsl(var(--border));
    font-size: 11px;
  }
  .small {
    font-size: 12px;
  }
  .err {
    color: hsl(var(--destructive));
    font-size: 12.5px;
  }
  .mfoot {
    padding: 13px 18px;
    border-top: 1px solid hsl(var(--border));
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 13px;
    border: none;
    border-radius: 7px;
    background: hsl(var(--muted));
    color: inherit;
    font-size: 13px;
    font-family: inherit;
    cursor: pointer;
  }
  .btn:hover {
    background: hsl(var(--border));
  }
  .btn.ghost {
    background: transparent;
  }
  .btn.primary {
    background: hsl(var(--primary));
    color: hsl(var(--primary-foreground));
    font-weight: 600;
  }
  .btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
