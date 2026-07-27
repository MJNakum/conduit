<script lang="ts">
  import Appearance from './Appearance.svelte'
  import Shortcuts from './Shortcuts.svelte'
  import { checkForUpdates, updateState } from './updates.svelte'

  let { tab = $bindable('appearance') }: { tab?: 'appearance' | 'shortcuts' | 'about' } = $props()
</script>

<div class="settings">
  <div class="tabs">
    <button class:active={tab === 'appearance'} onclick={() => (tab = 'appearance')}>Appearance</button>
    <button class:active={tab === 'shortcuts'} onclick={() => (tab = 'shortcuts')}>Shortcuts</button>
    <button class:active={tab === 'about'} onclick={() => (tab = 'about')}>About</button>
  </div>
  <div class="content">
    {#if tab === 'appearance'}
      <Appearance />
    {:else if tab === 'shortcuts'}
      <Shortcuts />
    {:else}
      <div class="about">
        <button class="update" onclick={() => checkForUpdates(true)} disabled={updateState.checking}>
          {updateState.checking ? 'Checking…' : 'Check for updates'}
        </button>
        <p class="hint">Updates install automatically once downloaded, then the app relaunches.</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .settings {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .tabs {
    flex: none;
    display: flex;
    gap: 4px;
    padding: 12px 16px 0;
    border-bottom: 1px solid hsl(var(--border));
  }
  .tabs button {
    padding: 8px 14px;
    border: none;
    background: none;
    color: hsl(var(--muted-foreground));
    font: inherit;
    font-size: 13px;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
  }
  .tabs button.active {
    color: hsl(var(--foreground));
    border-bottom-color: hsl(var(--primary));
  }
  .content {
    flex: 1;
    overflow: auto;
    padding: 18px 16px;
  }
  .about {
    display: flex;
    flex-direction: column;
    gap: 10px;
    align-items: flex-start;
  }
  .about .update {
    padding: 8px 14px;
    border: 1px solid hsl(var(--border));
    border-radius: 6px;
    background: hsl(var(--secondary));
    color: hsl(var(--foreground));
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .about .update:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .about .hint {
    margin: 0;
    color: hsl(var(--muted-foreground));
    font-size: 12px;
  }
</style>
