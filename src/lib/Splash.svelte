<script lang="ts">
  import { onMount } from 'svelte'

  let { done }: { done: () => void } = $props()
  let leaving = $state(false)

  onMount(() => {
    const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches
    const hold = reduce ? 150 : 880
    const t1 = setTimeout(() => (leaving = true), hold)
    const t2 = setTimeout(done, hold + 320)
    return () => {
      clearTimeout(t1)
      clearTimeout(t2)
    }
  })
</script>

<div class="splash" class:leaving aria-hidden="true">
  <svg class="mark" viewBox="0 0 256 256" fill="none" xmlns="http://www.w3.org/2000/svg">
    <defs>
      <linearGradient id="conduit-splash" x1="80" y1="76" x2="176" y2="210" gradientUnits="userSpaceOnUse">
        <stop offset="0" stop-color="#2DD4BF" />
        <stop offset="1" stop-color="#22D3EE" />
      </linearGradient>
    </defs>
    <path
      class="chev"
      d="M80 76 L152 128 L80 180"
      stroke="url(#conduit-splash)"
      stroke-width="22"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
    <rect class="cur" x="96" y="188" width="80" height="22" rx="11" fill="url(#conduit-splash)" />
  </svg>
</div>

<style>
  .splash {
    position: fixed;
    inset: 0;
    z-index: 999;
    display: grid;
    place-items: center;
    background: hsl(var(--background));
    transition: opacity 300ms var(--ease);
  }
  .splash.leaving {
    opacity: 0;
  }
  .mark {
    width: 84px;
    height: 84px;
    filter: drop-shadow(0 6px 20px hsl(var(--primary) / 0.25));
  }
  /* Draw the chevron stroke in, then pop the cursor bar. */
  .chev {
    stroke-dasharray: 200;
    stroke-dashoffset: 200;
    animation: draw 600ms var(--ease) forwards;
  }
  .cur {
    opacity: 0;
    animation: pop 280ms var(--ease) 540ms forwards;
  }
  @keyframes draw {
    to {
      stroke-dashoffset: 0;
    }
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .chev {
      stroke-dashoffset: 0;
      animation: none;
    }
    .cur {
      opacity: 1;
      animation: none;
    }
  }
</style>
