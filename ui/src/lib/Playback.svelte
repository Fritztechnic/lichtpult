<script lang="ts">
  import { S } from '../store.svelte'
  import Tool from './Tool.svelte'
  import Info from './Info.svelte'
  import Fader from './Fader.svelte'
  import Play from '@lucide/svelte/icons/play'
  import SkipBack from '@lucide/svelte/icons/skip-back'
  import Square from '@lucide/svelte/icons/square'
  import Copy from '@lucide/svelte/icons/copy'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import RefreshCw from '@lucide/svelte/icons/refresh-cw'
  import Plus from '@lucide/svelte/icons/plus'
  import Repeat from '@lucide/svelte/icons/repeat'
  import Pencil from '@lucide/svelte/icons/pencil'
  import Film from '@lucide/svelte/icons/film'
  import Clapperboard from '@lucide/svelte/icons/clapperboard'
  import Zap from '@lucide/svelte/icons/zap'
  import GripVertical from '@lucide/svelte/icons/grip-vertical'

  const live = $derived(S.live!)
  const show = $derived(S.show!)
  const l = $derived(live.active)
  const list = $derived(show.lists[l])
  const player = $derived(live.players[l])
  let row = $state<number | null>(null)
  let dragFrom = $state<number | null>(null)
  let dragOver = $state<number | null>(null)

  const num = (v: string, d = 0) => (Number.isFinite(parseFloat(v)) ? Math.max(0, parseFloat(v)) : d)
  async function rename() {
    const n = await S.ask('Rename cuelist', list.name)
    if (n) S.act({ RenameList: [l, n] })
  }
  async function renameCue() {
    if (row == null) return
    const n = await S.ask('Rename cue', list.cues[row].name)
    if (n) S.act({ RenameCue: [row, n] })
  }
  function drop(to: number) {
    if (dragFrom != null && dragFrom !== to) S.act({ MoveCue: [dragFrom, to] })
    dragFrom = dragOver = null
  }
  const tcolor = (c: { chase: boolean }) => (c.chase ? 'var(--blue)' : 'var(--red)')
</script>

<div class="pb">
  <section class="win cl">
    <header>
      <Tool icon={SkipBack} label="Back" onclick={() => S.play('Back', l)} />
      <Tool icon={Play} label="Go" on={player?.cue != null} onclick={() => S.play('Go', l)} />
      <Tool icon={Square} label="Release" onclick={() => S.play('ReleaseList', l)} />
      <div class="title">{list.name} <span class="dim">· {list.cues.length} cues · {player?.cue != null ? `cue ${player.cue + 1}` : 'released'}</span>
        <Info text="Double-click a row to jump to a cue. Drag rows by the handle to reorder. Fade, delay, follow and comment are edited in place. Follow = seconds after the fade until the next cue starts automatically; empty means manual Go. Scenes contain a full look and track nothing from earlier cues." /></div>
      <Tool icon={Film} label="Record Cue" onclick={() => S.act('RecordCue')} />
      <Tool icon={Clapperboard} label="Store Scene" onclick={() => S.act('StoreScene')} />
      <Tool icon={RefreshCw} label="Update" disabled={row == null} onclick={() => row != null && S.act({ UpdateCue: row })} title="Write the programmer into the selected cue" />
      <Tool icon={Copy} label="Duplicate" disabled={row == null} onclick={() => row != null && S.act({ DuplicateCue: row })} />
      <Tool icon={Pencil} label="Rename" disabled={row == null} onclick={renameCue} />
      <Tool icon={Trash2} label="Delete" disabled={row == null} onclick={() => { if (row != null) { S.act({ DeleteCue: row }); row = null } }} />
    </header>
    <div class="fadebar"><div style="width:{(player?.progress ?? 0) * 100}%"></div></div>
    <div class="body" style="padding:0">
      <table class="table">
        <thead><tr><th style="width:26px"></th><th style="width:44px">No</th><th>Name</th><th style="width:80px">Trigger</th><th style="width:70px">Delay</th><th style="width:70px">Fade</th><th style="width:90px">Type</th><th>Comment</th></tr></thead>
        <tbody>
          {#each list.cues as c, i}
            <tr class:sel={row === i} class:cur={player?.cue === i} class:drop={dragOver === i && dragFrom !== i}
              onclick={() => (row = i)} ondblclick={() => S.act({ GotoCue: i })}
              oncontextmenu={(e) => { row = i; S.menu(e, [{ label: 'Go to cue', run: () => S.act({ GotoCue: i }) }, { label: 'Edit…', run: () => S.openWin('cue', i) }, { label: 'Update from programmer', run: () => S.act({ UpdateCue: i }) }, { label: 'Duplicate', run: () => S.act({ DuplicateCue: i }) }, { label: 'Move up', disabled: i === 0, run: () => S.act({ MoveCue: [i, i - 1] }) }, { label: 'Move down', disabled: i === list.cues.length - 1, run: () => S.act({ MoveCue: [i, i + 1] }) }, { sep: true }, { label: 'Delete', danger: true, run: () => { S.act({ DeleteCue: i }); row = null } }]) }}
              ondragover={(e) => { e.preventDefault(); dragOver = i }} ondrop={() => drop(i)}>
              <td draggable="true" ondragstart={(e) => { dragFrom = i; e.dataTransfer?.setData('text/plain', String(i)) }} ondragend={() => (dragFrom = dragOver = null)} style="cursor:grab;color:#777"><GripVertical size={14} /></td>
              <td class="num mono">{i + 1}</td>
              <td>{c.name}</td>
              <td><input value={c.follow ?? ''} placeholder="Go" onchange={(e) => S.act({ SetCueFollow: [i, e.currentTarget.value === '' ? null : num(e.currentTarget.value)] })} /></td>
              <td><input value={c.delay} onchange={(e) => S.act({ SetCueDelay: [i, num(e.currentTarget.value)] })} /></td>
              <td><input value={c.fade} onchange={(e) => S.act({ SetCueFade: [i, num(e.currentTarget.value)] })} /></td>
              <td>{#if c.block}<span class="chip" style="background:var(--purple)">Scene</span>{:else}<span class="chip">Tracking</span>{/if}</td>
              <td><input value={c.comment} onchange={(e) => S.act({ SetCueComment: [i, e.currentTarget.value] })} /></td>
            </tr>
          {/each}
          {#if !list.cues.length}<tr><td colspan="8" class="dim" style="height:60px;text-align:center">Empty cuelist. Build a look in Compose, then Record Cue or Store Scene.</td></tr>{/if}
        </tbody>
      </table>
    </div>
    <footer class="opts">
      <button class="btn sm" class:on={list.looped} onclick={() => S.act({ SetLooped: [l, !list.looped] })}><Repeat size={14} /> Loop</button>
      <button class="btn sm" class:on={list.chase} onclick={() => S.act({ SetChase: [l, !list.chase] })}><Zap size={14} /> Chase</button>
      {#if list.chase}<label class="row dim">BPM <input style="width:64px" value={list.bpm} onchange={(e) => S.act({ SetBpm: [l, num(e.currentTarget.value, 120)] })} /></label>{/if}
      <span class="spacer"></span>
      <label class="row dim">Fade speed <input type="range" min="0.1" max="10" step="0.1" value={live.fade_speed} oninput={(e) => S.act({ FadeSpeed: +e.currentTarget.value })} /> <span class="mono">×{live.fade_speed.toFixed(1)}</span></label>
    </footer>
  </section>

  <section class="win dir">
    <header>
      <Tool icon={Plus} label="Add" onclick={() => S.act('AddList')} />
      <Tool icon={Pencil} label="Rename" onclick={rename} />
      <Tool icon={Trash2} label="Delete" disabled={show.lists.length < 2} onclick={() => S.act({ DeleteList: l })} />
      <div class="title">Cuelist Directory</div>
    </header>
    <div class="body">
      <div class="lists">
        {#each show.lists as c, i}
          <div class="lrow" class:act={i === l} role="presentation" oncontextmenu={(e) => S.menu(e, [{ label: 'Go', run: () => S.play('Go', i) }, { label: 'Back', run: () => S.play('Back', i) }, { label: 'Release', run: () => S.play('ReleaseList', i) }, { label: 'Select', run: () => S.act({ SelectList: i }) }, { sep: true }, { label: 'Edit…', run: () => S.openWin('list', i) }, { label: c.looped ? 'Loop off' : 'Loop on', run: () => S.act({ SetLooped: [i, !c.looped] }) }, { label: c.chase ? 'Chase off' : 'Chase on', run: () => S.act({ SetChase: [i, !c.chase] }) }, { sep: true }, { label: 'Delete', danger: true, disabled: show.lists.length < 2, run: () => S.act({ DeleteList: i }) }])}>
            <button class="pad ph" class:act={i === l} onclick={() => S.act({ SelectList: i })}>
              <span class="sw" style="background:{tcolor(c)}"></span>
              <span class="n">{i + 1}</span><span class="nm">{c.name}</span>
              <span class="v">{live.players[i]?.cue != null ? `${live.players[i].cue! + 1}/${c.cues.length}` : `–/${c.cues.length}`}</span>
            </button>
            <div class="lf"><Fader value={live.players[i]?.level ?? 0} color={tcolor(c)} onchange={(v) => S.act({ SetLevel: [i, v] })} /></div>
            <div class="lb">
              <button class="btn sm" onclick={() => S.play('Go', i)} aria-label="Go"><Play size={14} /></button>
              <button class="btn sm" onclick={() => S.play('ReleaseList', i)} aria-label="Release"><Square size={14} /></button>
            </div>
          </div>
        {/each}
      </div>
    </div>
  </section>
</div>

<style>
  .pb { display: grid; grid-template-columns: 1fr 300px; gap: 2px; padding: 2px; height: 100%; }
  .fadebar { height: 4px; background: #000; } .fadebar div { height: 100%; background: var(--orange); }
  .opts { padding: 5px; gap: 6px; align-items: center; }
  .lists { display: flex; flex-direction: column; gap: 3px; }
  .lrow { display: grid; grid-template-columns: 1fr 44px 36px; gap: 3px; height: 150px; }
  .lrow .pad { height: 100%; }
  .lf { height: 100%; } .lb { display: flex; flex-direction: column; gap: 3px; justify-content: flex-end; }
  .ph .nm { font-size: 14px; }
</style>
