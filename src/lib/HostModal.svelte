<script lang="ts">
  import { Pencil, Plus, X } from '@lucide/svelte'
  import { saveHost, keysStore, store, type Host } from './state.svelte'
  import { allSchemes } from './theme.svelte'

  let { host, onclose }: { host: Host; onclose: () => void } = $props()

  // "" = use a raw identity-file path; otherwise a managed key id.
  let keyChoice = $state(host.keyId ?? '')

  // Other saved hosts, offered as ProxyJump hops (a host can't jump through itself).
  const otherHosts = $derived(store.hosts.filter((h) => h.id !== draft.id))
  function addJump() {
    const first = otherHosts.find((h) => !draft.jumps.includes(h.id))
    if (first) draft.jumps = [...draft.jumps, first.id]
  }

  // Edit a copy; commit only on save. tags edited as a comma string.
  let draft = $state({ ...host })
  let tagsText = $state(host.tags.join(', '))
  // type=color needs a valid hex; keep a local default and a separate "use color" flag.
  let useColor = $state(host.color != null)
  let color = $state(host.color ?? '#10b981')

  async function save() {
    draft.tags = tagsText
      .split(',')
      .map((t) => t.trim())
      .filter(Boolean)
    draft.color = useColor ? color : null
    // Managed key vs raw path are mutually exclusive.
    draft.keyId = draft.auth === 'key' && keyChoice ? keyChoice : null
    if (draft.keyId) draft.identityFile = null
    await saveHost({ ...draft })
    onclose()
  }
</script>

<div class="backdrop" onclick={onclose} role="presentation">
  <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
    <div class="mh"><Pencil size={15} /> {host.name ? `Edit host — ${host.name}` : 'New host'}</div>
    <div class="mbody">
      <div class="grid3">
        <div class="field"><label for="f-name">Label</label><input id="f-name" bind:value={draft.name} placeholder="my server" /></div>
        <div class="field"><label for="f-port">Port</label><input id="f-port" class="mono" type="number" bind:value={draft.port} /></div>
        <div class="field">
          <label for="f-fav">Favorite</label>
          <label class="check"><input id="f-fav" type="checkbox" bind:checked={draft.favorite} /> Starred</label>
        </div>
      </div>
      <div class="grid3">
        <div class="field"><label for="f-host">Address</label><input id="f-host" class="mono" bind:value={draft.hostname} placeholder="example.com" /></div>
        <div class="field"><label for="f-user">Username</label><input id="f-user" class="mono" bind:value={draft.user} placeholder="root" /></div>
        <div class="field">
          <label for="f-proto">Protocol</label>
          <select id="f-proto" bind:value={draft.protocol}>
            <option value="ssh">SSH</option>
            <option value="telnet">Telnet</option>
          </select>
        </div>
      </div>
      <div class="grid2">
        <div class="field"><label for="f-tags">Tags</label><input id="f-tags" bind:value={tagsText} placeholder="prod, web" /></div>
        <div class="field"><label for="f-group">Group</label><input id="f-group" value={draft.group ?? ''} oninput={(e) => (draft.group = (e.currentTarget as HTMLInputElement).value || null)} placeholder="Clients/Acme" /></div>
      </div>
      <div class="grid2">
        <div class="field">
          <label for="f-color">Accent color</label>
          <label class="check"><input id="f-color" type="checkbox" bind:checked={useColor} /> <input class="swatch" type="color" bind:value={color} disabled={!useColor} /> Use color</label>
        </div>
        <div class="field">
          <label for="f-recon">Auto-reconnect</label>
          <label class="check"><input id="f-recon" type="checkbox" bind:checked={draft.autoReconnect} /> Reconnect on drop</label>
        </div>
      </div>
      <div class="field">
        <label for="f-log">Session logging</label>
        <label class="check"><input id="f-log" type="checkbox" bind:checked={draft.logging} /> Auto-save terminal output to a log file</label>
      </div>
      <div class="grid2">
        <div class="field">
          <label for="f-auth">Auth</label>
          <select id="f-auth" bind:value={draft.auth}>
            <option value="password">Password</option>
            <option value="key">Key</option>
          </select>
        </div>
        {#if draft.auth === 'key'}
          <div class="field">
            <label for="f-keysel">Key</label>
            <select id="f-keysel" bind:value={keyChoice}>
              {#each keysStore.keys as k (k.id)}
                <option value={k.id}>{k.name} ({k.key_type})</option>
              {/each}
              <option value="">Use file path…</option>
            </select>
          </div>
        {/if}
      </div>
      {#if draft.auth === 'key' && !keyChoice}
        <div class="field">
          <label for="f-key">Identity file</label>
          <input id="f-key" class="mono" value={draft.identityFile ?? ''} oninput={(e) => (draft.identityFile = (e.currentTarget as HTMLInputElement).value || null)} placeholder="~/.ssh/id_ed25519" />
        </div>
      {/if}
      <div class="field">
        <label>Jump hosts (ProxyJump)</label>
        {#if draft.jumps.length === 0}
          <p class="muted small">Direct connection. Add a bastion to tunnel through it.</p>
        {/if}
        {#each draft.jumps as jid, i (i)}
          <div class="jrow">
            <span class="muted mono hop">{i + 1}</span>
            <select value={jid} onchange={(e) => (draft.jumps[i] = (e.currentTarget as HTMLSelectElement).value)}>
              {#each otherHosts as h (h.id)}
                <option value={h.id}>{h.name} ({h.user}@{h.hostname})</option>
              {/each}
            </select>
            <button type="button" class="jx" aria-label="Remove jump" onclick={() => (draft.jumps = draft.jumps.filter((_, k) => k !== i))}><X size={14} /></button>
          </div>
        {/each}
        {#if otherHosts.length > draft.jumps.length}
          <button type="button" class="addjump" onclick={addJump}><Plus size={13} /> Add jump</button>
        {/if}
      </div>
      <div class="grid3">
        <div class="field">
          <label for="f-scheme">Terminal theme</label>
          <select id="f-scheme" value={draft.scheme ?? ''} onchange={(e) => (draft.scheme = (e.currentTarget as HTMLSelectElement).value || null)}>
            <option value="">Global default</option>
            {#each allSchemes() as s (s.name)}
              <option value={s.name}>{s.name}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="f-font">Font</label>
          <input id="f-font" class="mono" value={draft.font ?? ''} oninput={(e) => (draft.font = (e.currentTarget as HTMLInputElement).value || null)} placeholder="global" />
        </div>
        <div class="field">
          <label for="f-fsize">Font size</label>
          <input id="f-fsize" class="mono" type="number" min="8" max="32" value={draft.fontSize ?? ''} oninput={(e) => (draft.fontSize = Number((e.currentTarget as HTMLInputElement).value) || null)} placeholder="13" />
        </div>
      </div>
    </div>
    <div class="mfoot">
      <button class="btn ghost" onclick={onclose}>Cancel</button>
      <button class="btn primary" onclick={save} disabled={!draft.name || !draft.hostname}>Save</button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: grid;
    place-items: start center;
    padding-top: 12vh;
    z-index: 60;
  }
  .modal {
    width: 580px;
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
    gap: 13px;
    max-height: 52vh;
    overflow: auto;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .field label {
    font-size: 11.5px;
    color: hsl(var(--muted-foreground));
  }
  .field input,
  .field select {
    background: hsl(var(--muted));
    border: 1px solid hsl(var(--border));
    border-radius: 7px;
    padding: 8px 10px;
    color: inherit;
    outline: none;
    font-size: 13px;
    font-family: inherit;
  }
  .field input:focus,
  .field select:focus {
    border-color: hsl(var(--ring) / 0.6);
  }
  .grid2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 13px;
  }
  .grid3 {
    display: grid;
    grid-template-columns: 2fr 1fr 1fr;
    gap: 13px;
  }
  .check {
    flex-direction: row !important;
    align-items: center;
    gap: 8px;
    padding: 8px 0;
    color: hsl(var(--foreground)) !important;
    font-size: 13px !important;
  }
  .check input[type='checkbox'] {
    width: auto;
    background: none;
    padding: 0;
  }
  .small {
    font-size: 12px;
    margin: 2px 0 0;
  }
  .jrow {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 6px;
  }
  .jrow select {
    flex: 1;
  }
  .hop {
    width: 12px;
    text-align: right;
    font-size: 11px;
  }
  .jx {
    display: grid;
    place-items: center;
    width: 28px;
    height: 30px;
    flex: none;
    border: 1px solid hsl(var(--border));
    background: hsl(var(--muted));
    color: hsl(var(--muted-foreground));
    border-radius: 7px;
    cursor: pointer;
  }
  .jx:hover {
    color: hsl(var(--destructive));
  }
  .addjump {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
    padding: 6px 10px;
    border: 1px dashed hsl(var(--border));
    background: none;
    color: hsl(var(--muted-foreground));
    border-radius: 7px;
    font-size: 12.5px;
    font-family: inherit;
    cursor: pointer;
  }
  .addjump:hover {
    background: hsl(var(--muted));
    color: hsl(var(--foreground));
  }
  .swatch {
    padding: 2px;
    width: 40px;
    height: 28px;
    background: none;
    border: 1px solid hsl(var(--border));
    border-radius: 6px;
    cursor: pointer;
  }
  .swatch:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .mfoot {
    padding: 13px 18px;
    border-top: 1px solid hsl(var(--border));
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .btn {
    padding: 8px 14px;
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
  .btn.primary:hover {
    filter: brightness(1.08);
  }
  .btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
