<script lang="ts">
  // Info-Symbol: Erklärung erscheint beim Überfahren mit der Maus.
  import InfoIcon from '@lucide/svelte/icons/info'
  let { text }: { text: string } = $props()
  let open = $state(false)
</script>

<span class="info" role="note" onmouseenter={() => (open = true)} onmouseleave={() => (open = false)}>
  <InfoIcon size={14} />
  {#if open}<span class="tip">{text}</span>{/if}
</span>

<style>
  .info { position: relative; display: inline-flex; color: var(--dim); cursor: help; }
  .info:hover { color: #fff; }
  .tip { position: absolute; top: 20px; right: -6px; z-index: 40; width: 240px; padding: 8px 10px; background: #1d1d1d; border: 1px solid #666; color: #ddd; font-size: 12px; line-height: 1.35; white-space: normal; box-shadow: 0 4px 14px #000; }
</style>
