<script lang="ts">
  // Senkrechte Trennlinie vor der rechten Spalte einer Ansicht. `key` ist die Spaltenbreite in
  // `S.layout`; der Container setzt `--w` und ist `position: relative`.
  import { S, DEFAULT_LAYOUT, type Layout } from '../store.svelte'
  import { startDrag } from './split'

  let { key, min = 200, max = 700 }: { key: keyof Layout; min?: number; max?: number } = $props()

  function drag(e: PointerEvent) {
    const w0 = S.layout[key]
    startDrag(e, (dx) => (S.layout[key] = Math.round(Math.min(max, Math.max(min, w0 - dx)))), () => S.saveLayout())
  }
  const reset = () => { S.layout[key] = DEFAULT_LAYOUT[key]; S.saveLayout() }
</script>

<div class="hs" role="separator" aria-orientation="vertical" title="Ziehen: Spaltenbreite ändern, Doppelklick: zurücksetzen" style="right:calc(var(--w) - 1px)" onpointerdown={drag} ondblclick={reset}></div>

<style>
  .hs { position: absolute; top: 0; bottom: 0; width: 7px; margin-right: -3px; cursor: ew-resize; z-index: 20; }
  .hs:hover { background: #5a7fc866; }
</style>
