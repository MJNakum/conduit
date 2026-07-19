<script lang="ts">
  import { Pencil } from '@lucide/svelte'
  import { saveHost, keysStore, type Host } from './state.svelte'

  let { host, onclose }: { host: Host; onclose: () => void } = $props()

  // "" = use a raw identity-file path; otherwise a managed key id.
  let keyChoice = $state(host.keyId ?? '')

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
      <div class="grid2">
        <div class="field"><label for="f-host">Address</label><input id="f-host" class="mono" bind:value={draft.hostname} placeholder="example.com" /></div>
        <div class="field"><label for="f-user">Username</label><input id="f-user" class="mono" bind:value={draft.user} placeholder="root" /></div>
      </div>
      <div class="field"><label for="f-tags">Tags</label><input id="f-tags" bind:value={tagsText} placeholder="prod, web" /></div>
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
