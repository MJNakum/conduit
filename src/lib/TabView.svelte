<script lang="ts">
  import Pane from './Pane.svelte'
  import { type Tab } from './state.svelte'

  // Splits are deferred to their own track (see docs/splits.md). For now a tab
  // is a single pane. The Pane/Tab model already supports N panes so the
  // drag-and-drop split work slots in here without reshaping state.
  let { tab }: { tab: Tab } = $props()
</script>

<div class="tabview">
  <div class="grid {tab.layout}">
    {#each tab.panes as pane (pane.key)}
      <Pane
        {pane}
        active={tab.panes.length > 1 && tab.active === pane.key}
        onfocus={() => (tab.active = pane.key)}
      />
    {/each}
  </div>
</div>

<style>
  .tabview {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .grid {
    flex: 1;
    min-height: 0;
    display: grid;
    gap: 2px;
    background: hsl(var(--border));
  }
  .grid.single {
    grid-template-columns: 1fr;
  }
  .grid.split2 {
    grid-template-columns: 1fr 1fr;
  }
  .grid.split4 {
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
  }
</style>
