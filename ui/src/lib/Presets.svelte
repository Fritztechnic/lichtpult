<script lang="ts">
  import { S } from '../store.svelte'
  import Tool from './Tool.svelte'
  import Info from './Info.svelte'
  import type { Kind } from './types'
  import Save from '@lucide/svelte/icons/save'
  import RefreshCw from '@lucide/svelte/icons/refresh-cw'
  import Pencil from '@lucide/svelte/icons/pencil'
  import Trash2 from '@lucide/svelte/icons/trash-2'

  const show = $derived(S.show!)
  type Tab = Kind | 'All'
  let tab = $state<Tab>('Intensity')
  let mode = $state<'apply' | 'update' | 'rename' | 'delete'>('apply')
  const TC: Record<string, string> = { Intensity: '#e03a38', Position: '#b060e0', Color: '#3fbf55', Beam: '#3a8ee0', All: '#e0a030' }
  const tabs: [string, Tab][] = [['I', 'Intensity'], ['P', 'Position'], ['C', 'Color'], ['B', 'Beam'], ['A', 'All']]
  const presets = $derived(show.palettes.map((p, i) => ({ p, i })).filter((x) => tab === 'All' || x.p.kind === tab))
  const fill = (n: number) => Math.max(48, Math.ceil(n / 8) * 8) - n
  const setMode = (m: typeof mode) => (mode = mode === m ? 'apply' : m)
  const swatch = (p: (typeof show.palettes)[number]) => (p.kind === 'Color' ? `rgb(${p.values.Red ?? 0},${p.values.Green ?? 0},${p.values.Blue ?? 0})` : TC[p.kind])

  async function rec() {
    const kind = tab === 'All' ? 'Intensity' : tab
    const n = await S.ask(`Name of the new ${kind} preset`, `${kind} ${show.palettes.filter((p) => p.kind === kind).length + 1}`)
    if (n === null) return
    const idx = show.palettes.length
    S.act([{ RecordPalette: kind }, { RenamePalette: [idx, n] }])
  }
  async function rename(i: number) {
    const n = await S.ask('Rename preset', show.palettes[i].name)
    if (n) S.act({ RenamePalette: [i, n] })
  }
  function click(i: number) {
    if (mode === 'apply') S.act({ ApplyPalette: i })
    else if (mode === 'update') S.act({ UpdatePalette: i })
    else if (mode === 'delete') S.act({ DeletePalette: i })
    else rename(i)
  }
</script>

<div class="presets">
  <section class="win">
    <header>
      <Tool icon={Save} label="Record" acc="var(--red-hi)" onclick={rec} title="Store the programmer values of the tab's type as a new preset" />
      <Tool icon={RefreshCw} label="Update" acc="var(--yellow)" on={mode === 'update'} onclick={() => setMode('update')} />
      <Tool icon={Pencil} label="Rename" acc="var(--blue)" on={mode === 'rename'} onclick={() => setMode('rename')} />
      <Tool icon={Trash2} label="Delete" acc="var(--red-hi)" on={mode === 'delete'} onclick={() => setMode('delete')} />
      <div class="title">Presets <Info text="Record stores the programmer values of the current type as a new preset. Choose Update, Rename or Delete first, then click a preset. A plain click applies it to the selected fixtures. Right-click for more." /></div>
    </header>
    <div class="body">
      <div class="padgrid" style="--cols:8;--rh:72px">
        {#each presets as { p, i }}
          <button class="pad" onclick={() => click(i)} oncontextmenu={(e) => S.menu(e, [{ label: 'Apply', run: () => S.act({ ApplyPalette: i }) }, { label: 'Edit…', run: () => S.openWin('preset', i) }, { label: 'Update from programmer', run: () => S.act({ UpdatePalette: i }) }, { label: 'Rename…', run: () => rename(i) }, { sep: true }, { label: 'Delete', danger: true, run: () => S.act({ DeletePalette: i }) }])}>
            <span class="sw" style="background:{swatch(p)}"></span>
            <span class="n">{i + 1}</span><span class="nm">{p.name}</span>
          </button>
        {/each}
        {#each Array(fill(presets.length)) as _, k}<div class="cellx" role="presentation" oncontextmenu={(e) => S.menu(e, [{ label: 'Record preset', run: rec }])}><i>{presets.length + k + 1}</i></div>{/each}
      </div>
    </div>
    <footer class="tabs">
      {#each tabs as [k, n]}
        <button class="tab" data-k={k} style="--tc:{TC[n]}" class:on={tab === n} onclick={() => (tab = n)}>{n} <span class="cnt">{n === 'All' ? show.palettes.length : show.palettes.filter((p) => p.kind === n).length}</span></button>
      {/each}
    </footer>
  </section>
</div>

<style>
  .presets { height: 100%; padding: 2px; display: grid; }
  .presets section { min-height: 0; }
</style>
