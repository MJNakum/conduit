<script lang="ts">
  import { Zap, Plus, Play, Copy, Pencil, Trash2, Check } from '@lucide/svelte'
  import {
    ui,
    snippetsStore,
    blankSnippet,
    saveSnippet,
    deleteSnippet,
    snippetVars,
    fillSnippet,
    runInActiveSession,
    type Snippet,
  } from './state.svelte'

  let editing = $state<Snippet | null>(null)
  // Run-with-parameters state.
  let running = $state<Snippet | null>(null)
  let values = $state<Record<string, string>>({})
  let note = $state('')
  let copiedId = $state('')

  const preview = (cmd: string) => cmd.split('\n')[0].slice(0, 80) + (cmd.length > 80 ? '…' : '')

  function startRun(s: Snippet) {
    note = ''
    const vars = snippetVars(s.command)
    if (vars.length) {
      values = Object.fromEntries(vars.map((v) => [v, '']))
      running = s
    } else {
      execute(s, s.command)
    }
  }

  function confirmRun() {
    if (!running) return
    execute(running, fillSnippet(running.command, values))
    running = null
  }

  function execute(s: Snippet, command: string) {
    if (s.confirm && !confirm(`Run "${s.name}"?\n\n${command}`)) return
    if (!runInActiveSession(command)) note = 'No active session — open and connect a host first.'
  }

  function copy(s: Snippet) {
    navigator.clipboard?.writeText(s.command)
    copiedId = s.id
    setTimeout(() => (copiedId = ''), 1200)
  }

  async function save() {
    if (!editing) return
    await saveSnippet({ ...editing })
    editing = null
  }
</script>

<div class="wrap">
  <header>
    <h1><Zap size={18} /> Snippets</h1>
    <button class="btn primary" onclick={() => (editing = blankSnippet())}><Plus size={14} /> New snippet</button>
  </header>

  {#if !ui.lastSession}
    <p class="muted small">Tip: Run sends into your most recently active session. Connect a host first.</p>
  {/if}
  {#if note}<div class="err">{note}</div>{/if}

  {#if snippetsStore.list.length === 0}
    <div class="empty"><Zap size={30} /><p>No snippets yet. Save commands you run often.</p></div>
  {:else}
    <ul class="list">
      {#each snippetsStore.list as s (s.id)}
        {@const vars = snippetVars(s.command)}
        <li>
          <span class="name">{s.name || '(unnamed)'}</span>
          <code class="cmd mono">{preview(s.command)}</code>
          {#if vars.length}<span class="badge">{vars.length} var{vars.length === 1 ? '' : 's'}</span>{/if}
          {#if s.confirm}<span class="badge warn">confirm</span>{/if}
          <span class="acts">
            <button class="icon run" title="Run in active session" onclick={() => startRun(s)}><Play size={14} /></button>
            <button class="icon" title="Copy" onclick={() => copy(s)}>{#if copiedId === s.id}<Check size={14} />{:else}<Copy size={14} />{/if}</button>
            <button class="icon" title="Edit" onclick={() => (editing = { ...s })}><Pencil size={14} /></button>
            <button class="icon danger" title="Delete" onclick={() => deleteSnippet(s.id)}><Trash2 size={14} /></button>
          </span>
        </li>
      {/each}
    </ul>
  {/if}
</div>

{#if editing}
  <div class="backdrop" onclick={() => (editing = null)} role="presentation">
    <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
      <div class="mh"><Zap size={15} /> {editing.name ? 'Edit snippet' : 'New snippet'}</div>
      <div class="mbody">
        <div class="field"><label for="s-name">Name</label><input id="s-name" bind:value={editing.name} placeholder="restart service" /></div>
        <div class="field">
          <label for="s-cmd">Command</label>
          <textarea id="s-cmd" class="mono" rows="5" bind:value={editing.command} placeholder="sudo systemctl restart {'{{service}}'}"></textarea>
          <p class="help">Use <code class="mono">{'{{name}}'}</code> tokens for values you fill in at run time.</p>
        </div>
        <label class="check"><input type="checkbox" bind:checked={editing.confirm} /> Confirm before running</label>
      </div>
      <div class="mfoot">
        <button class="btn ghost" onclick={() => (editing = null)}>Cancel</button>
        <button class="btn primary" onclick={save} disabled={!editing.name || !editing.command.trim()}>Save</button>
      </div>
    </div>
  </div>
{/if}

{#if running}
  <div class="backdrop" onclick={() => (running = null)} role="presentation">
    <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
      <div class="mh"><Play size={15} /> Run {running.name}</div>
      <div class="mbody">
        {#each snippetVars(running.command) as v (v)}
          <div class="field"><label for={'v-' + v}>{v}</label><input id={'v-' + v} class="mono" bind:value={values[v]} /></div>
        {/each}
        <p class="help mono">{fillSnippet(running.command, values)}</p>
      </div>
      <div class="mfoot">
        <button class="btn ghost" onclick={() => (running = null)}>Cancel</button>
        <button class="btn primary" onclick={confirmRun}><Play size={13} /> Run</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .wrap { flex: 1; min-height: 0; overflow: auto; padding: 18px 22px; }
  header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 16px; }
  h1 { display: flex; align-items: center; gap: 9px; font-size: 17px; font-weight: 600; }
  .small { font-size: 12px; margin: 0 0 12px; }
  .list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 4px; }
  .list li { display: flex; align-items: center; gap: 12px; padding: 8px 12px; border: 1px solid hsl(var(--border)); border-radius: 8px; background: hsl(var(--card)); font-size: 12.5px; }
  .name { font-weight: 600; flex: none; min-width: 120px; }
  .cmd { color: hsl(var(--muted-foreground)); flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .badge { padding: 2px 7px; border-radius: 5px; background: hsl(var(--muted)); border: 1px solid hsl(var(--border)); font-size: 11px; flex: none; }
  .badge.warn { color: hsl(var(--amber)); }
  .acts { display: flex; gap: 3px; flex: none; }
  .icon { display: grid; place-items: center; width: 27px; height: 27px; border: none; background: none; border-radius: 6px; color: hsl(var(--muted-foreground)); cursor: pointer; }
  .icon:hover { background: hsl(var(--muted)); color: hsl(var(--foreground)); }
  .icon.run:hover { color: hsl(var(--primary)); }
  .icon.danger:hover { color: hsl(var(--destructive)); }
  .empty { display: flex; flex-direction: column; align-items: center; gap: 12px; padding: 60px 0; color: hsl(var(--muted-foreground)); text-align: center; }
  .err { color: hsl(var(--destructive)); font-size: 12.5px; margin-bottom: 10px; }

  .backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.5); display: grid; place-items: start center; padding-top: 12vh; z-index: 60; }
  .modal { width: 540px; max-width: 92vw; background: hsl(var(--card)); border: 1px solid hsl(var(--border)); border-radius: 12px; overflow: hidden; box-shadow: 0 24px 64px rgba(0, 0, 0, 0.55); }
  .mh { display: flex; align-items: center; gap: 9px; padding: 14px 18px; border-bottom: 1px solid hsl(var(--border)); font-size: 14px; font-weight: 600; }
  .mbody { padding: 16px 18px; display: flex; flex-direction: column; gap: 13px; }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field label { font-size: 11.5px; color: hsl(var(--muted-foreground)); }
  .help { font-size: 11.5px; color: hsl(var(--muted-foreground)); margin: 2px 0 0; word-break: break-all; }
  input, textarea { background: hsl(var(--muted)); border: 1px solid hsl(var(--border)); border-radius: 7px; padding: 8px 10px; color: inherit; outline: none; font-size: 13px; font-family: inherit; resize: vertical; }
  input:focus, textarea:focus { border-color: hsl(var(--ring) / 0.6); }
  .check { display: flex; align-items: center; gap: 8px; font-size: 13px; }
  .check input { width: auto; }
  .mfoot { padding: 13px 18px; border-top: 1px solid hsl(var(--border)); display: flex; justify-content: flex-end; gap: 8px; }
  .btn { display: inline-flex; align-items: center; gap: 6px; padding: 8px 14px; border: none; border-radius: 7px; background: hsl(var(--muted)); color: inherit; font-size: 13px; font-family: inherit; cursor: pointer; }
  .btn:hover { background: hsl(var(--border)); }
  .btn.ghost { background: transparent; }
  .btn.primary { background: hsl(var(--primary)); color: hsl(var(--primary-foreground)); font-weight: 600; }
  .btn:disabled { opacity: 0.5; cursor: default; }
</style>
