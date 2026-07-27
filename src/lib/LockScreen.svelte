<script lang="ts">
  import { Lock, Fingerprint } from '@lucide/svelte'
  import { vault, unlockVault } from './vault.svelte'
  import { toast } from './toast.svelte'
  import { trapFocus } from './actions/trapFocus'

  async function unlock() {
    const ok = await unlockVault()
    if (!ok) toast('Authentication failed', 'err')
  }
</script>

<div class="lock">
  <div class="card" role="dialog" aria-modal="true" aria-label="Vault locked" tabindex="-1" use:trapFocus={{}}>
    <span class="ico"><Lock size={30} /></span>
    <h2>Vault locked</h2>
    <p class="muted">Authenticate to unlock your hosts, keys, and sessions.</p>
    <button class="btn" onclick={unlock} disabled={vault.authing}>
      <Fingerprint size={16} /> {vault.authing ? 'Waiting…' : 'Unlock'}
    </button>
  </div>
</div>

<style>
  .lock {
    position: fixed;
    inset: 0;
    z-index: 90;
    display: grid;
    place-items: center;
    background: hsl(var(--background) / 0.72);
    backdrop-filter: blur(18px);
  }
  .card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 34px 40px;
    border: 1px solid hsl(var(--border));
    border-radius: 16px;
    background: hsl(var(--card));
    box-shadow: 0 30px 80px rgba(0, 0, 0, 0.5);
    text-align: center;
  }
  .ico {
    display: grid;
    place-items: center;
    width: 60px;
    height: 60px;
    border-radius: 50%;
    background: hsl(var(--primary) / 0.12);
    color: hsl(var(--primary));
  }
  h2 {
    margin: 4px 0 0;
    font-size: 17px;
  }
  p {
    margin: 0;
    font-size: 13px;
    max-width: 260px;
  }
  .btn {
    margin-top: 8px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 9px 18px;
    border: none;
    border-radius: 8px;
    background: hsl(var(--primary));
    color: hsl(var(--primary-foreground));
    font: inherit;
    font-weight: 600;
    font-size: 13px;
    cursor: pointer;
  }
  .btn:disabled {
    opacity: 0.6;
    cursor: default;
  }
</style>
