<script lang="ts">
  import { Check, AlertCircle } from '@lucide/svelte'
  import { toasts } from './toast.svelte'
</script>

<div class="toaster">
  {#each toasts.list as t (t.id)}
    <div class="toast {t.kind}">
      {#if t.kind === 'err'}<AlertCircle size={15} />{:else}<Check size={15} />{/if}
      <span>{t.msg}</span>
    </div>
  {/each}
</div>

<style>
  .toaster {
    position: fixed;
    bottom: 46px;
    right: 16px;
    z-index: 80;
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: flex-end;
    pointer-events: none;
  }
  .toast {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 9px 13px;
    border-radius: 9px;
    background: hsl(var(--popover));
    border: 1px solid hsl(var(--border));
    color: hsl(var(--foreground));
    font-size: 12.5px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.4);
    animation: rise 0.16s ease;
  }
  .toast.ok :global(svg) {
    color: hsl(var(--primary));
  }
  .toast.err {
    border-color: hsl(var(--destructive) / 0.6);
  }
  .toast.err :global(svg) {
    color: hsl(var(--destructive));
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .toast {
      animation: none;
    }
  }
</style>
