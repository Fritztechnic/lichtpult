<script lang="ts">
  // Cuelist Values: zeigt die Werte einer Cue je Fixture und Attribut.
  // "Tracked" blendet zusätzlich die von früheren Cues übernommenen Werte ein.
  import { S } from '../store.svelte'
  import Tool from './Tool.svelte'
  import Info from './Info.svelte'
  import { ATTR_LABEL, KIND_OF } from './types'
  import ChevronLeft from '@lucide/svelte/icons/chevron-left'
  import ChevronRight from '@lucide/svelte/icons/chevron-right'
  import Route from '@lucide/svelte/icons/route'
  import Crosshair from '@lucide/svelte/icons/crosshair'

  const show = $derived(S.show!)
  const live = $derived(S.live!)
  const list = $derived(show.lists[live.active])
  const player = $derived(live.players[live.active])

  let cue = $state(0)
  let tracked = $state(false)
  let follow = $state(true) // Anzeige folgt der laufenden Cue
  let kind = $state<'All' | 'Intensity' | 'Color' | 'Position' | 'Beam'>('All')

  const idx = $derived(Math.min(follow && player?.cue != null ? player.cue : cue, Math.max(0, list.cues.length - 1)))
  const step = (d: number) => { follow = false; cue = Math.min(list.cues.length - 1, Math.max(0, idx + d)) }

  // Werte der Cue; mit Tracking zusätzlich alle früheren Cues (spätere überschreiben frühere).
  const rows = $derived.by(() => {
    const own = list.cues[idx]?.values ?? {}
    const out = new Map<string, Map<string, { v: number; own: boolean }>>()
    const put = (vals: typeof own, isOwn: boolean) => {
      for (const [id, attrs] of Object.entries(vals)) {
        const m = out.get(id) ?? new Map()
        for (const [a, v] of Object.entries(attrs)) if (kind === 'All' || KIND_OF[a] === kind) m.set(a, { v, own: isOwn })
        if (m.size) out.set(id, m)
      }
    }
    if (tracked) for (let i = 0; i < idx; i++) put(list.cues[i].values, false)
    put(own, true)
    return [...out.entries()].sort((a, b) => +a[0] - +b[0])
  })
  const cols = $derived([...new Set(rows.flatMap(([, m]) => [...m.keys()]))].sort((a, b) => Object.keys(ATTR_LABEL).indexOf(a) - Object.keys(ATTR_LABEL).indexOf(b)))
  const name = (id: string) => show.fixtures.find((f) => f.id === +id)?.name ?? '#' + id
  const pct = (v: number) => Math.round((v / 255) * 100) + '%'
</script>

<div class="cv">
  <section class="win">
    <header>
      <Tool icon={ChevronLeft} label="Prev" disabled={idx <= 0} onclick={() => step(-1)} />
      <Tool icon={ChevronRight} label="Next" disabled={idx >= list.cues.length - 1} onclick={() => step(1)} />
      <Tool icon={Crosshair} label="Follow" acc="var(--green)" on={follow} onclick={() => (follow = !follow)} title="Anzeige folgt der laufenden Cue" />
      <Tool icon={Route} label="Tracked" acc="var(--blue)" on={tracked} onclick={() => (tracked = !tracked)} title="Übernommene Werte früherer Cues mit anzeigen" />
      <div class="title">Cuelist Values <span class="dim">· {list.cues[idx] ? `Cue ${idx + 1} ${list.cues[idx].name}` : 'keine Cue'}</span>
        <Info text="Zeigt die gespeicherten Werte der gewählten Cue. Tracked blendet Werte ein, die von früheren Cues weiterlaufen (gedimmt dargestellt)." /></div>
    </header>
    <div class="body" style="padding:0">
      <table class="table">
        <thead><tr><th style="width:44px">ID</th><th>Name</th>{#each cols as a}<th>{ATTR_LABEL[a] ?? a}</th>{/each}</tr></thead>
        <tbody>
          {#each rows as [id, m]}
            <tr><td class="num mono">{id}</td><td>{name(id)}</td>
              {#each cols as a}{@const c = m.get(a)}<td class="mono" class:trk={c && !c.own}>{c ? pct(c.v) : ''}</td>{/each}
            </tr>
          {/each}
          {#if !rows.length}<tr><td colspan={2 + cols.length} class="dim" style="height:60px;text-align:center">Diese Cue enthält keine Werte{kind !== 'All' ? ` vom Typ ${kind}` : ''}.</td></tr>{/if}
        </tbody>
      </table>
    </div>
    <footer class="tabs">
      {#each ['All', 'Intensity', 'Color', 'Position', 'Beam'] as k}
        <button class="tab" class:on={kind === k} onclick={() => (kind = k as typeof kind)}>{k}</button>
      {/each}
    </footer>
  </section>
</div>

<style>
  .cv { height: 100%; padding: 2px; display: grid; }
  .cv section { min-height: 0; }
  td.trk { color: #777; }
</style>
