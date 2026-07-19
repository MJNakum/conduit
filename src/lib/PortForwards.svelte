<script lang="ts">
  import { ArrowRightLeft, Plus, Trash2, Pencil } from '@lucide/svelte'
  import {
    store,
    forwardsStore,
    blankForward,
    saveForward,
    deleteForward,
    startForward,
    stopForward,
    type Forward,
  } from './state.svelte'

  let editing = $state<Forward | null>(null)

  const hostName = (id: string) => store.hosts.find((h) => h.id === id)?.name ?? '(host?)'

  function route(f: Forward): string {
    if (f.kind === 'local') return `localhost:${f.bindPort} → ${f.destHost}:${f.destPort}`
    if (f.kind === 'remote') return `:${f.bindPort} ← ${f.destHost}:${f.destPort}`
    return `SOCKS :${f.bindPort}`
  }

  const statusOf = (id: string) => forwardsStore.status[id]?.state ?? 'stopped'
  function dot(state: string): string {
    if (state === 'active') return 'hsl(var(--primary))'
    if (state === 'starting') return 'hsl(var(--connecting))'
    if (state === 'error') return 'hsl(var(--destructive))'
    return 'hsl(var(--muted-foreground))'
  }

  function toggle(f: Forward) {
    const on = statusOf(f.id) === 'active' || statusOf(f.id) === 'starting'
    if (on) stopForward(f.id)
    else {
      forwardsStore.status[f.id] = { state: 'starting' }
      startForward(f.id)
    }
  }

  async function save() {
    if (!editing) return
    await saveForward({ ...editing })
    editing = null
  }

  const kindHelp: Record<string, string> = {
    local: 'Local: reach a remote service as if it were on your machine.',
    remote: 'Remote: expose a service on your machine to the remote host.',
    dynamic: 'Dynamic: a local SOCKS proxy that tunnels all its traffic through the host.',
  }
</script>

<div class="wrap">
  <header>
    <h1><ArrowRightLeft size={18} /> Port Forwards</h1>
    <button class="btn primary" onclick={() => (editing = blankForward())}><Plus size={14} /> New forward</button>
  </header>

  {#if forwardsStore.list.length === 0}
    <div class="empty"><ArrowRightLeft size={30} /><p>No forwards yet. Create a local, remote, or dynamic (SOCKS) tunnel.</p></div>
  {:else}
    <ul class="list">
      {#each forwardsStore.list as f (f.id)}
        {@const st = statusOf(f.id)}
        <li>
          <span class="badge">{f.kind}</span>
          <span class="name">{f.name || '(unnamed)'}</span>
          <span class="route mono">{route(f)}</span>
          <span class="muted host">{hostName(f.hostId)}</span>
          {#if forwardsStore.status[f.id]?.message && st === 'error'}
            <span class="errmsg" title={forwardsStore.status[f.id].message}>error</span>
          {/if}
          <span class="dot" style:background={dot(st)}></span>
          <label class="switch" title={st === 'active' ? 'Active' : st}>
            <input type="checkbox" checked={st === 'active' || st === 'starting'} onchange={() => toggle(f)} />
            <span class="track"></span>
          </label>
          <button class="icon" title="Edit" onclick={() => (editing = { ...f })}><Pencil size={14} /></button>
          <button class="icon danger" title="Delete" onclick={() => deleteForward(f.id)}><Trash2 size={14} /></button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

{#if editing}
  <div class="backdrop" onclick={() => (editing = null)} role="presentation">
    <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
      <div class="mh"><ArrowRightLeft size={15} /> {editing.name ? 'Edit forward' : 'New forward'}</div>
      <div class="mbody">
        <div class="grid2">
          <div class="field"><label for="ff-name">Name</label><input id="ff-name" bind:value={editing.name} placeholder="db tunnel" /></div>
          <div class="field">
            <label for="ff-host">Host</label>
            <select id="ff-host" bind:value={editing.hostId}>
              <option value="" disabled>Select a host…</option>
              {#each store.hosts as h (h.id)}<option value={h.id}>{h.name}</option>{/each}
            </select>
          </div>
        </div>
        <div class="field">
          <label for="ff-kind">Type</label>
          <select id="ff-kind" bind:value={editing.kind}>
            <option value="local">Local (-L)</option>
            <option value="remote">Remote (-R)</option>
            <option value="dynamic">Dynamic / SOCKS (-D)</option>
          </select>
          <p class="help">{kindHelp[editing.kind]}</p>
        </div>
        <div class="grid2">
          <div class="field"><label for="ff-ba">Bind address</label><input id="ff-ba" class="mono" bind:value={editing.bindAddr} /></div>
          <div class="field"><label for="ff-bp">{editing.kind === 'remote' ? 'Remote bind port' : 'Local port'}</label><input id="ff-bp" class="mono" type="number" bind:value={editing.bindPort} /></div>
        </div>
        {#if editing.kind !== 'dynamic'}
          <div class="grid2">
            <div class="field"><label for="ff-dh">{editing.kind === 'remote' ? 'Local dest host' : 'Remote dest host'}</label><input id="ff-dh" class="mono" bind:value={editing.destHost} placeholder="127.0.0.1" /></div>
            <div class="field"><label for="ff-dp">Dest port</label><input id="ff-dp" class="mono" type="number" bind:value={editing.destPort} /></div>
          </div>
        {/if}
      </div>
      <div class="mfoot">
        <button class="btn ghost" onclick={() => (editing = null)}>Cancel</button>
        <button class="btn primary" onclick={save} disabled={!editing.name || !editing.hostId}>Save</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .wrap { flex: 1; min-height: 0; overflow: auto; padding: 18px 22px; }
  header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 16px; }
  h1 { display: flex; align-items: center; gap: 9px; font-size: 17px; font-weight: 600; }
  .list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 4px; }
  .list li {
    display: flex; align-items: center; gap: 12px; padding: 8px 12px;
    border: 1px solid hsl(var(--border)); border-radius: 8px; background: hsl(var(--card)); font-size: 12.5px;
  }
  .badge { padding: 2px 7px; border-radius: 5px; background: hsl(var(--muted)); border: 1px solid hsl(var(--border)); font-size: 11px; text-transform: capitalize; flex: none; }
  .name { font-weight: 600; flex: none; min-width: 90px; }
  .route { color: hsl(var(--muted-foreground)); flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .host { flex: none; }
  .errmsg { color: hsl(var(--destructive)); font-size: 11px; }
  .dot { width: 8px; height: 8px; border-radius: 50%; flex: none; }
  .switch { position: relative; width: 34px; height: 20px; flex: none; }
  .switch input { position: absolute; opacity: 0; width: 100%; height: 100%; margin: 0; cursor: pointer; }
  .track { position: absolute; inset: 0; background: hsl(var(--muted)); border: 1px solid hsl(var(--border)); border-radius: 999px; transition: background 0.15s; }
  .track::after { content: ''; position: absolute; top: 2px; left: 2px; width: 14px; height: 14px; border-radius: 50%; background: hsl(var(--muted-foreground)); transition: transform 0.15s; }
  .switch input:checked + .track { background: hsl(var(--primary)); }
  .switch input:checked + .track::after { transform: translateX(14px); background: #fff; }
  .icon { display: grid; place-items: center; width: 26px; height: 26px; border: none; background: none; border-radius: 6px; color: hsl(var(--muted-foreground)); cursor: pointer; flex: none; }
  .icon:hover { background: hsl(var(--muted)); color: hsl(var(--foreground)); }
  .icon.danger:hover { color: hsl(var(--destructive)); }
  .empty { display: flex; flex-direction: column; align-items: center; gap: 12px; padding: 60px 0; color: hsl(var(--muted-foreground)); text-align: center; }

  .backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.5); display: grid; place-items: start center; padding-top: 12vh; z-index: 60; }
  .modal { width: 520px; max-width: 92vw; background: hsl(var(--card)); border: 1px solid hsl(var(--border)); border-radius: 12px; overflow: hidden; box-shadow: 0 24px 64px rgba(0, 0, 0, 0.55); }
  .mh { display: flex; align-items: center; gap: 9px; padding: 14px 18px; border-bottom: 1px solid hsl(var(--border)); font-size: 14px; font-weight: 600; }
  .mbody { padding: 16px 18px; display: flex; flex-direction: column; gap: 13px; }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field label { font-size: 11.5px; color: hsl(var(--muted-foreground)); }
  .help { font-size: 11.5px; color: hsl(var(--muted-foreground)); margin: 2px 0 0; }
  input, select { background: hsl(var(--muted)); border: 1px solid hsl(var(--border)); border-radius: 7px; padding: 8px 10px; color: inherit; outline: none; font-size: 13px; font-family: inherit; }
  input:focus, select:focus { border-color: hsl(var(--ring) / 0.6); }
  .grid2 { display: grid; grid-template-columns: 1fr 1fr; gap: 13px; }
  .mfoot { padding: 13px 18px; border-top: 1px solid hsl(var(--border)); display: flex; justify-content: flex-end; gap: 8px; }
  .btn { display: inline-flex; align-items: center; gap: 6px; padding: 8px 14px; border: none; border-radius: 7px; background: hsl(var(--muted)); color: inherit; font-size: 13px; font-family: inherit; cursor: pointer; }
  .btn:hover { background: hsl(var(--border)); }
  .btn.ghost { background: transparent; }
  .btn.primary { background: hsl(var(--primary)); color: hsl(var(--primary-foreground)); font-weight: 600; }
  .btn:disabled { opacity: 0.5; cursor: default; }
</style>
