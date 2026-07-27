<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { History as HistoryIcon, FolderOpen, RefreshCw, Trash2, ExternalLink } from '@lucide/svelte'
  import { toast } from './toast.svelte'
  import { roving } from './actions/roving'
  import { confirmDialog } from './dialog.svelte'

  type Entry = { file: string; host: string; ts: number; size: number }

  let logs = $state<Entry[]>([])
  let sel = $state<Entry | null>(null)
  let content = $state('')
  let loading = $state(false)

  async function load() {
    logs = await invoke<Entry[]>('logs_list')
    if (sel && !logs.some((l) => l.file === sel!.file)) {
      sel = null
      content = ''
    }
  }
  onMount(load)

  async function open(e: Entry) {
    sel = e
    loading = true
    try {
      content = await invoke<string>('log_read', { file: e.file })
    } catch (err) {
      content = String(err)
    } finally {
      loading = false
    }
  }

  async function reveal(file?: string) {
    try {
      await invoke('log_reveal', { file: file ?? null })
    } catch (e) {
      toast(String(e), 'err')
    }
  }

  async function remove(e: Entry, ev: MouseEvent) {
    ev.stopPropagation()
    const ok = await confirmDialog({ title: 'Delete log', message: `Delete the ${e.host} log from ${fmtDate(e.ts)}?`, okLabel: 'Delete', danger: true })
    if (!ok) return
    await invoke('log_delete', { file: e.file })
    await load()
    toast('Log deleted')
  }

  const fmtDate = (ts: number) => (ts ? new Date(ts * 1000).toLocaleString() : 'unknown date')
  const fmtSize = (b: number) =>
    b < 1024 ? `${b} B` : b < 1048576 ? `${(b / 1024).toFixed(1)} KB` : `${(b / 1048576).toFixed(1)} MB`
</script>

<div class="wrap">
  <header>
    <h1><HistoryIcon size={18} /> History</h1>
    <div class="actions">
      <button class="btn" onclick={load} title="Refresh"><RefreshCw size={14} /> Refresh</button>
      <button class="btn" onclick={() => reveal()}><FolderOpen size={14} /> Logs folder</button>
    </div>
  </header>

  {#if logs.length === 0}
    <div class="empty">
      <HistoryIcon size={30} />
      <p>No session logs yet.</p>
      <p class="muted small">Logs are saved only for hosts with <b>Session logging</b> enabled (in the host's edit dialog). Each connection writes one transcript.</p>
    </div>
  {:else}
    <div class="split">
      <ul class="list" role="group" aria-label="Session logs" use:roving={{ orientation: 'vertical' }}>
        {#each logs as e (e.file)}
          <li>
            <div
              class="row"
              class:sel={sel?.file === e.file}
              role="button"
              tabindex="-1"
              data-roving-item
              aria-label={`${e.host} log from ${fmtDate(e.ts)}`}
              onclick={() => open(e)}
              onkeydown={(ev) => { if (ev.key === 'Enter' || ev.key === ' ') { ev.preventDefault(); open(e) } }}
            >
              <span class="host">{e.host}</span>
              <span class="date muted">{fmtDate(e.ts)}</span>
              <span class="size muted mono">{fmtSize(e.size)}</span>
              <span class="rowacts">
                <button class="ic" data-roving-action title="Reveal in Finder" aria-label="Reveal in Finder" onclick={(ev) => { ev.stopPropagation(); reveal(e.file) }}><ExternalLink size={13} /></button>
                <button class="ic danger" data-roving-action title="Delete" aria-label="Delete log" onclick={(ev) => remove(e, ev)}><Trash2 size={13} /></button>
              </span>
            </div>
          </li>
        {/each}
      </ul>
      <div class="viewer">
        {#if !sel}
          <div class="hint muted">Select a log to view its transcript.</div>
        {:else if loading}
          <div class="hint muted">Loading…</div>
        {:else}
          <pre class="mono">{content}</pre>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .wrap {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  header {
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 16px;
    border-bottom: 1px solid hsl(var(--border));
  }
  h1 {
    display: flex;
    align-items: center;
    gap: 9px;
    margin: 0;
    font-size: 16px;
  }
  .actions {
    margin-left: auto;
    display: flex;
    gap: 8px;
  }
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 7px 12px;
    border: 1px solid hsl(var(--border));
    border-radius: 7px;
    background: hsl(var(--muted));
    color: inherit;
    font-size: 12.5px;
    font-family: inherit;
    cursor: pointer;
  }
  .btn:hover {
    background: hsl(var(--border));
  }
  .empty {
    margin: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    text-align: center;
    color: hsl(var(--muted-foreground));
    padding: 10vh 20px;
  }
  .empty .small {
    max-width: 380px;
  }
  .split {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(280px, 360px) 1fr;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 6px;
    overflow: auto;
    border-right: 1px solid hsl(var(--border));
  }
  .row {
    width: 100%;
    display: grid;
    grid-template-columns: 1fr auto;
    grid-template-areas: 'host acts' 'date size';
    gap: 2px 8px;
    align-items: center;
    padding: 8px 10px;
    border: none;
    background: none;
    color: inherit;
    text-align: left;
    border-radius: 8px;
    cursor: pointer;
    font-family: inherit;
  }
  .row:hover {
    background: hsl(var(--muted));
  }
  .row.sel {
    background: hsl(var(--primary) / 0.1);
    box-shadow: inset 2px 0 0 hsl(var(--primary));
  }
  .row:focus-visible {
    outline: none;
    background: hsl(var(--primary) / 0.14);
    box-shadow: inset 2px 0 0 hsl(var(--primary));
  }
  .host {
    grid-area: host;
    font-size: 13px;
    font-weight: 500;
  }
  .date {
    grid-area: date;
    font-size: 11.5px;
  }
  .size {
    grid-area: size;
    font-size: 11px;
    text-align: right;
  }
  .rowacts {
    grid-area: acts;
    display: flex;
    gap: 2px;
    opacity: 0;
  }
  .row:hover .rowacts,
  .row.sel .rowacts,
  .row:focus-within .rowacts {
    opacity: 1;
  }
  .ic {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: none;
    background: none;
    border-radius: 6px;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
  }
  .ic:hover {
    background: hsl(var(--border));
    color: hsl(var(--foreground));
  }
  .ic.danger:hover {
    color: hsl(var(--destructive));
  }
  .viewer {
    min-width: 0;
    overflow: auto;
    background: hsl(var(--background));
  }
  .hint {
    padding: 24px;
    font-size: 13px;
  }
  pre {
    margin: 0;
    padding: 14px 16px;
    font-size: 12px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
