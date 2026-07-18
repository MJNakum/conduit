<script lang="ts">
  // Driven by real ssh://state events — never a faked animation.
  let { phase, error }: { phase: string; error: string } = $props()

  const steps = ['connecting', 'authenticating', 'connected']
  const labels: Record<string, string> = {
    connecting: 'Connecting',
    authenticating: 'Authenticating',
    connected: 'Connected',
  }
  // Index of the current phase in the pipeline (-1 before it starts).
  const idx = $derived(steps.indexOf(phase))
  const failed = $derived(phase === 'error' || phase === 'disconnected')
</script>

<div class="stepper">
  {#each steps as step, i}
    <div class="step" class:done={idx > i} class:active={idx === i} class:failed={failed && idx < i}>
      <span class="bullet"></span>
      <span class="label">{labels[step]}</span>
    </div>
    {#if i < steps.length - 1}<span class="line" class:done={idx > i}></span>{/if}
  {/each}
</div>
{#if failed}
  <p class="msg">{phase === 'error' ? error || 'Connection error' : 'Disconnected'}</p>
{/if}

<style>
  .stepper {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
  }
  .step {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    color: #666;
  }
  .bullet {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #333;
    border: 2px solid #444;
  }
  .step.active .bullet {
    background: #2b6cff;
    border-color: #2b6cff;
    animation: pulse 1s infinite;
  }
  .step.done {
    color: #7c7;
  }
  .step.done .bullet {
    background: #3c3;
    border-color: #3c3;
  }
  .step.active {
    color: #6cf;
  }
  .step.failed .bullet {
    background: #a33;
    border-color: #a33;
  }
  .line {
    width: 32px;
    height: 2px;
    background: #333;
  }
  .line.done {
    background: #3c3;
  }
  .msg {
    text-align: center;
    color: #f66;
    font-size: 0.85rem;
  }
  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.4;
    }
  }
</style>
