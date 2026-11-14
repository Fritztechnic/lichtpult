<script lang="ts">
  // Status: Überblick über Show, Patch, Ausgabe und laufende Cuelists.
  import { S } from '../store.svelte'

  const show = $derived(S.show!)
  const live = $derived(S.live!)
  const unis = $derived(Object.keys(live.frames).length)
  const channels = $derived(show.fixtures.reduce((n, f) => n + (show.types[f.ty]?.channels.length ?? 0), 0))
  const running = $derived(live.players.map((p, i) => ({ p, i })).filter((x) => x.p.cue != null))
  const fx = $derived(live.fx_running.filter(Boolean).length)
  const flag = (b: boolean) => (b ? 'ON' : 'OFF')
</script>

<div class="st">
  <dl>
    <dt>Show</dt><dd>{show.name}</dd>
    <dt>Fixtures</dt><dd>{show.fixtures.length} <span class="dim">({channels} channels, {unis} universes)</span></dd>
    <dt>Selection</dt><dd>{live.selection.length}</dd>
    <dt>Address overlaps</dt><dd class:bad={live.conflicts.length > 0}>{live.conflicts.length || 'none'}</dd>
    <dt>Grand Master</dt><dd>{Math.round((live.master / 255) * 100)}%</dd>
    <dt>Blackout / Blind / Highlight</dt><dd>{flag(live.blackout)} / {flag(live.blind)} / {flag(live.highlight)}</dd>
    <dt>Programmer</dt><dd>{Object.keys(live.programmer).length} fixtures</dd>
    <dt>Parked</dt><dd>{Object.keys(live.parked).length} fixtures</dd>
    <dt>Effects</dt><dd>{fx} of {show.effects.length} running</dd>
    <dt>Cuelists</dt>
    <dd>{#each running as { p, i }}<div>{show.lists[i].name}: Cue {(p.cue ?? 0) + 1} <span class="dim">({Math.round(p.level * 100)}%)</span></div>{:else}<span class="dim">none running</span>{/each}</dd>
  </dl>
</div>

<style>
  .st { padding: 10px 14px; overflow: auto; height: 100%; }
  dl { display: grid; grid-template-columns: max-content 1fr; gap: 6px 18px; margin: 0; }
  dt { color: var(--dim); }
  dd { margin: 0; font-weight: 700; }
  .bad { color: var(--yellow); }
</style>
