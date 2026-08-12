<script lang="ts">
  import { Check, X, ChevronRight } from '@lucide/svelte'
  import { stepsFor, stepStatus, stampTime, STEP_LABEL, type Step, type LogLine } from './connsteps'
  // Driven by real ssh://state + ssh://log events — never a faked animation.
  // Each step is an accordion; opening one reveals its detailed log lines.
  let {
    phase,
    error,
    method = '',
    protocol = 'ssh',
    mfa = false,
    log = [],
    activeStep,
  }: {
    phase: string
    error: string
    method?: string
    protocol?: string
    mfa?: boolean // the server issued a challenge -> the MFA step applies here
    log?: LogLine[]
    activeStep: Step
  } = $props()

  const steps = $derived(stepsFor(protocol, mfa))
  const statuses = $derived(stepStatus(steps, activeStep, phase))

  // The step that opens by default: the active or failed one. A user click pins
  // `openStep`, overriding the auto-follow until they toggle it off.
  const autoStep = $derived(
    steps.find((_, i) => statuses[i] === 'active' || statuses[i] === 'failed') ?? null,
  )
  let openStep = $state<Step | 'none' | null>(null)
  // A fresh attempt clears the log — drop the pin so auto-follow resumes.
  $effect(() => {
    if (log.length === 0) openStep = null
  })
  const effectiveOpen = $derived(openStep ?? autoStep)
  function toggle(s: Step) {
    openStep = effectiveOpen === s ? 'none' : s
  }

  const label = (s: Step) => (s === 'auth' && method ? `Authenticate (${method})` : STEP_LABEL[s])
  const failed = $derived(phase === 'error' || phase === 'disconnected')
</script>

<div class="acc">
  {#each steps as step, i (step)}
    {@const st = statuses[i]}
    {@const lines = log.filter((l) => l.step === step)}
    {@const isOpen = effectiveOpen === step}
    <div class="step {st}">
      <button class="hd" onclick={() => toggle(step)} aria-expanded={isOpen}>
        <span class="si">
          {#if st === 'done'}<Check size={12} />
          {:else if st === 'failed'}<X size={12} />
          {:else if st === 'active'}<span class="spin"></span>{/if}
        </span>
        <span class="lbl">{label(step)}</span>
        <span class="spacer"></span>
        {#if lines.length}<span class="count">{lines.length}</span>{/if}
        <span class="chev" class:open={isOpen}><ChevronRight size={13} /></span>
      </button>
      {#if isOpen}
        <div class="body">
          {#if lines.length}
            {#each lines as l, li (li)}
              <div class="line"><span class="ts mono">{stampTime(l.ts)}</span><span class="msg mono">{l.msg}</span></div>
            {/each}
          {:else}
            <div class="line empty">No detail yet.</div>
          {/if}
        </div>
      {/if}
    </div>
  {/each}
</div>
{#if failed && error}<p class="errmsg">{phase === 'error' ? error : 'Disconnected'}</p>{/if}

<style>
  .acc {
    width: 300px;
    max-width: 82vw;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .hd {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 8px 6px;
    border: none;
    background: none;
    color: inherit;
    font: inherit;
    font-size: 13px;
    cursor: pointer;
    border-radius: 6px;
  }
  .hd:hover {
    background: hsl(var(--muted));
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
    text-align: left;
  }
  .step.pending .lbl,
  .step.done .lbl {
    color: hsl(var(--muted-foreground));
  }
  .step.active .lbl,
  .step.failed .lbl {
    color: hsl(var(--foreground));
  }
  .spacer {
    flex: 1;
  }
  .count {
    font-size: 10.5px;
    color: hsl(var(--muted-foreground));
    background: hsl(var(--muted));
    border-radius: 999px;
    padding: 1px 7px;
  }
  .chev {
    display: grid;
    place-items: center;
    color: hsl(var(--muted-foreground));
    transition: transform 0.12s ease;
  }
  .chev.open {
    transform: rotate(90deg);
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 2px 6px 8px 37px;
  }
  .line {
    display: flex;
    gap: 10px;
    font-size: 11.5px;
    line-height: 1.4;
  }
  .ts {
    color: hsl(var(--muted-foreground));
    flex: none;
  }
  .msg {
    color: hsl(var(--foreground));
    word-break: break-word;
  }
  .step.failed .msg {
    color: hsl(var(--destructive));
  }
  .empty {
    color: hsl(var(--muted-foreground));
    font-style: italic;
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
  .errmsg {
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
