<script lang="ts">
  // Patch: gepatchte Fixtures als Tabelle, neue Fixtures aus Bibliothek oder Import.
  // Die Zeilenauswahl ist dieselbe Auswahl wie in Compose.
  import { S } from '../store.svelte'
  import { call } from '../api'
  import Splitter from './Splitter.svelte'
  import Tool from './Tool.svelte'
  import Info from './Info.svelte'
  import Plus from '@lucide/svelte/icons/plus'
  import Upload from '@lucide/svelte/icons/upload'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import Wand from '@lucide/svelte/icons/wand-sparkles'

  const live = $derived(S.live!)
  const show = $derived(S.show!)
  const types = $derived([...show.types.map((t) => ({ t, lib: false })), ...S.library.filter((l) => !show.types.some((t) => t.name === l.name)).map((t) => ({ t, lib: true }))])
  let tyIdx = $state(0)
  let count = $state(1)
  let universe = $state(1)
  let address = $state(1)
  let filter = $state('')
  let file: HTMLInputElement

  const ty = $derived(types[Math.min(tyIdx, types.length - 1)]?.t)
  const inUse = $derived(show.fixtures.some((f) => f.ty === tyIdx))
  async function removeType() {
    const n = show.types[tyIdx].name
    await S.act({ RemoveType: tyIdx })
    tyIdx = 0
    S.say(`Fixture type “${n}” deleted`)
  }
  const width = $derived(ty?.channels.length ?? 1)
  const rows = $derived(show.fixtures.filter((f) => !filter || (f.name + show.types[f.ty]?.name).toLowerCase().includes(filter.toLowerCase())))
  const unis = $derived([...new Set([1, ...show.fixtures.map((f) => f.universe)])].sort((a, b) => a - b))
  const clash = $derived(new Set(live.conflicts))

  // nächste freie Adresse im gewählten Universum
  function nextFree() {
    let end = 1
    for (const f of show.fixtures) if (f.universe === universe) end = Math.max(end, f.address + show.types[f.ty].channels.length)
    address = end
  }
  async function add() {
    if (!ty) return
    try {
      await call('patch_add', { ty, count, universe, address })
      address += count * width
      S.say(`Patched ${count} × ${ty.name}`)
    } catch (e) {
      S.say(String(e), true)
    }
  }
  async function imp(e: Event) {
    const files = (e.currentTarget as HTMLInputElement).files
    if (!files) return
    for (const f of files) {
      try {
        const bytes = Array.from(new Uint8Array(await f.arrayBuffer()))
        const n = await call<number>('import', { name: f.name, bytes })
        S.say(`Imported ${n} fixture type(s) from ${f.name}`)
        tyIdx = types.length
      } catch (err) {
        S.say(`${f.name}: ${err}`, true)
      }
    }
    ;(e.currentTarget as HTMLInputElement).value = ''
    const s = await call<any>('show')
    S.library = s.library
  }
  const cells = (u: number) => {
    const m = new Array<number>(512).fill(-1)
    for (const f of show.fixtures) {
      if (f.universe !== u) continue
      for (let i = 0; i < show.types[f.ty].channels.length; i++) {
        const a = f.address - 1 + i
        if (a < 512) m[a] = m[a] === -1 ? f.id : -2
      }
    }
    return m
  }
  const num = (v: string) => Math.max(1, parseInt(v) || 1)
  function pick(e: MouseEvent, id: number) {
    if ((e.target as HTMLElement).closest('input,button')) return
    if (e.shiftKey && live.last_selected != null) S.act({ SelectRange: [live.last_selected, id] })
    else if (e.ctrlKey || e.metaKey) S.act({ Toggle: id })
    else S.act({ Select: id })
  }
  const targets = (id: number) => (live.selection.includes(id) ? live.selection : [id])
  function removeIds(ids: number[]) { for (const id of ids) S.act({ RemoveFixture: id }) }
</script>

<div class="patch" style="--w:{S.layout.patchW}px">
  <Splitter key="patchW" />
  <section class="win list">
    <header>
      <Tool icon={Wand} label="Auto Address" onclick={() => S.act({ AutoAddress: universe })} title="Re-address universe {universe} without gaps" />
      <div class="title">Patch <Info text="Fixtures are listed by ID. Edit universe and address inline. Auto Address closes gaps in the chosen universe in ID order. Red addresses overlap another fixture." /></div>
      <input placeholder="Filter…" bind:value={filter} style="width:140px" />
    </header>
    <div class="body" style="padding:0">
      <table class="table">
        <thead><tr><th style="width:56px">ID</th><th>Name</th><th>Type</th><th style="width:60px">Ch</th><th style="width:80px">Univ</th><th style="width:80px">Addr</th><th style="width:40px"></th></tr></thead>
        <tbody>
          {#each rows as f}
            <tr class:sel={live.selection.includes(f.id)} onclick={(e) => pick(e, f.id)} oncontextmenu={(e) => { if (!live.selection.includes(f.id)) S.act({ Select: f.id }); const t = targets(f.id); S.menu(e, [{ label: 'Select all', run: () => S.act('SelectAll') }, { label: 'Deselect all', run: () => S.act('ClearSelection') }, { label: 'Edit…', run: () => S.openWin('fixture', f.id) }, { sep: true }, { label: t.length > 1 ? `Remove ${t.length} fixtures` : 'Remove fixture', danger: true, run: () => removeIds(t) }]) }}>
              <td class="num mono">{f.id}</td>
              <td><input value={f.name} onchange={(e) => S.act({ RenameFixture: [f.id, e.currentTarget.value] })} /></td>
              <td class="dim">{show.types[f.ty]?.name}</td>
              <td class="mono">{show.types[f.ty]?.channels.length}</td>
              <td><input class="mono" value={f.universe} onchange={(e) => { const u = num(e.currentTarget.value); for (const id of targets(f.id)) { const g = show.fixtures.find((x) => x.id === id); if (g) S.act({ SetPatch: { id, universe: u, address: g.address } }) } }} /></td>
              <td style={clash.has(f.id) ? 'background:#5a1d1d' : ''}><input class="mono" value={f.address} onchange={(e) => S.act({ SetPatch: { id: f.id, universe: f.universe, address: num(e.currentTarget.value) } })} /></td>
              <td><button class="btn sm" aria-label="Remove" onclick={() => S.act({ RemoveFixture: f.id })}><Trash2 size={13} /></button></td>
            </tr>
          {/each}
          {#if !rows.length}<tr><td colspan="7" class="dim" style="height:60px;text-align:center">Nothing patched yet. Use the panel on the right to add fixtures.</td></tr>{/if}
        </tbody>
      </table>
    </div>
  </section>
  <section class="win add">
    <header><div class="title">Add Fixtures</div></header>
    <div class="body col">
      <label class="f">Fixture type
        <select bind:value={tyIdx}>
          {#each types as x, i}<option value={i}>{x.t.name}{x.lib ? ' (library)' : ''}</option>{/each}
        </select></label>
      <div class="dim" style="font-size:11px">{ty?.channels.length ?? 0} channels: {ty?.channels.map((c) => c ?? '–').join(', ')}</div>
      <div class="row">
        <label class="f">Count<input type="number" min="1" max="512" bind:value={count} /></label>
        <label class="f">Universe<input type="number" min="1" bind:value={universe} onchange={nextFree} /></label>
        <label class="f">Address<input type="number" min="1" max="512" bind:value={address} /></label>
      </div>
      <button class="btn on" onclick={add}><Plus size={14} /> Patch {count} fixture{count > 1 ? 's' : ''}</button>
      <button class="btn" onclick={() => S.openWin('typedit')}><Wand size={14} /> Create fixture type…</button>
      {#if tyIdx < show.types.length}
        <div class="row">
          <button class="btn" onclick={() => S.openWin('typedit', tyIdx + 1)}>Edit type…</button>
          <button class="btn" disabled={inUse} title={inUse ? 'Fixtures of this type are patched' : ''} onclick={removeType}>Delete type</button>
        </div>
      {/if}
      <button class="btn" onclick={() => file.click()}><Upload size={14} /> Import GDTF / OFL…</button>
      <input bind:this={file} type="file" accept=".gdtf,.json" multiple hidden onchange={imp} />
      <div class="dim" style="margin-top:8px">DMX map</div>
      {#each unis as u}
        <div class="um">
          <div class="dim" style="font-size:11px">Universe {u}</div>
          <div class="map">
            {#each cells(u) as c, i}<i title={String(i + 1)} class:used={c >= 0} class:bad={c === -2} style={c >= 0 ? `background:hsl(${(c * 47) % 360} 55% 42%)` : ''}></i>{/each}
          </div>
        </div>
      {/each}
    </div>
  </section>
</div>

<style>
  .patch { display: grid; grid-template-columns: minmax(0, 1fr) var(--w); position: relative; gap: 2px; padding: 2px; height: 100%; }
  .col { display: flex; flex-direction: column; gap: 8px; padding: 10px; }
  .row > * { flex: 1; }
  .map { display: grid; grid-template-columns: repeat(64, 1fr); gap: 1px; }
  .map i { aspect-ratio: 1; background: #1c1c1c; }
  .map i.bad { background: var(--red-hi); }
</style>
