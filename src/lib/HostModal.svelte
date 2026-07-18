<script lang="ts">
  import { saveHost, type Host } from './state.svelte'

  let { host, onclose }: { host: Host; onclose: () => void } = $props()

  // Edit a copy; commit only on save. tags edited as a comma string.
  let draft = $state({ ...host })
  let tagsText = $state(host.tags.join(', '))
  // type=color needs a valid hex; keep a local default and a separate "use color" flag.
  let useColor = $state(host.color != null)
  let color = $state(host.color ?? '#3b82f6')

  async function save() {
    draft.tags = tagsText
      .split(',')
      .map((t) => t.trim())
      .filter(Boolean)
    draft.color = useColor ? color : null
    await saveHost({ ...draft })
    onclose()
  }
</script>

<div class="backdrop" onclick={onclose} role="presentation">
  <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
    <h2>{host.name ? 'Edit host' : 'New host'}</h2>
    <label>Name<input bind:value={draft.name} placeholder="my server" /></label>
    <label>Host<input bind:value={draft.hostname} placeholder="example.com" /></label>
    <div class="row">
      <label>User<input bind:value={draft.user} placeholder="root" /></label>
      <label class="port">Port<input type="number" bind:value={draft.port} /></label>
    </div>
    <label>Tags<input bind:value={tagsText} placeholder="prod, web" /></label>
    <div class="row">
      <label class="fav"><input type="checkbox" bind:checked={useColor} /> Color</label>
      <input class="swatch" type="color" bind:value={color} disabled={!useColor} />
      <label class="fav"><input type="checkbox" bind:checked={draft.favorite} /> Favorite</label>
    </div>
    <label class="fav">
      <input type="checkbox" bind:checked={draft.autoReconnect} /> Auto-reconnect on drop
    </label>
    <div class="actions">
      <button onclick={onclose}>Cancel</button>
      <button class="primary" onclick={save} disabled={!draft.name || !draft.hostname}>Save</button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .modal {
    background: #1c1c1c;
    padding: 1.2rem;
    border-radius: 8px;
    width: 320px;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  h2 {
    margin: 0 0 0.4rem;
    font-size: 1rem;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    font-size: 0.75rem;
    color: #aaa;
  }
  input {
    padding: 0.4rem;
    background: #111;
    border: 1px solid #333;
    color: #eee;
    border-radius: 4px;
  }
  /* Native color input: no dark fill masking the swatch. */
  .swatch {
    padding: 2px;
    width: 46px;
    height: 32px;
    background: none;
    cursor: pointer;
  }
  .swatch:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .row {
    display: flex;
    gap: 0.6rem;
  }
  .row label {
    flex: 1;
  }
  .port {
    max-width: 90px;
  }
  .fav {
    flex-direction: row;
    align-items: center;
    justify-content: flex-start;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 0.4rem;
  }
  button {
    padding: 0.4rem 0.9rem;
    border-radius: 4px;
    border: 1px solid #333;
    background: #222;
    color: #eee;
    cursor: pointer;
  }
  .primary {
    background: #2b6cff;
    border-color: #2b6cff;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
