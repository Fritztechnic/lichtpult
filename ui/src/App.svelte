<script lang="ts">
  // Onyx-Show einlesen; das Parsen passiert im Rust-Kern
  async function importOnyx(e: Event) {
    const el = e.currentTarget as HTMLInputElement
    const f = el.files?.[0]
    if (!f) return
    try {
      const bytes = Array.from(new Uint8Array(await f.arrayBuffer()))
      const r = await call<any>('show_import_onyx', { name: f.name, bytes })
      S.settings = false
      S.say(`Imported ${r.fixtures} fixtures, ${r.groups} groups, ${r.lists} cuelists — ${r.notes.join('; ')}`)
    } catch (err) {
      S.say(`${f.name}: ${err}`, true)
    }
    el.value = ''
  }
  // Rahmen der Oberfläche: Kopfzeile, linke Navigation, aktive Ansicht, Bank mit Cuelists,
  // Befehlszeile und Dialoge. Alle Bedienung läuft über den Store (`S`) zur Engine.
  import { onMount } from 'svelte'
  import { S } from './store.svelte'
  import { call } from './api'
  import Compose from './lib/Compose.svelte'
  import Presets from './lib/Presets.svelte'
  import Playback from './lib/Playback.svelte'
  import Plan from './lib/Plan.svelte'
  import Patch from './lib/Patch.svelte'
  import Output from './lib/Output.svelte'
  import Windows from './lib/Windows.svelte'
  import Ctx from './lib/Ctx.svelte'
  import KeyboardIcon from '@lucide/svelte/icons/calculator'
  import Fader from './lib/Fader.svelte'
  import Sliders from '@lucide/svelte/icons/sliders-vertical'
  import ListVideo from '@lucide/svelte/icons/list-video'
  import { startDrag } from './lib/split'
  import AppWindow from '@lucide/svelte/icons/app-window'
  import { WIN_VIEWS } from './lib/types'
  import Palette from '@lucide/svelte/icons/palette'
  import Map from '@lucide/svelte/icons/map'
  import Cable from '@lucide/svelte/icons/cable'
  import Network from '@lucide/svelte/icons/network'
  import Universes from './lib/Universes.svelte'
  import Monitor from '@lucide/svelte/icons/monitor'
  import Settings from '@lucide/svelte/icons/settings'
  import Undo from '@lucide/svelte/icons/undo-2'
  import Redo from '@lucide/svelte/icons/redo-2'
  import Play from '@lucide/svelte/icons/play'
  import SkipBack from '@lucide/svelte/icons/skip-back'
  import Square from '@lucide/svelte/icons/square'
  import Zap from '@lucide/svelte/icons/zap'
  import ChevronUp from '@lucide/svelte/icons/chevron-up'
  import ChevronDown from '@lucide/svelte/icons/chevron-down'
  import Power from '@lucide/svelte/icons/power'
  import Terminal from '@lucide/svelte/icons/terminal'

  onMount(() => S.start())

  let cmdEl = $state<HTMLInputElement>()!
  let hi = $state(-1)
  let shows = $state<string[]>([])
  let newName = $state('')

  const views = [
    ['compose', 'Compose', Sliders],
    ['presets', 'Presets', Palette],
    ['playback', 'Playback', ListVideo],
    ['plan', 'Stage', Map],
    ['patch', 'Patch', Cable],
    ['universes', 'Universes', Network],
    ['output', 'Output', Monitor],
  ] as const

  const live = $derived(S.live)
  const show = $derived(S.show)
  const act = $derived(live?.active ?? 0)
  const nUni = $derived(new Set(show?.fixtures.map((f) => f.universe)).size)

  async function runCmd() {
    if (!S.cmdText.trim()) return
    await S.cmd(S.cmdText.trim())
    S.cmdText = ''
    hi = -1
  }
  function hist(e: KeyboardEvent) {
    if (e.key === 'ArrowUp') { hi = Math.min(S.history.length - 1, hi + 1); S.cmdText = S.history[hi] ?? S.cmdText; e.preventDefault() }
    if (e.key === 'ArrowDown') { hi = Math.max(-1, hi - 1); S.cmdText = hi < 0 ? '' : S.history[hi]; e.preventDefault() }
  }
  function keys(e: KeyboardEvent) {
    if (S.dialog) return
    const tag = (e.target as HTMLElement).tagName
    if (e.ctrlKey && e.key === 'l') { cmdEl.focus(); e.preventDefault(); return }
    if (e.ctrlKey && e.key === 'z') { S.act('Undo'); e.preventDefault(); return }
    if (e.ctrlKey && (e.key === 'y' || (e.shiftKey && e.key === 'Z'))) { S.act('Redo'); e.preventDefault(); return }
    if (e.ctrlKey && e.key === 'p') { S.act({ SetBlind: !live?.blind }); e.preventDefault(); return }
    if (e.ctrlKey && /^[1-7]$/.test(e.key)) { S.view = views[+e.key - 1][0]; e.preventDefault(); return }
    if (tag === 'INPUT' || tag === 'SELECT' || tag === 'TEXTAREA') return
    if (e.ctrlKey && e.key === 'a') { S.act('SelectAll'); e.preventDefault(); return }
    if (e.key === 'Escape') {
      if (S.ctx) S.ctx = null
      else if (S.wins.length) S.closeWin(S.wins.reduce((a, b) => (b.z > a.z ? b : a)).id)
      else S.act('ClearSelection')
      return
    }
    const m: Record<string, () => unknown> = {
      ' ': () => S.play('Go', act), Tab: () => S.play('Back', act), Home: () => S.act({ Snap: act }), End: () => S.play('ReleaseList', act),
      PageDown: () => S.act({ SelectStep: 1 }), PageUp: () => S.act({ SelectStep: -1 }), Delete: () => S.act('ClearProgrammer'),
    }
    if (m[e.key]) { m[e.key](); e.preventDefault() }
  }

  async function openSettings() {
    shows = await call('show_list')
    S.settings = true
  }
  async function saveWs() {
    const n = (await S.ask('Workspace name', ''))?.trim()
    if (n) { S.saveWorkspace(n); S.say(`Workspace “${n}” saved`) }
  }
  async function dlgOk(v: string | null) {
    S.dialog?.resolve(v)
    S.dialog = null
  }
  // Trennlinie über der Bank: Höhe mindestens 90 px, höchstens 60 % des Fensters
  function dragBank(e: PointerEvent) {
    const h0 = S.layout.bankH
    startDrag(e, (_, dy) => (S.layout.bankH = Math.round(Math.min(window.innerHeight * 0.6, Math.max(90, h0 - dy)))), () => S.saveLayout())
  }
  const cycle = (d: number) => { const i = views.findIndex((v) => v[0] === S.view); S.view = views[(i + d + views.length) % views.length][0] }
  const cell = (i: number) => S.page * 10 + i
  const lists = $derived(show?.lists ?? [])
  const tcolor = (i: number) => (lists[i]?.chase ? '#2a64c8' : '#1f7a2e')
</script>

<svelte:window onkeydown={keys} />

{#if !live || !show}
  <div class="boot">Connecting to engine…{#if S.msg}<div class="dim">{S.msg}</div>{/if}</div>
{:else}
  <div class="app" style="--bank:{S.layout.bankH}px">
    <div class="top">
      <button class="logo" onclick={openSettings}>LICHTPULT</button>
      <span class="dim show">{show.name}</span>
      <span class="spacer"></span>
      <span class="stat">{live.players[act] && show.lists[act] ? `Cuelist ${act + 1}: ${show.lists[act].name}` : 'No cuelist selected'}</span>
      <div class="msg" class:err={S.error}>{S.msg}</div>
      <span class="stat">{live.selection.length ? `${live.selection.length} fixture${live.selection.length > 1 ? 's' : ''} selected` : 'No fixture or zone selected'}</span>
      <span class="spacer"></span>
      <button class="btn sm" class:on={S.wins.some((w) => w.kind === 'view')} onclick={(e) => S.menu(e, [...WIN_VIEWS.map((v, i) => ({ label: (S.wins.some((w) => w.id === 'view:' + i) ? '✓ ' : '\u2003') + v.title, run: () => S.toggleWin('view', i) })), { sep: true }, { label: 'Reset layout', run: () => S.resetLayout() }, { label: 'Tile windows', disabled: !S.wins.length, run: () => S.tileWins() }, { label: 'Cascade windows', disabled: !S.wins.length, run: () => S.cascadeWins() }, { label: 'Save workspace…', run: saveWs }, ...Object.keys(S.workspaces).flatMap((n) => [{ label: '\u2003Load “' + n + '”', run: () => S.loadWorkspace(n) }, { label: '\u2003Delete “' + n + '”', danger: true, run: () => S.deleteWorkspace(n) }]), { sep: true }, { label: 'Close all windows', disabled: !S.wins.length, run: () => (S.wins = []) }])} aria-label="Windows"><AppWindow size={14} /> Windows</button>
      <button class="btn sm" class:on={S.wins.some((w) => w.kind === 'keypad')} onclick={() => S.toggleWin('keypad')} aria-label="Keypad"><KeyboardIcon size={14} /> Keypad</button>
      <button class="btn sm" disabled={!live.can_undo} onclick={() => S.act('Undo')} aria-label="Undo"><Undo size={14} /></button>
      <button class="btn sm" disabled={!live.can_redo} onclick={() => S.act('Redo')} aria-label="Redo"><Redo size={14} /></button>
      <span class="gm">
        <span class="dim">Grand Master</span>
        <input type="range" min="0" max="255" value={live.master} oninput={(e) => S.act({ Master: +e.currentTarget.value })} />
        <span class="num mono" style="width:38px">{Math.round((live.master / 255) * 100)}%</span>
      </span>
      <button class="btn sm" class:on={live.blackout} onclick={() => S.act({ Blackout: !live.blackout })}><Power size={14} /> Blackout</button>
    </div>

    <div class="main">
      <nav class="side">
        <button class="chev" aria-label="Previous view" onclick={() => cycle(-1)}><ChevronUp size={16} /></button>
        {#each views as [id, name, Icon]}
          <button class="sbtn" class:on={S.view === id} onclick={() => (S.view = id)}><Icon size={24} strokeWidth={1.6} /><span>{name}</span></button>
        {/each}
        <button class="chev" aria-label="Next view" onclick={() => cycle(1)}><ChevronDown size={16} /></button>
        <span class="spacer"></span>
        <button class="sbtn" onclick={openSettings}><Settings size={24} strokeWidth={1.6} /><span>Settings</span></button>
      </nav>
      <div class="view">
        {#if S.view === 'compose'}<Compose />
        {:else if S.view === 'presets'}<Presets />
        {:else if S.view === 'playback'}<Playback />
        {:else if S.view === 'plan'}<Plan />
        {:else if S.view === 'patch'}<Patch />
        {:else if S.view === 'universes'}<Universes />
        {:else}<Output />{/if}
      </div>
    </div>

    <div class="bank">
      <div class="vsplit" role="separator" aria-orientation="horizontal" title="Ziehen: Bank-Höhe ändern, Doppelklick: zurücksetzen" onpointerdown={dragBank} ondblclick={() => { S.layout.bankH = 190; S.saveLayout() }}></div>
      <div class="maingo">
        <div class="mg">
          <button class="gob" onclick={() => S.play('Back', act)}><SkipBack size={20} /><span>Back</span></button>
          <button class="gob go" onclick={() => S.play('Go', act)}><Play size={26} /><span>GO</span></button>
        </div>
        <div class="mg">
          <button class="gob" onclick={() => S.act({ Snap: act })}><Zap size={18} /><span>Snap</span></button>
          <button class="gob" onclick={() => S.play('ReleaseList', act)}><Square size={18} /><span>Release</span></button>
        </div>
        <div class="pager">
          <button class="btn sm" disabled={S.page <= 0} onclick={() => S.page--} aria-label="Page up"><ChevronUp size={14} /></button>
          <span class="mono">Page {S.page + 1}</span>
          <button class="btn sm" disabled={(S.page + 1) * 10 >= lists.length + 1} onclick={() => S.page++} aria-label="Page down"><ChevronDown size={14} /></button>
        </div>
      </div>
      <div class="cells">
        {#each Array(10) as _, i}
          {@const li = cell(i)}
          {@const c = lists[li]}
          {#if c}
            {@const p = live.players[li]}
            <div class="bcell" class:act={li === act} role="presentation" oncontextmenu={(e) => S.menu(e, [{ label: 'Go', run: () => S.play('Go', li) }, { label: 'Back', run: () => S.play('Back', li) }, { label: 'Release', run: () => S.play('ReleaseList', li) }, { label: 'Select', run: () => S.act({ SelectList: li }) }, { sep: true }, { label: 'Edit…', run: () => S.openWin('list', li) }, { label: 'Rename…', run: async () => { const n = await S.ask('Rename cuelist', c.name); if (n) S.act({ RenameList: [li, n] }) } }, { sep: true }, { label: 'Delete', danger: true, disabled: lists.length < 2, run: () => S.act({ DeleteList: li }) }])}>
              <button class="bhead" onclick={() => S.act({ SelectList: li })} style="--tc:{tcolor(li)}">
                <span class="n">{li + 1}</span><span class="nm">{c.name}</span>
              </button>
              <div class="bf"><Fader value={p?.level ?? 0} color={c.chase ? '#2a64c8' : '#c03030'} onchange={(v) => S.act({ SetLevel: [li, v] })} /></div>
              <div class="bb">
                <button class="btn sm" aria-label="Go" onclick={() => S.play('Go', li)}><Play size={14} /></button>
                <button class="btn sm" aria-label="Release" onclick={() => S.play('ReleaseList', li)}><Square size={14} /></button>
              </div>
              <div class="bfoot mono">#{li + 1} <span class="num">{p?.cue != null ? `${p.cue + 1}/${c.cues.length}` : `–/${c.cues.length}`}</span></div>
            </div>
          {:else}
            <button class="bcell empty" onclick={() => S.act('AddList')} oncontextmenu={(e) => S.menu(e, [{ label: 'Add cuelist', run: () => S.act('AddList') }])} title="Add cuelist"><span class="n">{li + 1}</span></button>
          {/if}
        {/each}
      </div>
    </div>

    <div class="cmdl">
      <Terminal size={16} />
      <span class="mode" class:blind={live.blind}>{live.blind ? 'BLIND' : 'LIVE'}</span>
      <input bind:this={cmdEl} bind:value={S.cmdText} placeholder="Command line, e.g. 1 thru 8 at 50  ·  Ctrl+L" onkeydown={(e) => (e.key === 'Enter' ? runCmd() : hist(e))} />
      {#if live.conflicts.length}<button class="warnb" onclick={() => (S.view = 'universes')} title="Show universes">⚠ {live.conflicts.length} overlapping</button>{/if}
      <span class="dim sel">{show.fixtures.length} fixtures · {nUni} universe{nUni === 1 ? '' : 's'} · {live.selection.length} sel · cue {live.players[act]?.cue != null ? live.players[act].cue! + 1 : '–'}</span>
    </div>
  </div>

  <Windows />
  <Ctx />

  {#if S.dialog}
    {@const d = S.dialog}
    <div class="modal-bg" role="presentation">
      <form class="modal" style="min-width:360px" onsubmit={(e) => { e.preventDefault(); dlgOk((e.currentTarget.elements.namedItem('v') as HTMLInputElement).value) }}>
        <h2>{d.title}</h2>
        <div class="in">
          <!-- svelte-ignore a11y_autofocus -->
          <input name="v" value={d.value} autofocus />
          <div class="row"><span class="spacer"></span><button type="button" class="btn" onclick={() => dlgOk(null)}>Cancel</button><button class="btn on">OK</button></div>
        </div>
      </form>
    </div>
  {/if}

  {#if S.settings}
    <div class="modal-bg" role="presentation" onclick={(e) => e.target === e.currentTarget && (S.settings = false)}>
      <div class="modal">
        <h2>Show &amp; Settings<span class="spacer"></span><button class="btn sm" onclick={() => (S.settings = false)}>Close</button></h2>
        <div class="in">
          <div class="dim">Current show: <b style="color:#fff">{show.name}</b> — changes are autosaved.</div>
          <div class="row">
            <input bind:value={newName} placeholder="Show name" style="flex:1" />
            <button class="btn" disabled={!newName.trim()} onclick={async () => { await call('show_save_as', newName.trim()); S.say('Saved ' + newName); shows = await call('show_list') }}>Save as</button>
            <button class="btn" disabled={!newName.trim()} onclick={async () => { await call('show_new', newName.trim()); S.settings = false }}>New show</button>
          </div>
          <div class="dim">Saved shows</div>
          <div class="padgrid" style="grid-template-columns:repeat(3,1fr)">
            {#each shows as n}<button class="btn" onclick={async () => { await call('show_load', n); S.settings = false; S.say('Loaded ' + n) }}>{n}</button>{/each}
          </div>
          {#if !shows.length}<div class="dim">No saved shows yet.</div>{/if}
          <div class="dim">Import</div>
          <label class="btn" style="cursor:pointer;text-align:center">Import Onyx show…
            <input type="file" accept=".OnyxShow,.zip" hidden onchange={importOnyx} />
          </label>
          <div class="dim" style="font-size:11px">Shortcuts: Space Go · Tab Back · Home Snap · End Release · Delete Clear · PgUp/PgDn prev/next fixture · Ctrl+P Blind · Ctrl+Z/Y Undo/Redo · Ctrl+L command line</div>
        </div>
      </div>
    </div>
  {/if}
{/if}

<style>
  .boot { height: 100%; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px; color: var(--dim); }
  .app { display: grid; grid-template-rows: 32px minmax(0, 1fr) var(--bank, 190px) 28px; height: 100%; }
  .top { display: flex; align-items: center; gap: 10px; padding: 0 8px; background: #050505; border-bottom: 1px solid var(--line); }
  .logo { display: inline-flex; align-items: center; gap: 6px; font-weight: 900; letter-spacing: .2em; font-size: 12px; color: #fff; }
  .show { font-size: 11px; }
  .stat { font-size: 12px; color: #cfcfcf; font-style: italic; }
  .msg { font-size: 11px; color: var(--orange); min-width: 200px; text-align: center; } .msg.err { color: #ff6060; }
  .gm { display: inline-flex; align-items: center; gap: 6px; margin: 0 8px; font-size: 11px; }
  .gm input[type='range'] { width: 110px; accent-color: var(--red-hi); padding: 0; }
  .main { display: grid; grid-template-columns: 70px minmax(0, 1fr); min-height: 0; overflow: hidden; }
  .view { min-width: 0; min-height: 0; overflow: hidden; height: 100%; }
  .side { display: flex; flex-direction: column; gap: 3px; padding: 3px; border-right: 1px solid var(--line); background: #050505; }
  .sbtn { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 4px; height: 58px; background: var(--btn); border: 1px solid var(--line-hi); font-size: 10px; color: #aaa; }
  .sbtn:hover { background: var(--btn-hi); }
  .sbtn.on { background: #2e2e2e; border-color: #fff; color: #fff; }
  .chev { height: 22px; display: flex; align-items: center; justify-content: center; border: 1px solid var(--line); color: #888; } .chev:hover { color: #fff; border-color: #888; }
  .bank { position: relative; display: grid; grid-template-columns: 230px 1fr; border-top: 1px solid var(--line); background: #050505; min-height: 0; }
  .maingo { display: flex; flex-direction: column; gap: 3px; padding: 3px; border-right: 1px solid var(--line); }
  .mg { display: grid; grid-template-columns: 1fr 1.4fr; gap: 3px; flex: 1; }
  .mg:last-of-type { grid-template-columns: 1fr 1fr; }
  .gob { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 4px; background: var(--btn); border: 1px solid var(--line-hi); font-size: 10px; font-weight: 700; }
  .gob:hover { border-color: #888; background: var(--btn-hi); } .gob:active { background: #333; }
  .gob.go { background: var(--btn-green); border-color: #50d068; font-size: 13px; }
  .pager { display: flex; align-items: center; justify-content: space-between; font-size: 11px; }
  .cells { display: grid; grid-template-columns: repeat(10, 1fr); gap: 3px; padding: 3px; min-width: 0; }
  .vsplit { position: absolute; left: 0; right: 0; top: -3px; height: 7px; cursor: ns-resize; z-index: 20; }
  .vsplit:hover { background: #5a7fc866; }
  .bcell { display: grid; grid-template-rows: 34px 1fr 26px 16px; gap: 3px; background: #0d0d0d; border: 1px solid #2a2a2a; padding: 3px; min-width: 0; min-height: 0; }
  .bcell.act { border-color: #fff; }
  .bcell.empty { background: transparent; border-color: #1a1a1a; display: block; position: relative; color: #333; }
  .bcell.empty .n { position: absolute; left: 4px; top: 3px; font-size: 10px; }
  .bhead { position: relative; background: var(--tc); border: 0; overflow: hidden; text-align: left; }
  .bhead .n { position: absolute; left: 4px; top: 2px; font-size: 9px; color: #ffffffb0; }
  .bhead .nm { display: block; padding: 12px 4px 0; font-size: 11px; line-height: 1.1; font-weight: 700; color: #fff; }
  .bf { min-height: 0; } .bf :global(.fader) { min-width: 0; } .bf :global(.track) { min-height: 20px; }
  .bb { display: grid; grid-template-columns: 1fr 1fr; gap: 3px; }
  .bfoot { font-size: 10px; color: var(--dim); text-align: center; }
  .cmdl { display: flex; align-items: center; gap: 8px; padding: 0 8px; background: #050505; border-top: 1px solid var(--line); }
  .cmdl input { flex: 1; background: #000; height: 20px; }
  .warnb { color: var(--yellow); font-size: 11px; padding: 0 8px; }
  .mode { font-size: 10px; font-weight: 700; padding: 1px 8px; background: var(--btn-green); } .mode.blind { background: var(--btn-red); }
  .sel { font-size: 10px; }
</style>
