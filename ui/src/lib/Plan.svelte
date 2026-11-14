<script lang="ts">
  import { S } from '../store.svelte'
  import Tool from './Tool.svelte'
  import Info from './Info.svelte'
  import { PROP_KINDS, PROP_LABEL, type Prop } from './types'
  import Copy from '@lucide/svelte/icons/copy'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import ArrowUp from '@lucide/svelte/icons/arrow-up'
  import ArrowDown from '@lucide/svelte/icons/arrow-down'
  import Grid3x3 from '@lucide/svelte/icons/grid-3x3'
  import Magnet from '@lucide/svelte/icons/magnet'

  const live = $derived(S.live!)
  const show = $derived(S.show!)
  const sel = $derived(new Set(live.selection))
  const liveFx = $derived(new Map(live.fixtures.map((f) => [f.id, f])))
  let stage: HTMLDivElement
  let grid = $state(true)
  let snap = $state(true)
  let selProp = $state<number | null>(null)
  type Drag = { kind: 'fx'; id: number; x: number; y: number } | { kind: 'prop'; i: number; p: Prop; mode: 'move' | 'size' } | null
  let drag = $state<Drag>(null)
  let moved = false

  const placed = $derived(show.fixtures.filter((f) => f.x !== 0 || f.y !== 0))
  const tray = $derived(show.fixtures.filter((f) => f.x === 0 && f.y === 0))
  const sn = (v: number) => (snap ? Math.round(v / 0.025) * 0.025 : v)
  const rel = (e: PointerEvent) => {
    const r = stage.getBoundingClientRect()
    return { x: (e.clientX - r.left) / r.width, y: (e.clientY - r.top) / r.height }
  }
  const pos = (f: { id: number; x: number; y: number }) => (drag?.kind === 'fx' && drag.id === f.id ? drag : f)

  function fxDown(e: PointerEvent, id: number) {
    e.stopPropagation()
    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
    const f = show.fixtures.find((x) => x.id === id)!
    drag = { kind: 'fx', id, x: f.x, y: f.y }
    moved = false
  }
  function propDown(e: PointerEvent, i: number, mode: 'move' | 'size') {
    e.stopPropagation()
    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
    selProp = i
    drag = { kind: 'prop', i, p: { ...show.props[i] }, mode }
    moved = false
  }
  function move(e: PointerEvent) {
    if (!drag) return
    const { x, y } = rel(e)
    moved = true
    if (drag.kind === 'fx') {
      drag.x = Math.min(0.98, Math.max(0.02, sn(x)))
      drag.y = Math.min(0.98, Math.max(0.02, sn(y)))
    } else if (drag.mode === 'move') {
      drag.p.x = Math.min(1, Math.max(0, sn(x)))
      drag.p.y = Math.min(1, Math.max(0, sn(y)))
    } else {
      drag.p.w = Math.max(0.02, 2 * Math.abs(x - drag.p.x))
      drag.p.h = Math.max(0.02, 2 * Math.abs(y - drag.p.y))
    }
  }
  function up(e: MouseEvent) {
    if (!drag) return
    const d = drag
    drag = null
    if (d.kind === 'fx') {
      if (moved) S.act({ SetPosition: [d.id, d.x, d.y] })
      else S.act(e.shiftKey ? { Toggle: d.id } : { Toggle: d.id })
    } else if (moved) S.act({ SetProp: [d.i, d.p] })
  }
  const prop = (i: number) => (drag?.kind === 'prop' && drag.i === i ? drag.p : show.props[i])
  const cur = $derived(selProp != null ? show.props[selProp] : null)
  function patch(p: Partial<Prop>) {
    if (selProp != null && cur) S.act({ SetProp: [selProp, { ...cur, ...p }] })
  }
  const hex = (c: [number, number, number] | null) => (c ? '#' + c.map((v) => v.toString(16).padStart(2, '0')).join('') : '#3a3a3a')
</script>

<div class="plan">
  <section class="win st">
    <header>
      <Tool icon={Grid3x3} label="Grid" on={grid} onclick={() => (grid = !grid)} />
      <Tool icon={Magnet} label="Snap" on={snap} onclick={() => (snap = !snap)} />
      <div class="title">Stage Plan <Info text="Drag fixtures to place them, click to toggle selection. Props (risers, trusses, speakers, …) can be moved, and resized with the corner handle. Unplaced fixtures wait in the tray on the right." /></div>
    </header>
    <div class="body" style="padding:8px;display:flex;align-items:center;justify-content:center">
      <div class="stage" class:grid bind:this={stage} onpointermove={move} onpointerup={up} onpointerdown={() => (selProp = null)} role="application">
        <div class="edge">STAGE EDGE · AUDIENCE</div>
        {#each show.props as _, i}
          {@const p = prop(i)}
          <div class="prop k-{p.kind}" class:sp={selProp === i}
            style="left:{(p.x - p.w / 2) * 100}%;top:{(p.y - p.h / 2) * 100}%;width:{p.w * 100}%;height:{p.h * 100}%;{p.color ? `background:${hex(p.color)}55;border-color:${hex(p.color)}` : ''}"
            onpointerdown={(e) => propDown(e, i, 'move')} role="button" tabindex="0">
            <span>{p.label || PROP_LABEL[p.kind]}</span>
            {#if selProp === i}<i class="h" onpointerdown={(e) => propDown(e, i, 'size')} role="button" tabindex="0" aria-label="Resize"></i>{/if}
          </div>
        {/each}
        {#each placed as f}
          {@const q = pos(f)}
          {@const lf = liveFx.get(f.id)}
          <div class="fx" class:sel={sel.has(f.id)} class:last={live.last_selected === f.id}
            style="left:{q.x * 100}%;top:{q.y * 100}%;--c:rgb({lf?.rgb.join(',') ?? '70,70,70'});--g:{(lf?.level ?? 0) * 26}px"
            onpointerdown={(e) => fxDown(e, f.id)} role="button" tabindex="0" title={f.name}>
            <b>{f.id}</b>
          </div>
        {/each}
      </div>
    </div>
  </section>
  <section class="win side">
    <header><div class="title">{cur ? 'Prop' : 'Add'}</div></header>
    <div class="body col">
      {#if cur}
        <label class="f">Label<input value={cur.label} placeholder={PROP_LABEL[cur.kind]} onchange={(e) => patch({ label: e.currentTarget.value })} /></label>
        <label class="f">Width<input type="range" min="0.02" max="1" step="0.01" value={cur.w} oninput={(e) => patch({ w: +e.currentTarget.value })} /></label>
        <label class="f">Height<input type="range" min="0.02" max="1" step="0.01" value={cur.h} oninput={(e) => patch({ h: +e.currentTarget.value })} /></label>
        <label class="f">Color<input type="color" value={hex(cur.color)} onchange={(e) => { const h = e.currentTarget.value; patch({ color: [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16)) as [number, number, number] }) }} /></label>
        <div class="row">
          <button class="btn" onclick={() => S.act({ DuplicateProp: selProp })}><Copy size={14} /> Copy</button>
          <button class="btn" onclick={() => { S.act({ DeleteProp: selProp }); selProp = null }}><Trash2 size={14} /> Delete</button>
        </div>
        <div class="row">
          <button class="btn" onclick={() => S.act({ MoveProp: [selProp, Math.min(show.props.length - 1, selProp! + 1)] }).then(() => selProp = Math.min(show.props.length - 1, selProp! + 1))}><ArrowUp size={14} /> Front</button>
          <button class="btn" onclick={() => S.act({ MoveProp: [selProp, Math.max(0, selProp! - 1)] }).then(() => selProp = Math.max(0, selProp! - 1))}><ArrowDown size={14} /> Back</button>
        </div>
      {:else}
        <div class="dim">Stage elements</div>
        <div class="padgrid" style="grid-template-columns:1fr 1fr">
          {#each PROP_KINDS as k}<button class="btn" onclick={() => S.act({ AddProp: k })}>{PROP_LABEL[k]}</button>{/each}
        </div>
      {/if}
      <div class="dim" style="margin-top:8px">Fixture tray ({tray.length})</div>
      <div class="padgrid" style="grid-template-columns:repeat(auto-fill,minmax(54px,1fr))">
        {#each tray as f}
          <button class="btn sm" onclick={() => S.act({ SetPosition: [f.id, 0.5, 0.5] })} title={f.name}>{f.id}</button>
        {/each}
      </div>
      {#if !tray.length}<div class="dim" style="font-size:11px">All fixtures are placed.</div>{/if}
    </div>
  </section>
</div>

<style>
  .plan { display: grid; grid-template-columns: 1fr 250px; gap: 2px; padding: 2px; height: 100%; }
  .col { display: flex; flex-direction: column; gap: 8px; padding: 10px; }
  .stage { position: relative; width: min(100%, calc((100vh - 220px) * 1.7)); aspect-ratio: 17/10; background: #070707; border: 1px solid #555; touch-action: none; }
  .stage.grid { background-image: linear-gradient(#1b1b1b 1px, transparent 1px), linear-gradient(90deg, #1b1b1b 1px, transparent 1px); background-size: 5% 5%; background-color: #070707; }
  .edge { position: absolute; bottom: 4px; left: 0; right: 0; text-align: center; font-size: 10px; letter-spacing: .25em; color: #444; }
  .prop { position: absolute; background: #1c1c1c; border: 1px solid #555; display: flex; align-items: center; justify-content: center; font-size: 11px; color: #999; cursor: move; touch-action: none; overflow: visible; }
  .prop.sp { border-color: var(--orange); outline: 1px dashed var(--orange); }
  .prop.k-Truss { border-style: dashed; background: transparent; }
  .prop.k-Publikum { background: repeating-linear-gradient(45deg, #151515 0 6px, #1d1d1d 6px 12px); }
  .prop.k-Vorhang { background: linear-gradient(90deg, #3a1010, #5a1818 20%, #3a1010 40%, #5a1818 60%, #3a1010); }
  .prop.k-Text { background: transparent; border-color: transparent; font-size: 14px; color: #ddd; }
  .h { position: absolute; right: -6px; bottom: -6px; width: 12px; height: 12px; background: var(--orange); cursor: nwse-resize; }
  .fx { position: absolute; width: 28px; height: 28px; margin: -14px; border-radius: 50%; background: radial-gradient(circle, var(--c) 0 45%, #1a1a1a 46%); border: 2px solid #555; box-shadow: 0 0 var(--g) var(--c); display: flex; align-items: center; justify-content: center; font-size: 10px; cursor: grab; touch-action: none; }
  .fx b { color: #fff; text-shadow: 0 0 3px #000, 0 0 3px #000; font-weight: 700; }
  .fx.sel { border-color: #5ed070; box-shadow: 0 0 0 2px #5ed07088, 0 0 var(--g) var(--c); }
  .fx.last { border-color: var(--red-hi); }
</style>
