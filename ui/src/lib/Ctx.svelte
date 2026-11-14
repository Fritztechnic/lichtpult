<script lang="ts">
  // Rechtsklick-Menü. Der Inhalt kommt aus `S.menu(event, items)`; Klick, Escape oder
  // ein weiterer Rechtsklick schließen es.
  import { S } from '../store.svelte'

  const c = $derived(S.ctx)
  // Menü im sichtbaren Bereich halten
  const pos = $derived(c ? { x: Math.min(c.x, window.innerWidth - 200), y: Math.min(c.y, window.innerHeight - c.items.length * 28 - 10) } : null)
  const close = () => (S.ctx = null)
</script>

<svelte:window onclick={close} onkeydown={(e) => e.key === 'Escape' && close()} oncontextmenu={(e) => { e.preventDefault(); close() }} onblur={close} />

{#if c && pos}
  <div class="cm" style="left:{pos.x}px;top:{pos.y}px" role="menu">
    {#each c.items as it}
      {#if it.sep}<hr />
      {:else}<button role="menuitem" class:danger={it.danger} disabled={it.disabled} onclick={() => { close(); it.run?.() }}>{it.label}</button>{/if}
    {/each}
  </div>
{/if}

<style>
  .cm { position: fixed; z-index: 100; min-width: 170px; background: #121212; border: 1px solid #666; box-shadow: 0 6px 24px #000c; padding: 2px; }
  .cm button { display: block; width: 100%; text-align: left; padding: 6px 12px; font-size: 12px; }
  .cm button:hover:not(:disabled) { background: #2a64c8; color: #fff; }
  .cm button.danger { color: #ff7070; } .cm button.danger:hover:not(:disabled) { background: var(--red); color: #fff; }
  .cm button:disabled { color: #555; cursor: default; }
  .cm hr { border: 0; border-top: 1px solid #333; margin: 2px 0; }
</style>
