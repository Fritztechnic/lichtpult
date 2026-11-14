<script lang="ts">
  // Frei verschiebbare Fenster (Editoren und Ansichten) innerhalb der App.
  // Verwaltet wird die Liste in `S.wins`; Position und Größe liegen im jeweiligen `Win`.
  import { S, type Win } from '../store.svelte'
  import Keypad from './Keypad.svelte'
  import Compose from './Compose.svelte'
  import Presets from './Presets.svelte'
  import Playback from './Playback.svelte'
  import Plan from './Plan.svelte'
  import Patch from './Patch.svelte'
  import Output from './Output.svelte'
  import Universes from './Universes.svelte'
  import CueValues from './CueValues.svelte'
  import Status from './Status.svelte'
  import FixtureEditor from './FixtureEditor.svelte'
  import X from '@lucide/svelte/icons/x'
  import { SHAPES, TARGETS, WIN_VIEWS } from './types'

  const show = $derived(S.show!)
  const live = $derived(S.live!)

  function drag(e: PointerEvent, w: Win) {
    S.focusWin(w.id)
    const sx = e.clientX - w.x, sy = e.clientY - w.y
    const el = e.currentTarget as HTMLElement
    el.setPointerCapture(e.pointerId)
    const mv = (m: PointerEvent) => {
      w.x = Math.max(0, Math.min(window.innerWidth - 60, m.clientX - sx))
      w.y = Math.max(0, Math.min(window.innerHeight - 30, m.clientY - sy))
    }
    const up = () => { el.removeEventListener('pointermove', mv); el.removeEventListener('pointerup', up) }
    el.addEventListener('pointermove', mv)
    el.addEventListener('pointerup', up)
  }
  // CSS-`resize` meldet die Größe nicht; sie wird beim Loslassen gemerkt
  function remember(e: PointerEvent, w: Win) {
    const el = e.currentTarget as HTMLElement
    if (el.offsetWidth !== w.w) w.w = el.offsetWidth
    if (el.offsetHeight !== w.h) w.h = el.offsetHeight
  }

  const num = (v: string, d = 0) => (Number.isFinite(parseFloat(v)) ? Math.max(0, parseFloat(v)) : d)
  const title = (w: Win) => {
    switch (w.kind) {
      case 'keypad': return 'Keypad'
      case 'typedit': return w.arg ? 'Edit fixture type' : 'New fixture type'
      case 'view': return WIN_VIEWS[w.arg].title
      case 'cue': return `Cue ${w.arg + 1} · ${show.lists[live.active]?.cues[w.arg]?.name ?? ''}`
      case 'list': return `Cuelist ${w.arg + 1} · ${show.lists[w.arg]?.name ?? ''}`
      case 'fx': return `Effect ${w.arg + 1} · ${show.effects[w.arg]?.name ?? ''}`
      case 'group': return `Group ${w.arg + 1} · ${show.groups[w.arg]?.name ?? ''}`
      case 'preset': return `Preset ${w.arg + 1} · ${show.palettes[w.arg]?.name ?? ''}`
      default: return `Fixture ${w.arg} · ${show.fixtures.find((f) => f.id === w.arg)?.name ?? ''}`
    }
  }
  const gone = (w: Win) =>
    (w.kind === 'cue' && !show.lists[live.active]?.cues[w.arg]) || (w.kind === 'list' && !show.lists[w.arg]) || (w.kind === 'fx' && !show.effects[w.arg]) ||
    (w.kind === 'group' && !show.groups[w.arg]) || (w.kind === 'preset' && !show.palettes[w.arg]) || (w.kind === 'fixture' && !show.fixtures.some((f) => f.id === w.arg))
  $effect(() => { for (const w of [...S.wins]) if (w.kind !== 'keypad' && w.kind !== 'view' && w.kind !== 'typedit' && gone(w)) S.closeWin(w.id) })
</script>

{#each S.wins as w (w.id)}
  <div class="fw" style="left:{w.x}px;top:{w.y}px;width:{w.w}px;height:{w.h}px;z-index:{w.z}" role="dialog" tabindex="-1" aria-label={title(w)}
    onpointerdown={() => S.focusWin(w.id)} onpointerup={(e) => remember(e, w)}>
    <div class="fh" role="presentation" onpointerdown={(e) => drag(e, w)}>
      <span>{title(w)}</span>
      <button aria-label="Close" onpointerdown={(e) => e.stopPropagation()} onclick={() => S.closeWin(w.id)}><X size={14} /></button>
    </div>
    <div class="fb" class:flush={w.kind === 'view'}>
      {#if w.kind === 'keypad'}
        <Keypad />
      {:else if w.kind === 'typedit'}
        <FixtureEditor arg={w.arg} />
      {:else if w.kind === 'view'}
        {@const v = WIN_VIEWS[w.arg].id}
        {#if v === 'fc' || v === 'prog' || v === 'par'}<Compose part={v} />
        {:else if v === 'groups'}<Compose part="fc" tab="groups" />
        {:else if v === 'cuevalues'}<CueValues />
        {:else if v === 'status'}<Status />
        {:else if v === 'presets'}<Presets />
        {:else if v === 'playback'}<Playback />
        {:else if v === 'plan'}<Plan />
        {:else if v === 'patch'}<Patch />
        {:else if v === 'universes'}<Universes />
        {:else}<Output />{/if}
      {:else if w.kind === 'cue'}
        {@const c = show.lists[live.active].cues[w.arg]}
        {#if c}
          <label class="f">Name<input value={c.name} onchange={(e) => S.act({ RenameCue: [w.arg, e.currentTarget.value] })} /></label>
          <div class="two">
            <label class="f">Fade (s)<input value={c.fade} onchange={(e) => S.act({ SetCueFade: [w.arg, num(e.currentTarget.value)] })} /></label>
            <label class="f">Delay (s)<input value={c.delay} onchange={(e) => S.act({ SetCueDelay: [w.arg, num(e.currentTarget.value)] })} /></label>
          </div>
          <label class="f">Follow (s, empty = manual Go)<input value={c.follow ?? ''} onchange={(e) => S.act({ SetCueFollow: [w.arg, e.currentTarget.value === '' ? null : num(e.currentTarget.value)] })} /></label>
          <label class="f">Comment<input value={c.comment} onchange={(e) => S.act({ SetCueComment: [w.arg, e.currentTarget.value] })} /></label>
          <div class="dim">{c.block ? 'Scene (full look)' : 'Tracking cue'} · {Object.keys(c.values).length} fixtures</div>
          <div class="row wrap">
            <button class="btn" onclick={() => S.act({ GotoCue: w.arg })}>Go to</button>
            <button class="btn" onclick={() => S.act({ UpdateCue: w.arg })}>Update</button>
            <button class="btn" onclick={() => S.act({ DuplicateCue: w.arg })}>Duplicate</button>
            <button class="btn" onclick={() => S.act({ Snap: live.active })}>Snap</button>
            <button class="btn on" onclick={() => S.act({ DeleteCue: w.arg })}>Delete</button>
          </div>
        {/if}
      {:else if w.kind === 'list'}
        {@const l = show.lists[w.arg]}
        {#if l}
          <label class="f">Name<input value={l.name} onchange={(e) => S.act({ RenameList: [w.arg, e.currentTarget.value] })} /></label>
          <div class="row wrap">
            <button class="btn" class:on={l.looped} onclick={() => S.act({ SetLooped: [w.arg, !l.looped] })}>Loop</button>
            <button class="btn" class:on={l.chase} onclick={() => S.act({ SetChase: [w.arg, !l.chase] })}>Chase</button>
            {#if l.chase}<label class="row dim">BPM <input style="width:64px" value={l.bpm} onchange={(e) => S.act({ SetBpm: [w.arg, num(e.currentTarget.value, 120)] })} /></label>{/if}
          </div>
          <label class="f">Level {Math.round(((live.players[w.arg]?.level ?? 0) / 255) * 100)}%<input type="range" min="0" max="255" value={live.players[w.arg]?.level ?? 0} oninput={(e) => S.act({ SetLevel: [w.arg, +e.currentTarget.value] })} /></label>
          <div class="row wrap">
            <button class="btn green on" onclick={() => S.play('Go', w.arg)}>Go</button>
            <button class="btn" onclick={() => S.play('Back', w.arg)}>Back</button>
            <button class="btn" onclick={() => S.play('ReleaseList', w.arg)}>Release</button>
            <button class="btn on" disabled={show.lists.length < 2} onclick={() => S.act({ DeleteList: w.arg })}>Delete</button>
          </div>
        {/if}
      {:else if w.kind === 'fx'}
        {@const e = show.effects[w.arg]}
        {#if e}
          {@const patch = (p: object) => S.act({ SetEffect: [w.arg, { ...e, ...p }] })}
          <div class="two">
            <label class="f">Shape<select value={e.shape} onchange={(ev) => patch({ shape: ev.currentTarget.value })}>{#each SHAPES as s}<option>{s}</option>{/each}</select></label>
            <label class="f">Target<select value={e.target} onchange={(ev) => patch({ target: ev.currentTarget.value })}>{#each TARGETS as [k, n]}<option value={k}>{n}</option>{/each}</select></label>
          </div>
          <label class="f">Speed {e.rate.toFixed(2)} Hz<input type="range" min="0.05" max="10" step="0.05" value={e.rate} oninput={(ev) => patch({ rate: +ev.currentTarget.value })} /></label>
          <label class="f">Size {Math.round((e.size / 255) * 100)}%<input type="range" min="0" max="255" value={e.size} oninput={(ev) => patch({ size: +ev.currentTarget.value })} /></label>
          <label class="f">Spread {Math.round(e.spread * 100)}%<input type="range" min="0" max="1" step="0.01" value={e.spread} oninput={(ev) => patch({ spread: +ev.currentTarget.value })} /></label>
          <div class="row wrap">
            <button class="btn" class:on={live.fx_running[w.arg]} onclick={() => S.act({ ToggleEffect: w.arg })}>{live.fx_running[w.arg] ? 'Stop' : 'Start'}</button>
            <button class="btn on" onclick={() => S.act({ DeleteEffect: w.arg })}>Delete</button>
            <span class="dim">{e.ids.length} fixtures</span>
          </div>
        {/if}
      {:else if w.kind === 'group'}
        {@const g = show.groups[w.arg]}
        {#if g}
          <label class="f">Name<input value={g.name} onchange={(e) => S.act({ RenameGroup: [w.arg, e.currentTarget.value] })} /></label>
          <div class="dim">{g.ids.length} fixtures: {g.ids.join(', ')}</div>
          <div class="row wrap">
            <button class="btn" onclick={() => S.act({ SelectGroup: w.arg })}>Select</button>
            <button class="btn" onclick={() => S.act({ UpdateGroup: w.arg })}>Update from selection</button>
            <button class="btn on" onclick={() => S.act({ DeleteGroup: w.arg })}>Delete</button>
          </div>
        {/if}
      {:else if w.kind === 'preset'}
        {@const p = show.palettes[w.arg]}
        {#if p}
          <label class="f">Name<input value={p.name} onchange={(e) => S.act({ RenamePalette: [w.arg, e.currentTarget.value] })} /></label>
          <div class="dim">{p.kind} preset</div>
          <table class="table"><tbody>{#each Object.entries(p.values) as [a, v]}<tr><td>{a}</td><td class="num mono">{Math.round((+v / 255) * 100)}%</td></tr>{/each}</tbody></table>
          <div class="row wrap">
            <button class="btn" onclick={() => S.act({ ApplyPalette: w.arg })}>Apply</button>
            <button class="btn" onclick={() => S.act({ UpdatePalette: w.arg })}>Update from programmer</button>
            <button class="btn on" onclick={() => S.act({ DeletePalette: w.arg })}>Delete</button>
          </div>
        {/if}
      {:else}
        {@const f = show.fixtures.find((x) => x.id === w.arg)}
        {#if f}
          <label class="f">Name<input value={f.name} onchange={(e) => S.act({ RenameFixture: [f.id, e.currentTarget.value] })} /></label>
          <div class="dim">{show.types[f.ty]?.name} · {show.types[f.ty]?.channels.length} channels</div>
          <div class="two">
            <label class="f">Universe<input value={f.universe + 1} onchange={(e) => S.act({ SetPatch: { id: f.id, universe: Math.max(0, num(e.currentTarget.value, 1) - 1), address: f.address } })} /></label>
            <label class="f">Address<input value={f.address} onchange={(e) => S.act({ SetPatch: { id: f.id, universe: f.universe, address: Math.max(1, num(e.currentTarget.value, 1)) } })} /></label>
          </div>
          <div class="row wrap">
            <button class="btn" onclick={() => S.act({ Select: f.id })}>Select</button>
            <button class="btn on" onclick={() => S.act({ RemoveFixture: f.id })}>Remove</button>
          </div>
        {/if}
      {/if}
    </div>
  </div>
{/each}

<style>
  .fw { position: fixed; display: flex; flex-direction: column; background: #0d0d0d; border: 1px solid #5a7fc8; box-shadow: 0 8px 30px #000c; resize: both; overflow: hidden; min-width: 240px; min-height: 120px; }
  .fh { display: flex; align-items: center; justify-content: space-between; padding: 4px 6px 4px 10px; background: #050505; border-bottom: 1px solid var(--line); font-size: 11px; font-weight: 700; letter-spacing: .06em; text-transform: uppercase; color: #aaa; cursor: move; touch-action: none; }
  .fh button { display: flex; color: #aaa; padding: 2px; } .fh button:hover { color: #fff; background: var(--red); }
  .fb { flex: 1; min-height: 0; overflow: auto; padding: 8px; display: flex; flex-direction: column; gap: 8px; }
  .fb.flush { padding: 0; gap: 0; overflow: hidden; }
  .fb.flush > :global(*) { flex: 1; min-height: 0; }
  .two { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
  .wrap { flex-wrap: wrap; }
  .fb input[type='range'] { padding: 0; accent-color: var(--red-hi); }
</style>
