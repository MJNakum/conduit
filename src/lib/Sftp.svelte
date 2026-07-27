<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { FolderOpen, Folder, File, ArrowUp, Download, Upload, RefreshCw } from '@lucide/svelte'
  import { openSftp, sftpList, sftpDownload, sftpUpload, sftpClose, type SftpEntry, type Host } from './state.svelte'
  import { trapFocus } from './actions/trapFocus'
  import { promptDialog } from './dialog.svelte'

  let { host, onclose }: { host: Host; onclose: () => void } = $props()

  let id = $state<string | null>(null)
  let cwd = $state('/')
  let entries = $state<SftpEntry[]>([])
  let err = $state('')
  let busy = $state(false)

  const join = (dir: string, name: string) => (dir === '/' ? '/' + name : dir + '/' + name)
  const parent = (p: string) => {
    const i = p.replace(/\/+$/, '').lastIndexOf('/')
    return i <= 0 ? '/' : p.slice(0, i)
  }
  const fmtSize = (n: number) =>
    n < 1024 ? `${n} B` : n < 1048576 ? `${(n / 1024).toFixed(1)} K` : `${(n / 1048576).toFixed(1)} M`

  async function list(path: string) {
    if (!id) return
    busy = true
    err = ''
    try {
      entries = await sftpList(id, path)
      cwd = path
    } catch (e) {
      err = String(e)
    } finally {
      busy = false
    }
  }

  async function download(e: SftpEntry) {
    if (!id) return
    const local = (await promptDialog({ title: 'Download to', value: `~/Downloads/${e.name}`, placeholder: 'Local path' }))?.trim()
    if (!local) return
    try {
      await sftpDownload(id, join(cwd, e.name), local)
    } catch (err2) {
      err = String(err2)
    }
  }

  async function upload() {
    if (!id) return
    const local = (await promptDialog({ title: 'Upload local file', placeholder: '/path/to/file' }))?.trim()
    if (!local) return
    const base = local.split('/').pop() ?? 'upload'
    try {
      await sftpUpload(id, local, join(cwd, base))
      list(cwd)
    } catch (e) {
      err = String(e)
    }
  }

  onMount(async () => {
    try {
      const o = await openSftp(host.id)
      id = o.id
      await list(o.cwd)
    } catch (e) {
      err = String(e)
    }
  })
  onDestroy(() => {
    if (id) sftpClose(id)
  })
</script>

<div class="backdrop" onclick={onclose} role="presentation">
  <div class="sheet" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1" aria-modal="true" use:trapFocus={{ onclose }}>
    <div class="sh">
      <FolderOpen size={15} /> SFTP — {host.name}
      <span class="spacer"></span>
      <button class="icon" title="Upload here" onclick={upload}><Upload size={15} /></button>
      <button class="icon" title="Refresh" onclick={() => list(cwd)}><RefreshCw size={15} /></button>
      <button class="close" onclick={onclose}>Close</button>
    </div>
    <div class="path mono">
      <button class="up" title="Up" onclick={() => list(parent(cwd))} disabled={cwd === '/'}><ArrowUp size={13} /></button>
      {cwd}
    </div>
    {#if err}<div class="err">{err}</div>{/if}
    <div class="body">
      {#if !id}
        <p class="muted small">Connecting…</p>
      {:else if entries.length === 0 && !busy}
        <p class="muted small">Empty directory.</p>
      {:else}
        <ul>
          {#each entries as e (e.name)}
            <li>
              {#if e.is_dir}
                <button class="row dir" onclick={() => list(join(cwd, e.name))}>
                  <Folder size={15} /> <span class="nm">{e.name}</span>
                </button>
              {:else}
                <div class="row">
                  <File size={15} /> <span class="nm">{e.name}</span>
                  <span class="sz muted mono">{fmtSize(e.size)}</span>
                  <button class="dl icon" title="Download" onclick={() => download(e)}><Download size={14} /></button>
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.4); z-index: 70; display: flex; justify-content: flex-end; }
  .sheet { width: 460px; max-width: 92vw; height: 100%; background: hsl(var(--card)); border-left: 1px solid hsl(var(--border)); display: flex; flex-direction: column; box-shadow: -12px 0 40px rgba(0, 0, 0, 0.4); }
  .sh { display: flex; align-items: center; gap: 9px; padding: 13px 16px; border-bottom: 1px solid hsl(var(--border)); font-size: 14px; font-weight: 600; }
  .spacer { flex: 1; }
  .close { border: 1px solid hsl(var(--border)); background: none; color: hsl(var(--muted-foreground)); border-radius: 6px; padding: 4px 10px; font: inherit; font-size: 12px; cursor: pointer; }
  .close:hover { background: hsl(var(--muted)); }
  .path { display: flex; align-items: center; gap: 8px; padding: 8px 16px; border-bottom: 1px solid hsl(var(--border)); font-size: 12px; color: hsl(var(--muted-foreground)); word-break: break-all; }
  .up { display: grid; place-items: center; width: 24px; height: 24px; border: 1px solid hsl(var(--border)); background: none; color: inherit; border-radius: 6px; cursor: pointer; flex: none; }
  .up:disabled { opacity: 0.4; cursor: default; }
  .body { flex: 1; overflow: auto; padding: 6px 10px; }
  ul { list-style: none; margin: 0; padding: 0; }
  .row { display: flex; align-items: center; gap: 9px; width: 100%; padding: 6px 8px; border: none; background: none; color: inherit; font: inherit; font-size: 13px; text-align: left; border-radius: 6px; }
  button.row { cursor: pointer; }
  button.row:hover { background: hsl(var(--muted)); }
  .nm { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sz { flex: none; font-size: 11px; }
  .icon { display: grid; place-items: center; width: 28px; height: 28px; border: none; background: none; color: hsl(var(--muted-foreground)); border-radius: 6px; cursor: pointer; flex: none; }
  .icon:hover { background: hsl(var(--muted)); color: hsl(var(--foreground)); }
  .dl { width: 24px; height: 24px; }
  .err { color: hsl(var(--destructive)); font-size: 12px; padding: 8px 16px; }
  .small { padding: 12px 16px; }
</style>
