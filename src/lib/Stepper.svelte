<script lang="ts">
  import { Check, X } from '@lucide/svelte'
  // Driven by real ssh://state events — never a faked animation. Only the three
  // phases the backend actually emits appear; richer steps (jump host, host-key,
  // MFA) arrive with their Phase 2 events.
  let { phase, error }: { phase: string; error: string } = $props()

  const steps = ['connecting', 'authenticating', 'connected']
  const labels: Record<string, string> = {
    connecting: 'Connecting',
    authenticating: 'Authenticating',
    connected: 'Connected',
  }
  const idx = $derived(steps.indexOf(phase))
  const failed = $derived(phase === 'error' || phase === 'disconnected')
</script>

<div class="stepper">
  {#each steps as step, i}
    {@const done = idx > i}
    {@const active = idx === i}
    {@const failhere = failed && idx <= i}
    <div class="step" class:done class:active class:failed={failhere}>
      <span class="si">
        {#if done}
          <Check size={12} />
        {:else if failhere}
          <X size={12} />
        {:else if active}
          <span class="spin"></span>
        {/if}
      </span>
      <span class="lbl">{labels[step]}</span>
    </div>
  {/each}
</div>
{#if failed}
  <p class="msg">{phase === 'error' ? error || 'Connection error' : 'Disconnected'}</p>
{/if}

<style>
  .stepper {
    width: 240px;
  }
  .step {
    display: flex;
    align-items: center;
    gap: 13px;
    padding: 7px 0;
  }
  .si {
    width: 20px;
    height: 20px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 50%;
    border: 1.5px solid hsl(var(--border));
    color: hsl(var(--muted-foreground));
  }
  .step.done .si {
    background: hsl(var(--primary));
    border-color: hsl(var(--primary));
    color: hsl(var(--primary-foreground));
  }
  .step.active .si {
    border-color: hsl(var(--connecting));
    color: hsl(var(--connecting));
  }
  .step.failed .si {
    background: hsl(var(--destructive));
    border-color: hsl(var(--destructive));
    color: #fff;
  }
  .lbl {
    font-size: 13px;
  }
  .step.done:not(.active) .lbl,
  .step:not(.done):not(.active):not(.failed) .lbl {
    color: hsl(var(--muted-foreground));
  }
  .spin {
    width: 12px;
    height: 12px;
    border: 2px solid hsl(var(--connecting) / 0.3);
    border-top-color: hsl(var(--connecting));
    border-radius: 50%;
    animation: sp 0.7s linear infinite;
  }
  @keyframes sp {
    to {
      transform: rotate(360deg);
    }
  }
  .msg {
    margin: 10px 0 0;
    font-size: 12px;
    color: hsl(var(--destructive));
  }
  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }
</style>
