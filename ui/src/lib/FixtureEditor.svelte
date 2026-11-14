<script lang="ts">
  // Editor für eigene Fixture-Typen: Name und Kanalbelegung in DMX-Reihenfolge.
  // "Create" legt den Typ in der Show an (`AddTypes`); danach lässt er sich im Patch verwenden.
  import { S } from '../store.svelte'
  import { ATTR_LABEL } from './types'
  import Plus from '@lucide/svelte/icons/plus'
  import X from '@lucide/svelte/icons/x'
  import ArrowUp from '@lucide/svelte/icons/arrow-up'
  import ArrowDown from '@lucide/svelte/icons/arrow-down'

  const show = $derived(S.show!)
  let name = $state('My Fixture')
  let chans = $state<string[]>(['Dimmer'])

  // Vorlagen für häufige Geräte
  const templates: [string, string[]][] = [
    ['Dimmer', ['Dimmer']],
    ['RGB', ['Red', 'Green', 'Blue']],
    ['RGBW', ['Red', 'Green', 'Blue', 'White']],
    ['Dimmer + RGB', ['Dimmer', 'Red', 'Green', 'Blue']],
    ['Moving Head', ['Pan', 'Tilt', 'Dimmer', 'Red', 'Green', 'Blue', 'White', 'Zoom', 'Strobe']],
  ]
  const attrs = Object.keys(ATTR_LABEL)
  const taken = $derived(show.types.some((t) => t.name === name.trim()))
  const dupes = $derived(chans.filter((c, i) => c && chans.indexOf(c) !== i))
  const ok = $derived(name.trim() !== '' && !taken && chans.length > 0 && !dupes.length)

  const move = (i: number, d: number) => {
    const j = i + d
    if (j < 0 || j >= chans.length) return
    ;[chans[i], chans[j]] = [chans[j], chans[i]]
  }
  async function create() {
    // leere Kanäle bleiben als belegte, ungenutzte Kanäle (`null`) erhalten
    await S.act({ AddTypes: [{ name: name.trim(), channels: chans.map((c) => c || null) }] })
    S.say(`Fixture type “${name.trim()}” created`)
    S.closeWin('typedit:0')
  }
</script>

<label class="f">Name<input bind:value={name} />{#if taken}<span class="err">Name already exists</span>{/if}</label>
<div class="row wrap">
  <span class="dim">Template</span>
  {#each templates as [n, c]}<button class="btn sm" onclick={() => (chans = [...c])}>{n}</button>{/each}
</div>
<div class="list">
  {#each chans as _, i}
    <div class="row ch">
      <span class="mono dim" style="width:26px">{i + 1}</span>
      <select bind:value={chans[i]}>
        <option value="">– unused –</option>
        {#each attrs as a}<option value={a}>{ATTR_LABEL[a]}</option>{/each}
      </select>
      <button class="btn sm" aria-label="Up" disabled={i === 0} onclick={() => move(i, -1)}><ArrowUp size={12} /></button>
      <button class="btn sm" aria-label="Down" disabled={i === chans.length - 1} onclick={() => move(i, 1)}><ArrowDown size={12} /></button>
      <button class="btn sm" aria-label="Remove channel" onclick={() => (chans = chans.filter((_, k) => k !== i))}><X size={12} /></button>
    </div>
  {/each}
</div>
{#if dupes.length}<div class="err">Each attribute can be used once: {dupes.map((d) => ATTR_LABEL[d]).join(', ')}</div>{/if}
<div class="row">
  <button class="btn" onclick={() => chans.push('')}><Plus size={14} /> Add channel</button>
  <span class="spacer"></span>
  <span class="dim">{chans.length} ch</span>
  <button class="btn on" disabled={!ok} onclick={create}>Create</button>
</div>

<style>
  .list { display: flex; flex-direction: column; gap: 3px; overflow: auto; min-height: 0; flex: 1; }
  .ch select { flex: 1; }
  .err { color: var(--yellow); font-size: 11px; }
</style>
