<script lang="ts">
  import { onMount } from 'svelte'
  import { getVersion } from '@tauri-apps/api/app'
  import { invoke } from '@tauri-apps/api/core'
  import Appearance from './Appearance.svelte'
  import Shortcuts from './Shortcuts.svelte'
  import { checkForUpdates, updateState } from './updates.svelte'
  import { Palette, Keyboard, Info } from '@lucide/svelte'

  let { tab = $bindable('appearance') }: { tab?: 'appearance' | 'shortcuts' | 'about' } = $props()

  let version = $state('')
  onMount(async () => {
    version = await getVersion()
  })

  // --- Edit these to your details ---------------------------------------
  // Brand marks are inlined as SVG paths: lucide removed brand icons (github,
  // linkedin) over trademark concerns, so they can't be imported.
  const author = 'Meet Nakum'
  const web =
    'M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z'
  const gh =
    'M12 1C5.923 1 1 5.923 1 12c0 4.867 3.149 8.979 7.521 10.436.55.096.756-.233.756-.522 0-.262-.013-1.128-.013-2.049-2.764.509-3.479-.674-3.699-1.292-.124-.317-.66-1.293-1.127-1.554-.385-.207-.936-.715-.014-.729.866-.014 1.485.797 1.691 1.128.99 1.663 2.571 1.196 3.204.907.096-.715.385-1.196.701-1.471-2.448-.275-5.005-1.224-5.005-5.432 0-1.196.426-2.186 1.128-2.956-.111-.275-.496-1.402.11-2.915 0 0 .921-.288 3.024 1.128a10.193 10.193 0 0 1 2.75-.371c.936 0 1.871.123 2.75.371 2.104-1.43 3.025-1.128 3.025-1.128.605 1.513.221 2.64.111 2.915.701.77 1.127 1.747 1.127 2.956 0 4.222-2.571 5.157-5.019 5.432.399.344.743 1.004.743 2.035 0 1.471-.014 2.654-.014 3.025 0 .289.206.632.756.522C19.851 20.979 23 16.854 23 12c0-6.077-4.922-11-11-11Z'
  const x =
    'M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z'
  const li =
    'M20.447 20.452h-3.554v-5.569c0-1.328-.027-3.037-1.852-3.037-1.853 0-2.136 1.445-2.136 2.939v5.667H9.351V9h3.414v1.561h.046c.477-.9 1.637-1.85 3.37-1.85 3.601 0 4.267 2.37 4.267 5.455v6.286zM5.337 7.433a2.062 2.062 0 0 1-2.063-2.065 2.064 2.064 0 1 1 2.063 2.065zm1.782 13.019H3.555V9h3.564v11.452zM22.225 0H1.771C.792 0 0 .774 0 1.729v20.542C0 23.227.792 24 1.771 24h20.451C23.2 24 24 23.227 24 22.271V1.729C24 .774 23.2 0 22.225 0z'
  const socials = [
    { label: 'Website', url: 'https://mjnakum.com/', path: web },
    { label: 'GitHub', url: 'https://github.com/MJNakum', path: gh },
    { label: 'X', url: 'https://twitter.com/MeetNakum31', path: x },
    { label: 'LinkedIn', url: 'https://www.linkedin.com/in/meetnakum/', path: li },
  ]
  // ----------------------------------------------------------------------

  const openLink = (url: string) => invoke('open_url', { url })
</script>

<div class="settings">
  <nav class="tabs" aria-label="Settings sections">
    <button class:active={tab === 'appearance'} onclick={() => (tab = 'appearance')}><Palette size={15} /> Appearance</button>
    <button class:active={tab === 'shortcuts'} onclick={() => (tab = 'shortcuts')}><Keyboard size={15} /> Shortcuts</button>
    <button class:active={tab === 'about'} onclick={() => (tab = 'about')}><Info size={15} /> About</button>
  </nav>
  <div class="content">
    {#if tab === 'appearance'}
      <Appearance />
    {:else if tab === 'shortcuts'}
      <Shortcuts />
    {:else}
      <div class="about">
        <div class="brand">
          <svg class="mark" viewBox="0 0 256 256" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
            <path d="M80 76 L152 128 L80 180" stroke="hsl(var(--primary))" stroke-width="22" stroke-linecap="round" stroke-linejoin="round" />
            <rect x="96" y="188" width="80" height="22" rx="11" fill="hsl(var(--primary))" />
          </svg>
          <div>
            <div class="app">Conduit</div>
            <div class="tag muted">Fast, local-first SSH client</div>
          </div>
        </div>

        <div class="ver">
          <span class="mono">Version {version || '…'}</span>
          <button class="update" onclick={() => checkForUpdates(true)} disabled={updateState.checking}>
            {updateState.checking ? 'Checking…' : 'Check for updates'}
          </button>
        </div>
        <p class="hint">Updates install automatically once downloaded, then the app relaunches.</p>

        <div class="sep"></div>

        <div class="by muted">Made by {author}</div>
        <div class="socials">
          {#each socials as s}
            <button class="social" onclick={() => openLink(s.url)}>
              <svg class="brand" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d={s.path} /></svg>
              <span>{s.label}</span>
            </button>
          {/each}
        </div>

        <div class="copy muted">© {new Date().getFullYear()} {author}</div>
      </div>
    {/if}
  </div>
</div>

<style>
  .settings {
    height: 100%;
    display: flex;
    flex-direction: row;
    overflow: hidden;
  }
  .tabs {
    flex: none;
    width: 176px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 16px 10px;
    border-right: 1px solid hsl(var(--border));
  }
  .tabs button {
    display: flex;
    align-items: center;
    gap: 9px;
    text-align: left;
    padding: 8px 11px;
    border: none;
    border-radius: 7px;
    background: none;
    color: hsl(var(--muted-foreground));
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .tabs button:hover {
    background: hsl(var(--muted));
    color: hsl(var(--foreground));
  }
  .tabs button.active {
    color: hsl(var(--foreground));
    background: hsl(var(--muted));
    font-weight: 600;
  }
  .content {
    flex: 1;
    overflow: auto;
    padding: 22px 24px;
  }
  .about {
    display: flex;
    flex-direction: column;
    gap: 10px;
    align-items: flex-start;
    max-width: 420px;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .brand .mark {
    width: 44px;
    height: 44px;
    flex: none;
  }
  .brand .app {
    font-size: 17px;
    font-weight: 600;
  }
  .brand .tag {
    font-size: 12px;
  }
  .ver {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 4px;
  }
  .ver .mono {
    font-size: 12px;
    color: hsl(var(--muted-foreground));
  }
  .update {
    padding: 8px 14px;
    border: 1px solid hsl(var(--border));
    border-radius: 6px;
    background: hsl(var(--muted));
    color: hsl(var(--foreground));
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .update:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .hint {
    margin: 0;
    color: hsl(var(--muted-foreground));
    font-size: 12px;
  }
  .sep {
    width: 100%;
    height: 1px;
    background: hsl(var(--border));
    margin: 8px 0;
  }
  .by {
    font-size: 13px;
  }
  .socials {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .social {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 7px 12px;
    border: 1px solid hsl(var(--border));
    border-radius: 6px;
    background: none;
    color: hsl(var(--foreground));
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .social:hover {
    background: hsl(var(--muted));
    border-color: hsl(var(--primary));
  }
  .social .brand {
    width: 15px;
    height: 15px;
    flex: none;
  }
  .copy {
    font-size: 11px;
    margin-top: 4px;
  }
</style>
