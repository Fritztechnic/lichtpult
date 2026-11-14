<script lang="ts">
  // Breiter vertikaler Fader (Zeiger-gesteuert) mit Prozent- oder Rohwert-Anzeige.
  let { value = 0, max = 255, color = '#c03030', label = '', unit = 'pct', onchange, onrelease, ondblclick }: {
    value?: number; max?: number; color?: string; label?: string; unit?: 'pct' | 'raw'
    onchange?: (v: number) => void; onrelease?: () => void; ondblclick?: () => void
  } = $props()
  let track: HTMLDivElement
  let drag = false
  const pos = (e: PointerEvent) => {
    const r = track.getBoundingClientRect()
    const f = 1 - (e.clientY - r.top) / r.height
    onchange?.(Math.round(Math.min(1, Math.max(0, f)) * max))
  }
  const txt = $derived(unit === 'pct' ? Math.round((value / max) * 100) + '%' : String(value))
</script>

<div class="fader">
  <div class="val mono">{txt}</div>
  <div class="track" bind:this={track} role="slider" aria-valuenow={value} tabindex="0"
    onpointerdown={(e) => { drag = true; track.setPointerCapture(e.pointerId); pos(e) }}
    onpointermove={(e) => drag && pos(e)}
    onpointerup={() => { drag = false; onrelease?.() }}
    ondblclick={() => ondblclick?.()}>
    <div class="fill" style="height:{(value / max) * 100}%;background:linear-gradient(90deg,{color},color-mix(in srgb,{color} 55%,#000))"></div>
    <div class="cap" style="bottom:calc({(value / max) * 100}% - 7px)"></div>
  </div>
  {#if label}<div class="lbl">{label}</div>{/if}
</div>

<style>
  .fader { display: flex; flex-direction: column; align-items: stretch; gap: 3px; width: 100%; min-width: 46px; height: 100%; }
  .val { text-align: center; color: var(--orange); font-size: 12px; font-weight: 700; background: #000; border: 1px solid #333; padding: 3px 0; }
  .track { position: relative; flex: 1; min-height: 80px; background: repeating-linear-gradient(0deg, #050505 0 19px, #141414 19px 20px); border: 1px solid #3a3a3a; cursor: ns-resize; touch-action: none; overflow: hidden; }
  .fill { position: absolute; left: 0; right: 0; bottom: 0; opacity: .85; }
  .cap { position: absolute; left: 0; right: 0; height: 14px; background: #d8d8d8; border-top: 1px solid #fff; border-bottom: 1px solid #777; pointer-events: none; }
  .lbl { text-align: center; font-size: 11px; background: var(--btn); border: 1px solid #3a3a3a; padding: 4px 2px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>
