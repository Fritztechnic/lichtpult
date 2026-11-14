<script lang="ts">
  // Universes: Überblick über alle DMX-Universen. Links die Liste mit Auslastung, in der Mitte die
  // 512 Kanäle live (Farbe = Fixture, Helligkeit = Wert), rechts Fixtures, freie Bereiche und Werkzeuge.
  import { S } from '../store.svelte'
  import { ATTR_LABEL } from './types'
  import Splitter from './Splitter.svelte'
  import Tool from './Tool.svelte'
  import Info from './Info.svelte'
  import Wand from '@lucide/svelte/icons/wand-sparkles'
  import Pointer from '@lucide/svelte/icons/mouse-pointer-2'
  import MoveRight from '@lucide/svelte/icons/move-right'

  const live = $derived(S.live!)
  const show = $derived(S.show!)
  let sel = $state(1)

  const width = (ty: number) => show.types[ty].channels.length
  const unis = $derived([...new Set([1, ...show.fixtures.map((f) => f.universe), ...Object.keys(live.frames).map(Number)])].sort((a, b) => a - b))
  const uni = $derived(unis.includes(sel) ? sel : unis[0])
  const frame = $derived(live.frames[String(uni)] ?? [])
  const fixtures = $derived(show.fixtures.filter((f) => f.universe === uni).sort((a, b) => a.address - b.address))

  // Besitzer je Kanal: Fixture-ID, Attribut, -2 = Überschneidung
  type Own = { id: number; attr: string | null } | null | 'clash'
  function owners(u: number) {
    const m = new Array<Own>(512).fill(null)
    for (const f of show.fixtures) {
      if (f.universe !== u) continue
      show.types[f.ty].channels.forEach((attr, i) => {
        const a = f.address - 1 + i
        if (a >= 0 && a < 512) m[a] = m[a] ? 'clash' : { id: f.id, attr }
      })
    }
    return m
  }
  const own = $derived(owners(uni))
  const used = (u: number) => owners(u).filter(Boolean).length
  const clashes = $derived(own.filter((o) => o === 'clash').length)
  const usedNow = $derived(own.filter(Boolean).length)

  // freie Bereiche als [von, bis] (1-basiert)
  const free = $derived.by(() => {
    const r: [number, number][] = []
    let s = -1
    own.forEach((o, i) => {
      if (!o && s < 0) s = i
      if ((o || i === 511) && s >= 0) { r.push([s + 1, o ? i : i + 1]); s = -1 }
    })
    return r
  })
  const nameOf = (id: number) => show.fixtures.find((f) => f.id === id)?.name ?? ''
  const tip = (o: Own, i: number) =>
    o === 'clash' ? `Ch ${i + 1}: overlap` : o ? `Ch ${i + 1} · ${nameOf(o.id)} · ${o.attr ? ATTR_LABEL[o.attr] ?? o.attr : 'unused'} = ${frame[i] ?? 0}` : `Ch ${i + 1}: free`

  // ausgewählte Fixtures lückenlos ans Ende dieses Universums legen
  function moveHere() {
    let end = 1
    for (const f of show.fixtures) if (f.universe === uni && !live.selection.includes(f.id)) end = Math.max(end, f.address + width(f.ty))
    const acts = []
    for (const id of live.selection) {
      const f = show.fixtures.find((x) => x.id === id)
      if (!f) continue
      if (end + width(f.ty) - 1 > 512) { S.say(`Universe ${uni} is full`, true); break }
      acts.push({ SetPatch: { id, universe: uni, address: end } })
      end += width(f.ty)
    }
    if (acts.length) { S.act(acts); S.say(`Moved ${acts.length} fixture(s) to universe ${uni}`) }
  }
  const selectUni = () => fixtures.length && S.act([{ Select: fixtures[0].id }, ...fixtures.slice(1).map((f) => ({ Toggle: f.id }))])
  let hov = $state(-1)
  const pick = (o: Own) => { if (o && o !== 'clash') S.act({ Select: o.id }) }
  const hue = (id: number) => (id * 47) % 360
</script>

<div class="uni" style="--w:{S.layout.uniW}px">
  <Splitter key="uniW" />
  <section class="win list">
    <header><div class="title">Universes</div></header>
    <div class="body" style="padding:0">
      {#each unis as u}
        <button class="u" class:on={u === uni} onclick={() => (sel = u)}>
          <span>Universe {u}</span>
          <span class="dim mono">{used(u)} / 512</span>
          <i style="width:{(used(u) / 512) * 100}%"></i>
        </button>
      {/each}
    </div>
  </section>

  <section class="win mon">
    <header>
      <div class="title">Universe {uni} <Info text="Live DMX values of this universe. Each patched fixture has its own colour, brighter means a higher value. Click a channel to select its fixture. Red outlines are overlapping addresses." /></div>
    </header>
    <div class="hint mono">{hov >= 0 ? tip(own[hov], hov) : 'Hover a channel for details'}</div>
    <div class="body">
      <div class="dmx">
        {#each own as o, i}
          {@const v = frame[i] ?? 0}
          <button class="c" class:clash={o === 'clash'} class:free={!o} onmouseenter={() => (hov = i)} onmouseleave={() => (hov = -1)} onclick={() => pick(o)}
            style={o && o !== 'clash' ? `background:hsl(${hue(o.id)} 55% ${14 + (v / 255) * 38}%)` : ''}>
            <span class="a">{i + 1}</span><span class="v mono">{o ? v : ''}</span>
          </button>
        {/each}
      </div>
    </div>
  </section>

  <section class="win side">
    <header>
      <Tool icon={Pointer} label="Select universe" disabled={!fixtures.length} onclick={selectUni} title="Select all fixtures of universe {uni}" />
      <Tool icon={Wand} label="Auto Address" onclick={() => S.act({ AutoAddress: uni })} title="Re-address universe {uni} without gaps" />
      <Tool icon={MoveRight} label="Move selection here" disabled={!live.selection.length} onclick={moveHere} title="Append the selected fixtures to the end of universe {uni}" />
    </header>
    <div class="body col">
      <div class="stat"><span>Used</span><b class="mono">{usedNow} / 512</b></div>
      <div class="stat"><span>Fixtures</span><b class="mono">{fixtures.length}</b></div>
      <div class="stat" class:bad={clashes}><span>Overlaps</span><b class="mono">{clashes}</b></div>
      <div class="dim">Free ranges</div>
      <div class="mono free">{free.length ? free.map(([a, b]) => (a === b ? a : `${a}–${b}`)).join(', ') : 'none'}</div>
      <div class="dim">Fixtures</div>
      <table class="table">
        <tbody>
          {#each fixtures as f}
            <tr class:sel={live.selection.includes(f.id)} onclick={() => S.act({ Select: f.id })}>
              <td class="num mono">{f.id}</td><td>{f.name}</td><td class="mono">{f.address}–{f.address + width(f.ty) - 1}</td>
            </tr>
          {/each}
          {#if !fixtures.length}<tr><td colspan="3" class="dim">Empty universe.</td></tr>{/if}
        </tbody>
      </table>
    </div>
  </section>
</div>

<style>
  .uni { display: grid; grid-template-columns: 200px minmax(0, 1fr) var(--w); gap: 2px; padding: 2px; height: 100%; position: relative; }
  .u { position: relative; display: flex; justify-content: space-between; width: 100%; padding: 9px 10px 11px; border-bottom: 1px solid #222; text-align: left; }
  .u.on { background: #1d2a44; }
  .u i { position: absolute; left: 0; bottom: 0; height: 3px; background: #4a7fd0; }
  .dmx { display: grid; grid-template-columns: repeat(auto-fill, minmax(44px, 1fr)); gap: 2px; }
  .c { position: relative; height: 38px; background: #141414; border: 1px solid #2a2a2a; padding: 0; }
  .c.free { opacity: 0.55; cursor: default; }
  .c.clash { border-color: var(--red-hi); background: #4a1a1a; }
  .a { position: absolute; left: 3px; top: 1px; font-size: 9px; color: #bbb; }
  .v { position: absolute; right: 3px; bottom: 2px; font-size: 12px; font-weight: 700; }
  .hint { padding: 4px 10px; font-size: 12px; color: var(--yellow); border-bottom: 1px solid #222; height: 24px; flex: none; }
  .col { display: flex; flex-direction: column; gap: 6px; padding: 10px; }
  .stat { display: flex; justify-content: space-between; }
  .stat.bad b { color: var(--red-hi); }
  .free { font-size: 12px; word-break: break-word; }
</style>
