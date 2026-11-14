<script lang="ts">
  import { S } from '../store.svelte'
  import Fader from './Fader.svelte'
  import Tool from './Tool.svelte'
  import Info from './Info.svelte'
  import { ATTR_COLOR, ATTR_LABEL, KIND_OF, SHAPES, TARGETS, type Kind } from './types'
  import Layers from '@lucide/svelte/icons/layers'
  import type { MenuItem } from '../store.svelte'
  import Lightbulb from '@lucide/svelte/icons/lightbulb'
  import Save from '@lucide/svelte/icons/save'
  import RefreshCw from '@lucide/svelte/icons/refresh-cw'
  import Pencil from '@lucide/svelte/icons/pencil'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import MousePointer2 from '@lucide/svelte/icons/mouse-pointer-2'
  import Eraser from '@lucide/svelte/icons/eraser'
  import ArrowLeftRight from '@lucide/svelte/icons/arrow-left-right'
  import ListFilter from '@lucide/svelte/icons/list-filter'
  import ChevronRight from '@lucide/svelte/icons/chevron-right'
  import ChevronLeft from '@lucide/svelte/icons/chevron-left'
  import SquareStack from '@lucide/svelte/icons/square-stack'
  import ParkingSquare from '@lucide/svelte/icons/square-parking'
  import EyeOff from '@lucide/svelte/icons/eye-off'
  import Fan from '@lucide/svelte/icons/fan'
  import Play from '@lucide/svelte/icons/play'
  import Square from '@lucide/svelte/icons/square'
  import Plus from '@lucide/svelte/icons/plus'
  import Clapperboard from '@lucide/svelte/icons/clapperboard'
  import Film from '@lucide/svelte/icons/film'

  const live = $derived(S.live!)
  const show = $derived(S.show!)
  const sel = $derived(new Set(live.selection))
  const liveFx = $derived(new Map(live.fixtures.map((f) => [f.id, f])))

  let fcTab = $state<'groups' | 'fixtures' | 'selected'>('fixtures')
  let mode = $state<'apply' | 'update' | 'rename' | 'delete'>('apply')
  let parTab = $state<'Intensity' | 'Position' | 'Color' | 'Beam' | 'FX'>('Intensity')
  let focusAttr = $state('Dimmer')
  let fxSel = $state(0)

  const TC: Record<string, string> = { Intensity: '#e03a38', Position: '#b060e0', Color: '#3fbf55', Beam: '#3a8ee0', FX: '#e0a030' }
  const GC = ['#3a8ee0', '#3fbf55', '#e0a030', '#b060e0', '#e03a38', '#40c8c8']
  const fill = (n: number, c: number) => Math.max(c * 6, Math.ceil(n / c) * c) - n
  const pct = (v: number) => Math.round((v / 255) * 100) + '%'

  // plain press selects only this fixture, dragging over more pads adds them; Ctrl toggles, Shift selects a range
  let dragging = false
  function padDown(id: number, e: PointerEvent) {
    if (e.button !== 0) return
    if (e.shiftKey && live.last_selected != null) S.act({ SelectRange: [live.last_selected, id] })
    else if (e.ctrlKey || e.metaKey) S.act({ Toggle: id })
    else { S.act({ Select: id }); dragging = true }
  }
  function padEnter(id: number, e: PointerEvent) {
    if (dragging && e.buttons & 1 && !sel.has(id)) S.act({ Toggle: id })
  }
  const idsOf = (ids: number[]) => ids.filter((id) => show.fixtures.some((f) => f.id === id))
  function groupPick(i: number, e: MouseEvent) {
    const ids = idsOf(show.groups[i].ids)
    if (e.ctrlKey || e.metaKey) S.act(ids.filter((id) => !sel.has(id)).map((id) => ({ Toggle: id })))
    else if (e.shiftKey) S.act(ids.filter((id) => sel.has(id)).map((id) => ({ Toggle: id })))
    else S.act({ SelectGroup: i })
  }
  const groupActive = (i: number) => { const g = idsOf(show.groups[i].ids); return g.length > 0 && g.length === live.selection.length && g.every((id) => sel.has(id)) }
  const groupsOf = (id: number) => show.groups.map((g, i) => ({ g, i })).filter((x) => x.g.ids.includes(id))

  let typeFilter = $state(-1)
  function sameType() {
    const ref = show.fixtures.find((f) => f.id === (live.last_selected ?? live.selection[0]))
    if (!ref) return
    const ids = show.fixtures.filter((f) => f.ty === ref.ty).map((f) => f.id)
    S.act([{ Select: ids[0] }, ...ids.slice(1).map((id) => ({ Toggle: id }))])
  }
  function addToGroup(i: number) {
    const keep = [...live.selection]
    const old = show.groups[i].ids
    if (!keep.length) return
    S.act([{ SelectGroup: i }, ...keep.filter((id) => !old.includes(id)).map((id) => ({ Toggle: id })), { UpdateGroup: i }, { Select: keep[0] }, ...keep.slice(1).map((id) => ({ Toggle: id }))])
  }

  async function groupClick(i: number) {
    if (mode === 'apply') S.act({ SelectGroup: i })
    else if (mode === 'update') S.act({ UpdateGroup: i })
    else if (mode === 'delete') S.act({ DeleteGroup: i })
    else if (mode === 'rename') {
      const n = await S.ask('Rename group', show.groups[i].name)
      if (n) S.act({ RenameGroup: [i, n] })
    }
  }
  async function groupRename(i: number) {
    const n = await S.ask('Rename group', show.groups[i].name)
    if (n) S.act({ RenameGroup: [i, n] })
  }
  function setMode(m: typeof mode) {
    mode = mode === m ? 'apply' : m
  }
  async function recGroup() {
    if (!live.selection.length) return
    const n = await S.ask('Name of the new group', `Group ${show.groups.length + 1}`)
    if (n === null) return
    S.act(['RecordGroup', { RenameGroup: [show.groups.length, n || `Group ${show.groups.length + 1}`] }])
  }

  async function every() {
    const n = await S.ask('Select every n-th fixture of the selection', '2')
    if (n && +n > 0) S.act({ SelectEvery: [+n, 0] })
  }

  const fcIds = $derived(
    (fcTab === 'selected' ? show.fixtures.filter((f) => sel.has(f.id)) : show.fixtures).filter((f) => typeFilter < 0 || f.ty === typeFilter),
  )

  // attributes available for the selected fixtures
  const attrs = $derived.by(() => {
    const out = new Set<string>()
    for (const f of show.fixtures) {
      if (!sel.has(f.id)) continue
      for (const a of show.types[f.ty]?.channels ?? []) if (a && !a.endsWith('Fine')) out.add(a)
    }
    return [...out]
  })
  const attrs2 = $derived.by(() => {
    const o = [...attrs]
    if (!o.includes('Dimmer') && show.fixtures.some((f) => sel.has(f.id) && ['Red', 'Green', 'Blue', 'White'].some((c) => show.types[f.ty]?.channels.includes(c)))) o.unshift('Dimmer')
    return o
  })
  const tabAttrs = $derived(attrs2.filter((a) => KIND_OF[a] === parTab))
  const refId = $derived(live.last_selected ?? live.selection[0] ?? null)
  const progVal = (a: string) => (refId != null ? live.programmer[refId]?.[a] : undefined)
  const outVal = (a: string) => progVal(a) ?? 0
  const set = (a: string, v: number) => {
    focusAttr = a
    S.act({ SetAttr: [a, v] })
  }

  const fxCount = $derived(show.effects.length)
  const eff = $derived(show.effects[fxSel] ?? null)
  const tabCount = (k: string) => attrs2.filter((a) => KIND_OF[a] === k).length

  function hex(a: string) {
    const to = (x: number) => Math.round(x).toString(16).padStart(2, '0')
    return '#' + to(outVal('Red')) + to(outVal('Green')) + to(outVal('Blue'))
  }
  function pickColor(h: string) {
    const v = [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16))
    S.act([{ SetAttr: ['Red', v[0]] }, { SetAttr: ['Green', v[1]] }, { SetAttr: ['Blue', v[2]] }])
  }
  const swatches = ['#ff0000', '#ff8000', '#ffd000', '#00ff00', '#00ffd0', '#0000ff', '#8000ff', '#ff00c0', '#ffffff']

  let xy = $state<HTMLDivElement>()!
  function xyMove(e: PointerEvent) {
    if (!(e.buttons & 1)) return
    const r = xy.getBoundingClientRect()
    const x = Math.min(1, Math.max(0, (e.clientX - r.left) / r.width))
    const y = Math.min(1, Math.max(0, (e.clientY - r.top) / r.height))
    S.act([{ SetAttr: ['Pan', Math.round(x * 255)] }, { SetAttr: ['Tilt', Math.round(y * 255)] }])
  }

  async function fan() {
    const t = await S.ask(`Fan ${ATTR_LABEL[focusAttr] ?? focusAttr}: "from-to" in %`, '0-100')
    const m = t?.match(/(\d+)\s*-\s*(\d+)/)
    if (m) S.act({ Fan: [focusAttr, Math.round(+m[1] * 2.55), Math.round(+m[2] * 2.55), false] })
  }

  function patchFx(p: Partial<NonNullable<typeof eff>>) {
    if (!eff) return
    S.act({ SetEffect: [fxSel, { ...eff, ...p }] })
  }

  const fkeys: [string, string, () => unknown][] = [
    ['F1', 'Clear', () => S.act('ClearProgrammer')],
    ['F2', 'Blind', () => S.act({ SetBlind: !live.blind })],
    ['F3', 'Highlight', () => S.act({ Highlight: !live.highlight })],
    ['F4', 'Park', () => S.act({ Park: focusAttr })],
    ['F5', 'Unpark', () => S.act('Unpark')],
    ['F6', 'Fan', () => fan()],
    ['F7', 'Record Cue', () => S.act('RecordCue')],
    ['F8', 'Store Scene', () => S.act('StoreScene')],
    ['F9', 'Undo', () => S.act('Undo')],
    ['F10', 'Redo', () => S.act('Redo')],
    ['F11', 'Blackout', () => S.act({ Blackout: !live.blackout })],
    ['F12', 'Stop FX', () => S.act('StopEffects')],
  ]
  function fkey(e: KeyboardEvent) {
    const i = /^F(\d+)$/.exec(e.key)
    if (i && +i[1] >= 1 && +i[1] <= 12 && !e.ctrlKey && !e.altKey) { fkeys[+i[1] - 1][2](); e.preventDefault() }
  }
  const progRows = $derived(Object.keys(live.programmer).map(Number))
  const progCols = $derived.by(() => {
    const s = new Set<string>()
    for (const v of Object.values(live.programmer)) for (const a of Object.keys(v)) s.add(a)
    return [...s].sort((a, b) => Object.keys(ATTR_LABEL).indexOf(a) - Object.keys(ATTR_LABEL).indexOf(b))
  })
  const fxName = (id: number) => show.fixtures.find((f) => f.id === id)?.name ?? '#' + id
</script>

<svelte:window onkeydown={fkey} onpointerup={() => (dragging = false)} />
<div class="compose">
  <section class="win fc">
    <header>
      <Tool icon={MousePointer2} label="All" acc="var(--green)" onclick={() => S.act('SelectAll')} />
      <Tool icon={Eraser} label="None" acc="var(--red-hi)" onclick={() => S.act('ClearSelection')} />
      <Tool icon={ArrowLeftRight} label="Invert" acc="var(--blue)" onclick={() => S.act('InvertSelection')} />
      <Tool icon={Lightbulb} label="Highlight" acc="var(--yellow)" on={live.highlight} onclick={() => S.act({ Highlight: !live.highlight })} />
      <Tool icon={ListFilter} label="Every" acc="var(--yellow)" onclick={every} />
      <Tool icon={Layers} label="Same type" acc="var(--purple)" disabled={!live.selection.length} onclick={sameType} title="Select all fixtures of the same type as the last selected" />
      <Tool icon={ChevronLeft} label="Last" acc="var(--blue)" onclick={() => S.act({ SelectStep: -1 })} />
      <Tool icon={ChevronRight} label="Next" acc="var(--green)" onclick={() => S.act({ SelectStep: 1 })} />
      <div class="title">Fixture Center <Info text="Click a fixture to select it, drag across pads to add more, Ctrl+click toggles, Shift+click selects a range. Groups store a selection: click selects, Ctrl+click adds, Shift+click removes its fixtures. The red pad is the last selected fixture." /></div>
      {#if fcTab === 'groups'}
        <Tool icon={Save} label="Record" acc="var(--red-hi)" disabled={!live.selection.length} onclick={recGroup} title="Record the selection as a new group" />
        <Tool icon={RefreshCw} label="Update" acc="var(--yellow)" on={mode === 'update'} onclick={() => setMode('update')} />
        <Tool icon={Pencil} label="Rename" acc="var(--blue)" on={mode === 'rename'} onclick={() => setMode('rename')} />
        <Tool icon={Trash2} label="Delete" acc="var(--red-hi)" on={mode === 'delete'} onclick={() => setMode('delete')} />
      {/if}
    </header>
    <div class="selbar"><span class="chip" style="background:var(--green)">{live.selection.length} selected</span><span class="dim">of {show.fixtures.length}</span><span class="spacer"></span>{#if show.types.length > 1 && fcTab !== 'groups'}<select bind:value={typeFilter}><option value={-1}>All types</option>{#each show.types as t, ti}<option value={ti}>{t.name}</option>{/each}</select>{/if}</div>
    <div class="body">
      {#if fcTab === 'groups'}
        <div class="padgrid" style="--cols:6">
          {#each show.groups as g, i}
            <button class="pad" class:sel={groupActive(i)} onclick={(e) => (mode === 'apply' ? groupPick(i, e) : groupClick(i))} oncontextmenu={(e) => S.menu(e, [{ label: 'Select', run: () => S.act({ SelectGroup: i }) }, { label: 'Add to selection', run: () => groupPick(i, new MouseEvent('click', { ctrlKey: true })) }, { label: 'Remove from selection', run: () => groupPick(i, new MouseEvent('click', { shiftKey: true })) }, { label: 'Add selection to group', disabled: !live.selection.length, run: () => addToGroup(i) }, { label: 'Edit…', run: () => S.openWin('group', i) }, { label: 'Update from selection', run: () => S.act({ UpdateGroup: i }) }, { label: 'Rename…', run: () => groupRename(i) }, { sep: true }, { label: 'Delete', danger: true, run: () => S.act({ DeleteGroup: i }) }])}><span class="sw" style="background:{GC[i % GC.length]}"></span><span class="n">{i + 1}</span><span class="nm">{g.name}</span><span class="v">{g.ids.length}</span></button>
          {/each}
          {#each Array(fill(show.groups.length, 6)) as _, k}<div class="cellx" role="presentation" oncontextmenu={(e) => S.menu(e, [{ label: 'Record group from selection', disabled: !live.selection.length, run: recGroup }])}><i>{show.groups.length + k + 1}</i></div>{/each}
        </div>
      {:else}
        <div class="padgrid" style="--cols:6">
          {#each fcIds as f}
            {@const lf = liveFx.get(f.id)}
            <button class="pad" class:sel={sel.has(f.id)} class:last={live.last_selected === f.id} class:conflict={live.conflicts.includes(f.id)}
              onpointerdown={(e) => padDown(f.id, e)} onpointerenter={(e) => padEnter(f.id, e)} oncontextmenu={(e) => S.menu(e, [{ label: 'Select only', run: () => S.act({ Select: f.id }) }, { label: sel.has(f.id) ? 'Remove from selection' : 'Add to selection', run: () => S.act({ Toggle: f.id }) }, { label: 'Select all', run: () => S.act('SelectAll') }, { label: 'Deselect all', run: () => S.act('ClearSelection') }, { sep: true }, { label: 'Edit…', run: () => S.openWin('fixture', f.id) }, { label: 'Rename…', run: async () => { const n = await S.ask('Rename fixture', f.name); if (n) S.act({ RenameFixture: [f.id, n] }) } }, { label: 'Same type', run: sameType }, { sep: true }, ...show.groups.slice(0, 8).map((g, i): MenuItem => ({ label: `Add selection to “${g.name}”`, run: () => addToGroup(i) })), { label: 'Record group from selection', run: recGroup }, { sep: true }, { label: 'Remove fixture', danger: true, run: () => S.act({ RemoveFixture: f.id }) }])}>
              <span class="sw" style="background:rgb({lf?.rgb.join(',') ?? '60,60,60'})"></span>
              <span class="n">{f.id}</span><span class="nm">{f.name}</span>{#each groupsOf(f.id).slice(0, 4) as x, gi}<span class="gd" style="background:{GC[x.i % GC.length]};left:{3 + gi * 7}px"></span>{/each}<span class="v">{Math.round((lf?.level ?? 0) * 100)}%</span>
            </button>
          {/each}
          {#each Array(fill(fcIds.length, 6)) as _, k}<div class="cellx" role="presentation" oncontextmenu={(e) => S.menu(e, [{ label: 'Select all', run: () => S.act('SelectAll') }, { label: 'Deselect all', run: () => S.act('ClearSelection') }, { label: 'Invert selection', run: () => S.act('InvertSelection') }])}><i>{fcIds.length + k + 1}</i></div>{/each}
        </div>
      {/if}
    </div>
    <footer class="tabs">
      <button class="tab" data-k="G" style="--tc:#3a8ee0" class:on={fcTab === 'groups'} onclick={() => (fcTab = 'groups')}>Groups <span class="cnt">{show.groups.length}</span></button>
      <button class="tab" data-k="F" style="--tc:#3fbf55" class:on={fcTab === 'fixtures'} onclick={() => (fcTab = 'fixtures')}>Fixtures <span class="cnt">{show.fixtures.length}</span></button>
      <button class="tab" data-k="S" style="--tc:#e0a030" class:on={fcTab === 'selected'} onclick={() => (fcTab = 'selected')}>Selected <span class="cnt">{live.selection.length}</span></button>
    </footer>
  </section>

  <section class="win prog">
    <header>
      <Tool icon={Eraser} label="Clear" acc="var(--red-hi)" disabled={!progRows.length} onclick={() => S.act('ClearProgrammer')} title="Clear the programmer (Delete)" />
      <Tool icon={EyeOff} label="Blind" acc="var(--yellow)" on={live.blind} onclick={() => S.act({ SetBlind: !live.blind })} title="Blind: programmer is not output (Ctrl+P)" />
      <Tool icon={ParkingSquare} label="Park" acc="var(--blue)" onclick={() => S.act({ Park: focusAttr })} title="Park the last touched attribute" />
      <Tool icon={Fan} label="Fan" acc="var(--purple)" onclick={fan} />
      <div class="title">Programmer {#if live.blind}<span class="chip" style="background:var(--red)">BLIND</span>{/if}</div>
      <Tool icon={Film} label="Record Cue" acc="var(--red-hi)" onclick={() => S.act('RecordCue')} title="Record the programmer as the next cue of the selected cuelist" />
      <Tool icon={Clapperboard} label="Store Scene" acc="var(--green)" onclick={() => S.act('StoreScene')} title="Store the complete look as a scene and play it" />
    </header>
    <div class="body">
      <table class="table">
        <thead><tr><th style="width:44px">ID</th><th>Name</th>{#each progCols as a}<th>{ATTR_LABEL[a] ?? a}</th>{/each}</tr></thead>
        <tbody>
          {#each progRows as id}
            <tr class:sel={sel.has(id)} onclick={(e) => { if ((e.target as HTMLElement).closest('input,button')) return; if (e.shiftKey && live.last_selected != null) S.act({ SelectRange: [live.last_selected, id] }); else if (e.ctrlKey || e.metaKey) S.act({ Toggle: id }); else S.act({ Select: id }) }} oncontextmenu={(e) => S.menu(e, [{ label: 'Select fixture', run: () => S.act({ Select: id }) }, { label: 'Clear programmer', danger: true, run: () => S.act('ClearProgrammer') }])}>
              <td style="background:#16401f" class="mono">{id}</td><td>{fxName(id)}</td>
              {#each progCols as a}<td class="num mono">{live.programmer[id]?.[a] != null ? pct(live.programmer[id][a]) : ''}</td>{/each}
            </tr>
          {/each}
          {#if !progRows.length}<tr><td colspan={2 + progCols.length} class="dim" style="height:40px">Programmer is empty.</td></tr>{/if}
        </tbody>
      </table>
    </div>
    <footer class="fkeys">
      {#each fkeys as [k, l, run]}
        <button class="fk" onclick={run}><b>{k}</b><span>{l}</span></button>
      {/each}
    </footer>
  </section>

  <section class="win par">
    <header>
      <div class="title">Parameters <span class="dim">· {live.selection.length} selected</span>
        <Info text="Wide faders set one attribute on all selected fixtures. Double-click a fader to release that attribute from the programmer. Color has a picker, Position an XY pad, FX holds the effect engine." /></div>
    </header>
    <div class="body pbody">
      {#if parTab === 'FX'}
        <div class="fxl">
          <div class="row">
            <select id="fxtarget" aria-label="Target">{#each TARGETS as [k, n]}<option value={k}>{n}</option>{/each}</select>
            <select id="fxshape" aria-label="Shape">{#each SHAPES as s}<option>{s}</option>{/each}</select>
            <button class="btn" disabled={!live.selection.length}
              onclick={() => S.act({ AddEffect: [(document.getElementById('fxtarget') as HTMLSelectElement).value, (document.getElementById('fxshape') as HTMLSelectElement).value] }).then(() => (fxSel = show.effects.length))}>
              <Plus size={14} /> New FX on selection</button>
            <button class="btn" onclick={() => S.act('StopEffects')}><Square size={14} /> Stop all</button>
          </div>
          <div class="padgrid" style="margin-top:6px">
            {#each show.effects as e, i}
              <button class="pad" class:act={live.fx_running[i]} style={fxSel === i ? 'outline:2px solid var(--orange)' : ''} onclick={() => (fxSel = i)} ondblclick={() => S.openWin('fx', i)} oncontextmenu={(e) => S.menu(e, [{ label: live.fx_running[i] ? 'Stop' : 'Start', run: () => S.act({ ToggleEffect: i }) }, { label: 'Edit…', run: () => S.openWin('fx', i) }, { sep: true }, { label: 'Delete', danger: true, run: () => S.act({ DeleteEffect: i }) }])}>
                <span class="n">{i + 1}</span><span class="nm">{e.name}</span><span class="v">{e.shape}</span></button>
            {/each}
          </div>
          {#if !fxCount}<div class="dim hint">No effects. Select fixtures, choose a target and shape, then create one.</div>{/if}
        </div>
        {#if eff}
          <div class="fxe">
            <div class="fcol"><Fader value={Math.round((eff.rate / 10) * 255)} color="#7a3fb0" label="Speed" unit="raw" onchange={(v) => patchFx({ rate: Math.max(0.05, (v / 255) * 10) })} /><div class="cap2 mono">{eff.rate.toFixed(2)} Hz</div></div>
            <div class="fcol"><Fader value={eff.size} color="#7a3fb0" label="Size" onchange={(v) => patchFx({ size: v })} /></div>
            <div class="fcol"><Fader value={Math.round(eff.spread * 255)} color="#7a3fb0" label="Spread" onchange={(v) => patchFx({ spread: v / 255 })} /></div>
            <div class="fside">
              <select value={eff.shape} onchange={(e) => patchFx({ shape: e.currentTarget.value })}>{#each SHAPES as s}<option>{s}</option>{/each}</select>
              <select value={eff.target} onchange={(e) => patchFx({ target: e.currentTarget.value })}>{#each TARGETS as [k, n]}<option value={k}>{n}</option>{/each}</select>
              <button class="btn" class:on={live.fx_running[fxSel]} onclick={() => S.act({ ToggleEffect: fxSel })}>{#if live.fx_running[fxSel]}<Square size={14} /> Stop{:else}<Play size={14} /> Start{/if}</button>
              <button class="btn" onclick={() => S.act({ DeleteEffect: fxSel }).then(() => (fxSel = 0))}><Trash2 size={14} /> Delete</button>
            </div>
          </div>
        {/if}
      {:else if !tabAttrs.length}
        <div class="dim" style="padding:14px">{live.selection.length ? `The selected fixtures have no ${parTab} parameters.` : 'Select fixtures to edit their parameters.'}</div>
      {:else}
        <div class="faders">
          {#each tabAttrs as a}
            <div class="fcol" class:prog={progVal(a) !== undefined}>
              <Fader value={outVal(a)} color={ATTR_COLOR[a] ?? '#c03030'} label={ATTR_LABEL[a] ?? a} onchange={(v) => set(a, v)} ondblclick={() => S.act({ ReleaseAttr: a })} />
            </div>
          {/each}
          {#if parTab === 'Position' && tabAttrs.includes('Pan') && tabAttrs.includes('Tilt')}
            <div class="xy" bind:this={xy} role="application" onpointerdown={(e) => { xy.setPointerCapture(e.pointerId); xyMove(e) }} onpointermove={xyMove}>
              <div class="dot" style="left:{(outVal('Pan') / 255) * 100}%;top:{(outVal('Tilt') / 255) * 100}%"></div>
            </div>
          {/if}
          {#if parTab === 'Color' && tabAttrs.includes('Red')}
            <div class="cpick">
              <input type="color" value={hex('')} oninput={(e) => pickColor(e.currentTarget.value)} />
              <div class="sws">{#each swatches as c}<button class="sw2" aria-label={c} style="background:{c}" onclick={() => pickColor(c)}></button>{/each}</div>
            </div>
          {/if}
        </div>
      {/if}
    </div>
    <footer class="tabs">
      {#each ['Intensity', 'Position', 'Color', 'Beam'] as k}
        <button class="tab" data-k={k[0]} style="--tc:{TC[k]}" class:on={parTab === k} onclick={() => (parTab = k as typeof parTab)}>{k}<span class="cnt">{tabCount(k)}</span></button>
      {/each}
      <button class="tab" data-k="F" style="--tc:{TC.FX}" class:on={parTab === 'FX'} onclick={() => (parTab = 'FX')}>FX<span class="cnt">{fxCount}</span></button>
    </footer>
  </section>
</div>

<style>
  .compose { display: grid; height: 100%; gap: 2px; grid-template-columns: minmax(520px, 1.15fr) minmax(460px, 1fr); grid-template-rows: minmax(0, 1.15fr) minmax(0, 1fr); grid-template-areas: 'fc par' 'prog par'; padding: 2px; }
  .fc { grid-area: fc; } .prog { grid-area: prog; } .par { grid-area: par; }
  .selbar { display: flex; align-items: center; gap: 8px; padding: 4px 6px; border-bottom: 1px solid var(--line); font-size: 11px; }
  .gd { position: absolute; bottom: 4px; width: 5px; height: 5px; border-radius: 50%; }
  .fkeys { display: grid; grid-template-columns: repeat(12, 1fr); gap: 3px; padding: 3px; }
  .fk { display: flex; flex-direction: column; align-items: center; gap: 1px; padding: 4px 2px; background: var(--btn); border: 1px solid var(--line-hi); font-size: 10px; color: #bbb; }
  .fk b { color: var(--orange); font-size: 10px; } .fk:hover { background: var(--btn-hi); border-color: #888; }
  .pbody { display: flex; gap: 6px; padding: 6px; }
  .faders { display: flex; gap: 6px; width: 100%; align-items: stretch; }
  .fcol { flex: 1; max-width: 120px; min-width: 56px; display: flex; flex-direction: column; }
  .fcol.prog :global(.val) { background: #3a2a00; }
  .cap2 { text-align: center; font-size: 11px; color: var(--dim); padding-top: 3px; }
  .xy { position: relative; width: 220px; align-self: stretch; margin: 0 0 0 8px; background: repeating-linear-gradient(0deg, #0a0a0a 0 24px, #151515 24px 25px), repeating-linear-gradient(90deg, #0a0a0a 0 24px, #151515 24px 25px); border: 1px solid #555; touch-action: none; cursor: crosshair; }
  .dot { position: absolute; width: 16px; height: 16px; margin: -8px; border-radius: 50%; background: var(--red-hi); border: 2px solid #fff; }
  .cpick { display: flex; flex-direction: column; gap: 8px; margin-left: 8px; width: 150px; }
  .cpick input[type='color'] { width: 100%; height: 90px; padding: 0; background: none; }
  .sws { display: grid; grid-template-columns: repeat(3, 1fr); gap: 4px; }
  .sw2 { height: 30px; border: 1px solid #555; }
  .fxl { display: flex; flex-direction: column; width: 38%; min-width: 250px; flex: none; }
  .fxl .row { flex-wrap: wrap; }
  .fxl select { flex: 1; min-width: 100px; }
  .fxe { display: flex; gap: 6px; flex: 1; }
  .fside { display: flex; flex-direction: column; gap: 6px; width: 110px; flex: none; }
  .fxe { min-width: 0; }
  .fxe .fcol { max-width: 84px; }
</style>
