<script lang="ts">
  import { Copy, Trash2, Check, Save } from '@lucide/svelte'
  import {
    settings,
    customSchemes,
    allSchemes,
    schemeByName,
    BUILTINS,
    setAppTheme,
    saveDefaults,
    saveCustomSchemes,
    type Scheme,
    type AppTheme,
  } from './theme.svelte'
  import { toast } from './toast.svelte'
  import Select from './ui/Select.svelte'

  const clone = (s: Scheme): Scheme => JSON.parse(JSON.stringify(s))
  const isBuiltin = (name: string) => BUILTINS.some((b) => b.name === name)
  const ANSI = ['black','red','green','yellow','blue','magenta','cyan','white',
    'br.black','br.red','br.green','br.yellow','br.blue','br.magenta','br.cyan','br.white']

  let editName = $state(settings.defaultScheme)
  let draft = $state<Scheme>(clone(schemeByName(settings.defaultScheme)))
  let err = $state('')
  let copied = $state(false)
  let importText = $state('')

  function loadEdit(name: string) {
    editName = name
    draft = clone(schemeByName(name))
    err = ''
  }

  function save() {
    err = ''
    if (!draft.name.trim()) return (err = 'Name required')
    if (isBuiltin(draft.name)) return (err = 'Rename to save a custom copy (built-ins are read-only)')
    const i = customSchemes.list.findIndex((s) => s.name === draft.name)
    const clean = { ...clone(draft), builtin: false }
    if (i >= 0) customSchemes.list[i] = clean
    else customSchemes.list.push(clean)
    saveCustomSchemes()
    editName = draft.name
    toast(`Saved scheme "${draft.name}"`)
  }

  function del() {
    customSchemes.list = customSchemes.list.filter((s) => s.name !== draft.name)
    saveCustomSchemes()
    if (settings.defaultScheme === draft.name) {
      settings.defaultScheme = 'Default Dark'
      saveDefaults()
    }
    loadEdit('Default Dark')
    toast('Scheme deleted')
  }

  function exportJson() {
    navigator.clipboard?.writeText(JSON.stringify({ ...draft, builtin: undefined }, null, 2))
    copied = true
    toast('Scheme JSON copied')
    setTimeout(() => (copied = false), 1500)
  }

  function importJson() {
    err = ''
    try {
      const s = JSON.parse(importText)
      if (!s.name || !s.background || !s.foreground || !s.cursor || !Array.isArray(s.ansi) || s.ansi.length !== 16)
        throw new Error('missing name/background/foreground/cursor or ansi[16]')
      draft = { name: s.name, background: s.background, foreground: s.foreground, cursor: s.cursor, ansi: s.ansi }
      importText = ''
    } catch (e) {
      err = 'Invalid scheme JSON: ' + String(e)
    }
  }

  const isCustom = $derived(customSchemes.list.some((s) => s.name === draft.name))
</script>

<div class="ap">
  <section>
    <h3>Theme</h3>
    <div class="field">
      <label>App theme</label>
      <div class="seg">
        {#each ['dark', 'light', 'system'] as t}
          <button class:active={settings.appTheme === t} onclick={() => setAppTheme(t as AppTheme)}>{t}</button>
        {/each}
      </div>
    </div>
    <div class="grid3">
      <div class="field">
        <label for="a-def">Default terminal scheme</label>
        <Select id="a-def" bind:value={settings.defaultScheme} onchange={saveDefaults}
          options={allSchemes().map((s) => ({ value: s.name, label: s.name }))} />
      </div>
      <div class="field">
        <label for="a-font">Default font</label>
        <input id="a-font" class="mono" bind:value={settings.defaultFont} onchange={saveDefaults} />
      </div>
      <div class="field">
        <label for="a-fs">Default size</label>
        <input id="a-fs" class="mono" type="number" min="8" max="32" bind:value={settings.defaultFontSize} onchange={saveDefaults} />
      </div>
    </div>
  </section>

  <section>
    <div class="sechead">
      <h3>Color schemes</h3>
      <span class="muted small">Edit a built-in (saves a custom copy) or craft your own.</span>
    </div>

    <div class="editrow">
      <div class="field grow">
        <label for="a-edit">Editing</label>
        <Select id="a-edit" value={editName} onchange={loadEdit}
          options={allSchemes().map((s) => ({ value: s.name, label: s.name + (s.builtin ? ' (built-in)' : '') }))} />
      </div>
      <div class="acts">
        {#if isCustom}<button class="btn danger" onclick={del}><Trash2 size={13} /> Delete</button>{/if}
        <button class="btn primary" onclick={save}><Save size={13} /> Save scheme</button>
      </div>
    </div>

    <div class="preview" style:background={draft.background} style:color={draft.foreground}>
      <div><span style:color={draft.ansi[2]}>user@host</span>:<span style:color={draft.ansi[4]}>~/dev</span>$ echo hello</div>
      <div class="muted-fg">{draft.foreground} on {draft.background}</div>
      <div class="swatches">
        {#each draft.ansi as c}<span class="sw" style:background={c}></span>{/each}
      </div>
    </div>

    <div class="grid3">
      <div class="field"><label for="s-name">Name</label><input id="s-name" bind:value={draft.name} /></div>
      <div class="field"><label for="s-bg">Background</label><input id="s-bg" type="color" bind:value={draft.background} /></div>
      <div class="field"><label for="s-fg">Foreground</label><input id="s-fg" type="color" bind:value={draft.foreground} /></div>
    </div>
    <div class="field narrow"><label for="s-cur">Cursor</label><input id="s-cur" type="color" bind:value={draft.cursor} /></div>

    <div class="field">
      <label>ANSI palette</label>
      <div class="ansigrid">
        {#each draft.ansi as _, i}
          <label class="ansi" title={ANSI[i]}>
            <input type="color" bind:value={draft.ansi[i]} />
            <span>{ANSI[i]}</span>
          </label>
        {/each}
      </div>
    </div>

    <div class="field">
      <label for="s-imp">Import / export</label>
      <textarea id="s-imp" class="mono" rows="3" bind:value={importText} placeholder={'Paste scheme JSON here…  { "name": "...", "background": "#...", "ansi": [16] }'}></textarea>
      <div class="row">
        <button class="btn" onclick={importJson} disabled={!importText.trim()}>Load JSON into editor</button>
        <button class="btn" onclick={exportJson}>{#if copied}<Check size={13} /> Copied{:else}<Copy size={13} /> Export current{/if}</button>
      </div>
    </div>

    {#if err}<div class="err">{err}</div>{/if}
  </section>
</div>

<style>
  .ap {
    display: flex;
    flex-direction: column;
    gap: 26px;
    max-width: 620px;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 13px;
  }
  h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }
  .sechead {
    display: flex;
    align-items: baseline;
    gap: 10px;
    flex-wrap: wrap;
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
  textarea {
    resize: vertical;
  }
  input[type='color'] {
    padding: 2px;
    height: 34px;
    cursor: pointer;
  }
  .grid3 {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 12px;
  }
  .narrow {
    max-width: 200px;
  }
  .editrow {
    display: flex;
    align-items: flex-end;
    gap: 12px;
  }
  .grow {
    flex: 1;
  }
  .acts {
    display: flex;
    gap: 8px;
  }
  .seg {
    display: inline-flex;
    border: 1px solid hsl(var(--border));
    border-radius: 8px;
    overflow: hidden;
    width: fit-content;
  }
  .seg button {
    padding: 7px 16px;
    border: none;
    background: hsl(var(--muted));
    color: hsl(var(--muted-foreground));
    cursor: pointer;
    font-family: inherit;
    font-size: 12.5px;
    text-transform: capitalize;
  }
  .seg button.active {
    background: hsl(var(--primary));
    color: hsl(var(--primary-foreground));
    font-weight: 600;
  }
  .preview {
    border-radius: 8px;
    padding: 12px 14px;
    font-family: ui-monospace, monospace;
    font-size: 12.5px;
    line-height: 1.6;
    border: 1px solid hsl(var(--border));
  }
  .muted-fg {
    opacity: 0.7;
  }
  .swatches {
    display: flex;
    gap: 3px;
    margin-top: 8px;
  }
  .sw {
    width: 16px;
    height: 16px;
    border-radius: 3px;
  }
  .ansigrid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
  }
  .ansi {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: hsl(var(--muted-foreground));
  }
  .ansi input {
    width: 34px;
    flex: none;
  }
  .row {
    display: flex;
    gap: 8px;
  }
  .small {
    font-size: 12px;
  }
  .err {
    color: hsl(var(--destructive));
    font-size: 12.5px;
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
  .btn.danger {
    color: hsl(var(--destructive));
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
