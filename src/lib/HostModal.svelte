<script lang="ts">
  import { Pencil, Plus, X, Star } from '@lucide/svelte'
  import { saveHost, keysStore, store, hostIcon, type Host } from './state.svelte'
  import { allSchemes } from './theme.svelte'
  import { toast } from './toast.svelte'

  let { host, onclose }: { host: Host; onclose: () => void } = $props()

  // "" = use a raw identity-file path; otherwise a managed key id.
  let keyChoice = $state(host.keyId ?? '')

  // Edit a copy; commit only on save. tags edited as a comma string.
  let draft = $state({ ...host })
  let tagsText = $state(host.tags.join(', '))

  const Icon = $derived(hostIcon(draft))
  const isTelnet = $derived(draft.protocol === 'telnet')

  // Accent color: draft.color holds the truth (null = none). A handful of
  // presets plus a custom picker; the native <input type=color> is hidden
  // inside its swatch so it never renders as a squished bar.
  const PRESET = ['#10b981', '#3b82f6', '#a855f7', '#ef4444', '#f59e0b', '#14b8a6', '#ec4899', '#64748b']
  let customColor = $state(host.color && !PRESET.includes(host.color) ? host.color : '#10b981')
  const isCustom = $derived(!!draft.color && !PRESET.includes(draft.color))

  // Other saved hosts, offered as ProxyJump hops (a host can't jump through itself).
  const otherHosts = $derived(store.hosts.filter((h) => h.id !== draft.id))
  function addJump() {
    const first = otherHosts.find((h) => !draft.jumps.includes(h.id))
    if (first) draft.jumps = [...draft.jumps, first.id]
  }

  async function save() {
    draft.tags = tagsText
      .split(',')
      .map((t) => t.trim())
      .filter(Boolean)
    // Managed key vs raw path are mutually exclusive.
    draft.keyId = draft.auth === 'key' && keyChoice ? keyChoice : null
    if (draft.keyId) draft.identityFile = null
    await saveHost({ ...draft })
    toast(`Saved "${draft.name || 'host'}"`)
    onclose()
  }
</script>

<div class="backdrop" onclick={onclose} role="presentation">
  <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
    <div class="mh">
      <Pencil size={15} />
      {host.name ? `Edit host — ${host.name}` : 'New host'}
    </div>

    <div class="mbody">
      <!-- Live identity preview: exactly how this host reads in the list -->
      <div class="preview">
        <span class="pv-ico" style:color={draft.color ?? undefined}><Icon size={17} /></span>
        <span class="pv-name">{draft.name || 'Host name'}</span>
        <span class="pv-addr mono">{draft.user || 'user'}@{draft.hostname || 'address'}:{draft.port}</span>
        {#if draft.favorite}<Star size={13} color="hsl(var(--amber))" />{/if}
        {#if draft.color}<span class="pv-dot" style:background={draft.color}></span>{/if}
      </div>

      <div class="sec">Connection</div>
      <div class="grid">
        <div class="field col2"><label for="f-name">Label</label><input id="f-name" bind:value={draft.name} placeholder="my server" /></div>
        <div class="field">
          <label for="f-proto">Protocol</label>
          <select id="f-proto" bind:value={draft.protocol}>
            <option value="ssh">SSH</option>
            <option value="telnet">Telnet</option>
          </select>
        </div>
      </div>
      <div class="grid">
        <div class="field col2"><label for="f-host">Address</label><input id="f-host" class="mono" bind:value={draft.hostname} placeholder="example.com" /></div>
        <div class="field"><label for="f-port">Port</label><input id="f-port" class="mono" type="number" bind:value={draft.port} /></div>
      </div>
      {#if !isTelnet}
        <div class="field"><label for="f-user">Username</label><input id="f-user" class="mono" bind:value={draft.user} placeholder="root" /></div>
      {/if}

      <div class="sec">Organize</div>
      <div class="grid">
        <div class="field"><label for="f-tags">Tags</label><input id="f-tags" bind:value={tagsText} placeholder="prod, web" /></div>
        <div class="field"><label for="f-group">Group</label><input id="f-group" value={draft.group ?? ''} oninput={(e) => (draft.group = (e.currentTarget as HTMLInputElement).value || null)} placeholder="Clients/Acme" /></div>
      </div>
      <div class="field">
        <label>Accent color</label>
        <div class="swatches">
          <button type="button" class="dot none" class:sel={!draft.color} title="None" aria-label="No color" onclick={() => (draft.color = null)}>
            <X size={12} />
          </button>
          {#each PRESET as c (c)}
            <button type="button" class="dot" class:sel={draft.color === c} style:background={c} aria-label={c} onclick={() => (draft.color = c)}></button>
          {/each}
          <label class="dot custom" class:sel={isCustom} style:background={isCustom ? draft.color : 'transparent'} title="Custom">
            <input type="color" bind:value={customColor} onchange={() => (draft.color = customColor)} />
            {#if !isCustom}<Plus size={12} />{/if}
          </label>
        </div>
      </div>
      <div class="row2">
        <label class="check"><input type="checkbox" bind:checked={draft.favorite} /> <Star size={13} /> Favorite</label>
        <label class="check"><input type="checkbox" bind:checked={draft.autoReconnect} /> Reconnect on drop</label>
      </div>

      {#if !isTelnet}
        <div class="sec">Authentication</div>
        <div class="grid">
          <div class="field">
            <label for="f-auth">Method</label>
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
      {/if}

      <div class="sec">Terminal</div>
      <div class="grid">
        <div class="field col2">
          <label for="f-scheme">Theme</label>
          <select id="f-scheme" value={draft.scheme ?? ''} onchange={(e) => (draft.scheme = (e.currentTarget as HTMLSelectElement).value || null)}>
            <option value="">Global default</option>
            {#each allSchemes() as s (s.name)}
              <option value={s.name}>{s.name}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="f-fsize">Font size</label>
          <input id="f-fsize" class="mono" type="number" min="8" max="32" value={draft.fontSize ?? ''} oninput={(e) => (draft.fontSize = Number((e.currentTarget as HTMLInputElement).value) || null)} placeholder="13" />
        </div>
      </div>
      <div class="field">
        <label for="f-font">Font</label>
        <input id="f-font" class="mono" value={draft.font ?? ''} oninput={(e) => (draft.font = (e.currentTarget as HTMLInputElement).value || null)} placeholder="global default" />
      </div>

      <div class="sec">Behavior</div>
      <label class="check"><input type="checkbox" bind:checked={draft.logging} /> Auto-save terminal output to a log file</label>
    </div>

    <div class="mfoot">
      <button class="btn ghost" onclick={onclose}>Cancel</button>
      <button class="btn primary" onclick={save} disabled={!draft.name || !draft.hostname}>Save host</button>
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
    padding-top: 9vh;
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
    padding: 14px 18px 18px;
    display: flex;
    flex-direction: column;
    gap: 11px;
    max-height: 64vh;
    overflow: auto;
  }

  /* live preview */
  .preview {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 11px 13px;
    border: 1px solid hsl(var(--border));
    border-radius: 9px;
    background: hsl(var(--muted) / 0.5);
  }
  .pv-ico {
    display: grid;
    place-items: center;
    color: hsl(var(--muted-foreground));
  }
  .pv-name {
    font-weight: 500;
    font-size: 13px;
  }
  .pv-addr {
    font-size: 12px;
    color: hsl(var(--muted-foreground));
  }
  .pv-dot {
    margin-left: auto;
    width: 9px;
    height: 9px;
    border-radius: 50%;
  }

  /* section header */
  .sec {
    margin-top: 6px;
    font-size: 10.5px;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: hsl(var(--muted-foreground));
    border-bottom: 1px solid hsl(var(--border) / 0.6);
    padding-bottom: 5px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .field > label {
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
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .grid .col2 {
    grid-column: span 1;
  }
  .row2 {
    display: flex;
    gap: 22px;
    flex-wrap: wrap;
  }
  .check {
    display: inline-flex;
    flex-direction: row;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: hsl(var(--foreground));
    cursor: pointer;
  }
  .check input[type='checkbox'] {
    width: auto;
    cursor: pointer;
  }

  /* color swatches */
  .swatches {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .dot {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    border: 2px solid transparent;
    box-shadow: 0 0 0 1px hsl(var(--border));
    cursor: pointer;
    padding: 0;
    display: grid;
    place-items: center;
    color: #fff;
    position: relative;
  }
  .dot.sel {
    border-color: hsl(var(--foreground));
  }
  .dot.none {
    background: hsl(var(--muted));
    color: hsl(var(--muted-foreground));
  }
  .dot.custom {
    background:
      conic-gradient(from 0deg, #ef4444, #f59e0b, #10b981, #3b82f6, #a855f7, #ef4444);
    color: hsl(var(--foreground));
    overflow: hidden;
  }
  .dot.custom input[type='color'] {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
    padding: 0;
    border: none;
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
    background: hsl(var(--muted));
    border: 1px solid hsl(var(--border));
    border-radius: 7px;
    padding: 8px 10px;
    color: inherit;
    font-size: 13px;
    font-family: inherit;
  }
  .hop {
    width: 12px;
    text-align: right;
    font-size: 11px;
  }
  .jx {
    display: grid;
    place-items: center;
    width: 30px;
    height: 34px;
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
  .mfoot {
    padding: 13px 18px;
    border-top: 1px solid hsl(var(--border));
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .btn {
    padding: 8px 15px;
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
