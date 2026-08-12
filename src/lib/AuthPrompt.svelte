<script lang="ts">
  import { ShieldCheck } from '@lucide/svelte'
  import Challenge from './Challenge.svelte'
  import { answerPrompt, authPrompts } from './state.svelte'
  import { trapFocus } from './actions/trapFocus'

  // Challenges from connections no pane owns — a port forward or an SFTP
  // browser reaching a host that wants a verification code. Only the head of the
  // queue is shown, so two background connects can't fight over the dialog.
  const current = $derived(authPrompts.queue[0] ?? null)
  const cancel = () => current && answerPrompt(current.prompt_id, null)
</script>

{#if current}
  {@const title = current.name.trim() || 'Verification required'}
  <div class="backdrop" role="presentation">
    <div
      class="dialog"
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      aria-label={title}
      use:trapFocus={{ onclose: cancel }}
    >
      <h2><ShieldCheck size={16} /> {title}</h2>
      <p class="who mono">{current.label}</p>
      <!-- Keyed so a second challenge on the same connection remounts the form
           with empty fields rather than reusing the answers just sent. -->
      {#key current.prompt_id}
        <Challenge prompt={current} autofocus />
      {/key}
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 20vh;
    z-index: 80;
  }
  .dialog {
    width: 420px;
    max-width: 92vw;
    background: hsl(var(--popover));
    border: 1px solid hsl(var(--border));
    border-radius: 12px;
    padding: 18px 20px;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.55);
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin: 0 0 2px;
    font-size: 14.5px;
    font-weight: 600;
  }
  .who {
    margin: 0 0 12px;
    font-size: 12px;
    color: hsl(var(--muted-foreground));
  }
</style>
