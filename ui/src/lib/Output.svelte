<script lang="ts">
  // Ausgabe: Art-Net/sACN-Einstellungen, MIDI-Bindungen und Live-Vorschau der Universen.
  import { S } from '../store.svelte'
  import { call } from '../api'
  import Splitter from './Splitter.svelte'
  import Info from './Info.svelte'
  import { onMount } from 'svelte'

  const live = $derived(S.live!)
  let cfg = $state<any>(null)
  let midi = $state<any>(null)
  let uni = $state<string>('1')
  const unis = $derived(Object.keys(live.frames).sort((a, b) => +a - +b))
  const frame = $derived(live.frames[uni] ?? live.frames[unis[0]] ?? [])

  async function loadCfg() { cfg = await call('output_get') }
  async function save() { await call('output_set', cfg); S.say('Output settings saved') }
  onMount(() => {
    loadCfg()
    const t = setInterval(async () => (midi = await call('midi_get')), 500)
    return () => clearInterval(t)
  })
  // Lernbare Ziele: [Schlüssel, Name, Argumentart] – 'n' = Nummer ab 1, 'a' = Attribut, '' = ohne Argument
  const targets: [string, string, string][] = [
    ['Go', 'Go', 'n'], ['Back', 'Back', 'n'], ['Release', 'Release', 'n'], ['Level', 'Playback fader', 'n'],
    ['Cue', 'Go to cue', 'n'], ['Group', 'Select group', 'n'], ['Effect', 'Toggle effect', 'n'],
    ['Attr', 'Attribute fader (selection)', 'a'], ['Master', 'Grand master', ''], ['Blackout', 'Blackout', ''],
    ['Blind', 'Blind', ''], ['Highlight', 'Highlight', ''], ['ClearProgrammer', 'Clear programmer', ''],
    ['Undo', 'Undo', ''], ['Redo', 'Redo', ''], ['NextFixture', 'Next fixture', ''], ['PrevFixture', 'Previous fixture', ''],
  ]
  const MIDI_ATTRS = ['Dimmer', 'Red', 'Green', 'Blue', 'White', 'Amber', 'Uv', 'Pan', 'Tilt', 'Zoom', 'Strobe']
  let tkind = $state('Go')
  let tnum = $state(1)
  let tattr = $state('Dimmer')
  const targ = $derived(targets.find((t) => t[0] === tkind)![2])
  const learn = (t: unknown) => call('midi_learn', t)
  const payload = () => (targ === 'n' ? { [tkind]: tnum - 1 } : targ === 'a' ? { Attr: tattr } : tkind)
</script>


<div class="out" style="--w:{S.layout.outW}px">
  <Splitter key="outW" />
  <section class="win mon">
    <header>
      <div class="title">DMX Output <Info text="Live view of what is sent to the lights, 0–255 per channel. Brighter cells are higher values." /></div>
      {#each unis as u}<button class="btn sm" class:on={(live.frames[uni] ? uni : unis[0]) === u} onclick={() => (uni = u)}>Universe {u}</button>{/each}
    </header>
    <div class="body">
      <div class="dmx">
        {#each frame as v, i}
          <div class="c" style="--v:{v / 255}" class:z={v === 0}><span class="a">{i + 1}</span><span class="v mono">{v}</span></div>
        {/each}
      </div>
      {#if !unis.length}<div class="dim" style="padding:12px">No universes active. Patch fixtures first.</div>{/if}
    </div>
  </section>
  <section class="win cfg">
    <header><div class="title">Network</div></header>
    <div class="body col">
      {#if cfg}
        <div class="row"><span>Output</span><button class="switch" class:on={cfg.enabled} onclick={() => { cfg.enabled = !cfg.enabled; save() }}>{cfg.enabled ? 'ON' : 'OFF'}</button></div>
        <label class="f">Protocol<select bind:value={cfg.protocol} onchange={save}><option value="ArtNet">Art-Net</option><option value="Sacn">sACN (E1.31)</option><option value="Both">Art-Net + sACN</option></select></label>
        <label class="f">Target IP (empty = broadcast / multicast)<input bind:value={cfg.target} onchange={save} placeholder="e.g. 192.168.1.50" /></label>
        <label class="f">sACN priority<input type="number" min="0" max="200" bind:value={cfg.sacn_priority} onchange={save} /></label>
      {/if}
    </div>
    <header><div class="title">MIDI <Info text="Pick a target, press Learn, then press a key or move a fader on your MIDI device. Notes and CC buttons trigger on press; CC faders are scaled 0–255. Devices are connected automatically." /></div></header>
    <div class="body col">
      {#if midi}
        <div class="dim">{midi.ports.length ? 'Devices: ' + midi.ports.join(', ') : 'No MIDI device found.'}</div>
        <div class="row">
          <select bind:value={tkind}>{#each targets as [k, n]}<option value={k}>{n}</option>{/each}</select>
          {#if targ === 'n'}<input type="number" min="1" style="width:60px" bind:value={tnum} />{/if}
          {#if targ === 'a'}<select bind:value={tattr}>{#each MIDI_ATTRS as a}<option>{a}</option>{/each}</select>{/if}
          <button class="btn" class:on={!!midi.learn} onclick={() => learn(midi.learn ? null : payload())}>{midi.learn ? 'Learning…' : 'Learn'}</button>
        </div>
        {#if midi.learn}<div class="dim">Waiting for input → {midi.learn}</div>{/if}
        <div class="dim">Last input: <span class="mono">{midi.last ?? '–'}</span></div>
        {#each midi.binds as b, i}
          <div class="row"><span class="mono">{b.src}</span> → <span>{b.target}</span><span class="spacer"></span><button class="btn sm" onclick={() => call('midi_unbind', i)}>Remove</button></div>
        {/each}
        {#if !midi.binds.length}<div class="dim">No bindings yet.</div>{/if}
      {/if}
    </div>
  </section>
</div>

<style>
  .out { display: grid; grid-template-columns: minmax(0, 1fr) var(--w); position: relative; gap: 2px; padding: 2px; height: 100%; }
  .col { display: flex; flex-direction: column; gap: 8px; padding: 10px; flex: none; }
  .dmx { display: grid; grid-template-columns: repeat(auto-fill, minmax(44px, 1fr)); gap: 2px; }
  .c { position: relative; height: 38px; background: color-mix(in srgb, #c03030 calc(var(--v) * 100%), #141414); border: 1px solid #2a2a2a; }
  .c.z .v { color: #555; }
  .a { position: absolute; left: 3px; top: 1px; font-size: 9px; color: #999; }
  .v { position: absolute; right: 3px; bottom: 2px; font-size: 12px; font-weight: 700; }
</style>
