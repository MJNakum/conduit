<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { getVersion } from '@tauri-apps/api/app'
  import { Copy, Save, X } from '@lucide/svelte'
  import { formatLog, stampTime, type LogLine } from './connsteps'
  import { toast } from './toast.svelte'

  // The whole attempt as one flat trace, rather than the per-step accordion —
  // when you're debugging you want every line in order, selectable, in one
  // place. `previous` is the attempt before this one, kept so a retry doesn't
  // erase what actually failed.
  let {
    log = [],
    previous = [],
    host,
    name,
    onclose,
  }: {
    log?: LogLine[]
    previous?: LogLine[]
    host: string // "user@host:port", for the file header
    name: string // host name, used for the saved log's filename
    onclose: () => void
  } = $props()

  let version = $state('')
  $effect(() => {
    getVersion().then((v) => (version = v))
  })

  const text = $derived(formatLog(log, { host, version }, previous))

  async function copy() {
    try {
      await navigator.clipboard.writeText(text)
      toast('Connection log copied')
    } catch {
      // Fallback for a webview that denies the async clipboard API.
      const ta = document.createElement('textarea')
      ta.value = text
      document.body.appendChild(ta)
      ta.select()
      document.execCommand('copy')
      ta.remove()
      toast('Connection log copied')
    }
  }

  async function save() {
    try {
      const file = await invoke<string>('conn_log_save', { name, text })
      await invoke('log_reveal', { file })
    } catch (e) {
      toast(String(e), 'err')
    }
  }
</script>

<div class="raw">
  <div class="bar">
    <span class="title">Connection log</span>
    <span class="spacer"></span>
    <button class="icon" onclick={copy} title="Copy to clipboard" aria-label="Copy connection log">
      <Copy size={13} />
    </button>
    <button class="icon" onclick={save} title="Save to a file" aria-label="Save connection log">
      <Save size={13} />
    </button>
    <button class="icon" onclick={onclose} title="Close" aria-label="Close connection log">
      <X size={13} />
    </button>
  </div>
  <div class="body">
    {#if previous.length}
      <div class="divider">previous attempt</div>
      {#each previous as l, i (i)}
        <div class="line old">
          <span class="ts mono">{stampTime(l.ts)}</span>
          <span class="st mono">{l.step}</span>
          <span class="msg mono">{l.msg}</span>
        </div>
      {/each}
      <div class="divider">this attempt</div>
    {/if}
    {#each log as l, i (i)}
      <div class="line">
        <span class="ts mono">{stampTime(l.ts)}</span>
        <span class="st mono">{l.step}</span>
        <span class="msg mono">{l.msg}</span>
      </div>
    {:else}
      <div class="line empty">No detail recorded.</div>
    {/each}
  </div>
</div>

<style>
  .raw {
    width: 520px;
    max-width: 92vw;
    max-height: 60vh;
    display: flex;
    flex-direction: column;
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border));
    border-radius: 10px;
    overflow: hidden;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 8px 6px 12px;
    border-bottom: 1px solid hsl(var(--border));
    flex: none;
  }
  .title {
    font-size: 12.5px;
    font-weight: 600;
  }
  .spacer {
    flex: 1;
  }
  .icon {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: none;
    border-radius: 6px;
    background: none;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
  }
  .icon:hover {
    background: hsl(var(--muted));
    color: hsl(var(--foreground));
  }
  .body {
    overflow: auto;
    padding: 8px 12px 12px;
    user-select: text;
  }
  .line {
    display: flex;
    gap: 10px;
    font-size: 11.5px;
    line-height: 1.5;
    text-align: left;
  }
  .ts {
    color: hsl(var(--muted-foreground));
    flex: none;
  }
  .st {
    color: hsl(var(--muted-foreground));
    flex: none;
    width: 68px;
  }
  .msg {
    word-break: break-word;
  }
  .line.old .msg {
    color: hsl(var(--muted-foreground));
  }
  .divider {
    margin: 6px 0 3px;
    font-size: 10.5px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: hsl(var(--muted-foreground));
    border-bottom: 1px solid hsl(var(--border));
  }
  .empty {
    color: hsl(var(--muted-foreground));
    font-style: italic;
  }
</style>
