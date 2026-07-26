<script lang="ts">
  import Appearance from './Appearance.svelte'
  import Shortcuts from './Shortcuts.svelte'

  let { tab = $bindable('appearance') }: { tab?: 'appearance' | 'shortcuts' } = $props()
</script>

<div class="settings">
  <div class="tabs">
    <button class:active={tab === 'appearance'} onclick={() => (tab = 'appearance')}>Appearance</button>
    <button class:active={tab === 'shortcuts'} onclick={() => (tab = 'shortcuts')}>Shortcuts</button>
  </div>
  <div class="content">
    {#if tab === 'appearance'}
      <Appearance />
    {:else}
      <Shortcuts />
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
</style>
