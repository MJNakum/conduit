<script lang="ts">
  import { KeyRound, Plus, Upload, Copy, Trash2, ShieldCheck } from '@lucide/svelte'
  import { keysStore, generateKey, importKey, deleteKey, store, type Key } from './state.svelte'
  import { toast } from './toast.svelte'

  let mode = $state<'none' | 'generate' | 'import'>('none')
  let busy = $state(false)
  let err = $state('')

  // generate form
  let gName = $state('')
  let gType = $state<'ed25519' | 'rsa' | 'ecdsa'>('ed25519')
  // import form
  let iName = $state('')
  let iPem = $state('')
  let iPass = $state('')
  // last-created key, shown with its public key + install command
  let created = $state<Key | null>(null)

  const fmtDate = (secs: string) => {
    const n = Number(secs)
    return n ? new Date(n * 1000).toLocaleDateString() : ''
  }

  // Hosts referencing a managed key — for the delete warning.
  const usedBy = (id: string) => store.hosts.filter((h) => h.keyId === id).length

  function copy(text: string) {
    navigator.clipboard?.writeText(text)
    toast('Copied to clipboard')
  }
  const installCmd = (pub: string) =>
    `mkdir -p ~/.ssh && echo '${pub.trim()}' >> ~/.ssh/authorized_keys`

  async function doGenerate() {
    if (!gName.trim()) return
    busy = true
    err = ''
    try {
      created = await generateKey(gName.trim(), gType)
      toast(`Key "${created.name}" generated`)
      gName = ''
      mode = 'none'
    } catch (e) {
      err = String(e)
    } finally {
      busy = false
    }
  }

  async function doImport() {
    if (!iName.trim() || !iPem.trim()) return
    busy = true
    err = ''
    try {
      created = await importKey(iName.trim(), iPem, iPass)
      toast(`Key "${created.name}" imported`)
      iName = ''
      iPem = ''
      iPass = ''
      mode = 'none'
    } catch (e) {
      err = String(e)
    } finally {
      busy = false
    }
  }

  async function remove(k: Key) {
    const n = usedBy(k.id)
    const msg = n > 0 ? `Delete "${k.name}"? It is used by ${n} host${n === 1 ? '' : 's'}.` : `Delete "${k.name}"?`
    if (confirm(msg)) {
      await deleteKey(k.id)
      toast(`Deleted key "${k.name}"`)
    }
  }
</script>

<div class="wrap">
  <header>
    <h1><KeyRound size={18} /> Keys</h1>
    <div class="actions">
      <button class="btn" onclick={() => { mode = 'import'; err = '' }}><Upload size={14} /> Import</button>
      <button class="btn primary" onclick={() => { mode = 'generate'; err = '' }}><Plus size={14} /> Generate</button>
    </div>
  </header>

  {#if created}
    <div class="reveal">
      <div class="rhead"><ShieldCheck size={15} /> {created.name} created — stored in Keychain</div>
      <code class="pub mono">{created.public_key}</code>
      <div class="rrow">
        <button class="btn" onclick={() => copy(created!.public_key)}><Copy size={13} /> Copy public key</button>
        <button class="btn" onclick={() => copy(installCmd(created!.public_key))}><Copy size={13} /> Copy install command</button>
        <button class="btn ghost" onclick={() => (created = null)}>Dismiss</button>
      </div>
    </div>
  {/if}

  {#if err}<div class="err">{err}</div>{/if}

  {#if keysStore.keys.length === 0}
    <div class="empty">
      <KeyRound size={30} />
      <p>No keys yet. Generate a new key or import an existing one.</p>
    </div>
  {:else}
    <ul class="list">
      {#each keysStore.keys as k (k.id)}
        <li>
          <span class="badge mono">{k.key_type}</span>
          <span class="name">{k.name}</span>
          <span class="fp mono" title={k.fingerprint}>{k.fingerprint}</span>
          <span class="badge kc"><ShieldCheck size={12} /> Keychain</span>
          <span class="muted date">{fmtDate(k.created)}</span>
          <span class="used muted">{usedBy(k.id) || ''}{usedBy(k.id) ? ' host' + (usedBy(k.id) === 1 ? '' : 's') : ''}</span>
          <span class="row-actions">
            <button class="icon" title="Copy public key" onclick={() => copy(k.public_key)}><Copy size={14} /></button>
            <button class="icon danger" title="Delete" onclick={() => remove(k)}><Trash2 size={14} /></button>
          </span>
        </li>
      {/each}
    </ul>
  {/if}
</div>

{#if mode !== 'none'}
  <div class="backdrop" onclick={() => (mode = 'none')} role="presentation">
    <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
      {#if mode === 'generate'}
        <div class="mh"><Plus size={15} /> Generate key</div>
        <div class="mbody">
          <div class="field"><label for="g-name">Name</label><input id="g-name" bind:value={gName} placeholder="work-laptop" /></div>
          <div class="field">
            <label for="g-type">Type</label>
            <select id="g-type" bind:value={gType}>
              <option value="ed25519">ed25519 (recommended)</option>
              <option value="rsa">RSA 4096</option>
              <option value="ecdsa">ECDSA P-256</option>
            </select>
          </div>
        </div>
        <div class="mfoot">
          <button class="btn ghost" onclick={() => (mode = 'none')}>Cancel</button>
          <button class="btn primary" onclick={doGenerate} disabled={busy || !gName.trim()}>Generate</button>
        </div>
      {:else}
        <div class="mh"><Upload size={15} /> Import key</div>
        <div class="mbody">
          <div class="field"><label for="i-name">Name</label><input id="i-name" bind:value={iName} placeholder="imported-key" /></div>
          <div class="field">
            <label for="i-pem">Private key (paste OpenSSH PEM)</label>
            <textarea id="i-pem" class="mono" bind:value={iPem} rows="7" placeholder="-----BEGIN OPENSSH PRIVATE KEY-----"></textarea>
          </div>
          <div class="field"><label for="i-pass">Passphrase (if encrypted)</label><input id="i-pass" type="password" bind:value={iPass} /></div>
        </div>
        <div class="mfoot">
          <button class="btn ghost" onclick={() => (mode = 'none')}>Cancel</button>
          <button class="btn primary" onclick={doImport} disabled={busy || !iName.trim() || !iPem.trim()}>Import</button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .wrap {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 18px 22px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }
  h1 {
    display: flex;
    align-items: center;
    gap: 9px;
    font-size: 17px;
    font-weight: 600;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  .list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .list li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 10px;
    border: 1px solid hsl(var(--border));
    border-radius: 8px;
    background: hsl(var(--card));
    font-size: 12.5px;
  }
  .badge {
    padding: 2px 7px;
    border-radius: 5px;
    background: hsl(var(--muted));
    border: 1px solid hsl(var(--border));
    font-size: 11px;
    flex: none;
  }
  .badge.kc {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: hsl(var(--primary));
  }
  .name {
    font-weight: 600;
    flex: none;
    min-width: 110px;
  }
  .fp {
    color: hsl(var(--muted-foreground));
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .date,
  .used {
    flex: none;
    font-size: 11.5px;
  }
  .row-actions {
    display: flex;
    gap: 4px;
    flex: none;
  }
  .icon {
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
  .icon:hover {
    background: hsl(var(--muted));
    color: hsl(var(--foreground));
  }
  .icon.danger:hover {
    color: hsl(var(--destructive));
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 60px 0;
    color: hsl(var(--muted-foreground));
    text-align: center;
  }
  .reveal {
    border: 1px solid hsl(var(--primary) / 0.4);
    background: hsl(var(--primary) / 0.06);
    border-radius: 10px;
    padding: 12px 14px;
    margin-bottom: 16px;
  }
  .rhead {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    font-weight: 600;
    margin-bottom: 9px;
    color: hsl(var(--primary));
  }
  .pub {
    display: block;
    padding: 8px 10px;
    background: hsl(var(--muted));
    border: 1px solid hsl(var(--border));
    border-radius: 6px;
    font-size: 11.5px;
    word-break: break-all;
    user-select: all;
  }
  .rrow {
    display: flex;
    gap: 8px;
    margin-top: 9px;
  }
  .err {
    color: hsl(var(--destructive));
    font-size: 12.5px;
    margin-bottom: 12px;
  }

  /* modal — mirrors HostModal */
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
    width: 520px;
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
  .field select,
  .field textarea {
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
  .field input:focus,
  .field select:focus,
  .field textarea:focus {
    border-color: hsl(var(--ring) / 0.6);
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
  .btn.primary:hover {
    filter: brightness(1.08);
  }
  .btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
