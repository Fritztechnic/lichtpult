// Zentraler Zustand der Oberfläche: Show- und Live-Daten (alle 40 ms gepollt), Menüs,
// Fenster und Dialoge. Änderungen an der Show gehen immer über `act` als `Action` an die Engine.
import { call } from './api'
import { WIN_VIEWS, type Live, type Show, type FixtureType } from './lib/types'

export type MenuItem = { label?: string; run?: () => unknown; danger?: boolean; disabled?: boolean; sep?: boolean }
// Breiten der rechten Spalten (px) je Ansicht: Patch, Playback, Output, Stage, Universes
export type Layout = { bankH: number; cw: number; rh: number; patchW: number; pbW: number; outW: number; planW: number; uniW: number }
export const DEFAULT_LAYOUT: Layout = { bankH: 190, cw: 0.53, rh: 0.54, patchW: 360, pbW: 300, outW: 360, planW: 250, uniW: 320 }
export type Win = { id: string; kind: 'keypad' | 'view' | 'typedit' | 'cue' | 'list' | 'fx' | 'group' | 'preset' | 'fixture'; arg: number; x: number; y: number; w: number; h: number; z: number }

class Store {
  live = $state<Live | null>(null)
  show = $state<Show | null>(null)
  library = $state<FixtureType[]>([])
  view = $state('compose')
  page = $state(0)
  msg = $state('')
  error = $state(false)
  history = $state<string[]>([])
  settings = $state(false)
  cmdText = $state('')
  ctx = $state<{ x: number; y: number; items: MenuItem[] } | null>(null)
  // Größe der Bereiche (Bank-Höhe in px, Compose-Spalten-/Zeilenanteil); bleibt im Browser-Speicher erhalten
  layout = $state<Layout>({ ...DEFAULT_LAYOUT, ...JSON.parse(localStorage.getItem('lichtpult.layout') ?? '{}') })
  saveLayout() { localStorage.setItem('lichtpult.layout', JSON.stringify(this.layout)) }
  resetLayout() { this.layout = { ...DEFAULT_LAYOUT }; this.saveLayout() }
  wins = $state<Win[]>([])
  prevSel: number[] = []
  private z = 10
  dialog = $state<{ title: string; value: string; resolve: (v: string | null) => void } | null>(null)
  private busy = false
  private showRev = -1

  // Polling starten; die Show wird nur neu geladen, wenn sich `rev` ändert
  start() {
    const tick = async () => {
      if (!this.busy) {
        this.busy = true
        try {
          const l = await call<Live>('live')
          if (!this.show || l.rev !== this.showRev) {
            const s = await call<{ rev: number; show: Show; library: FixtureType[] }>('show')
            this.showRev = s.rev
            this.show = s.show
            this.library = s.library
          }
          const prev = this.live?.selection
          if (prev?.length && l.selection.join() !== prev.join()) this.prevSel = prev
          this.live = l
        } catch (e) {
          this.say(String(e), true)
        }
        this.busy = false
      }
    }
    tick()
    setInterval(tick, 40)
  }

  say(m: string, err = false) {
    this.msg = m
    this.error = err
    clearTimeout(this.t)
    this.t = window.setTimeout(() => (this.msg = ''), 4000)
  }
  private t = 0

  // `Action` (oder Liste von Actions) an die Engine schicken
  async act(...a: unknown[]) {
    try {
      await call('dispatch', a.length === 1 ? a[0] : a)
    } catch (e) {
      this.say(String(e), true)
    }
  }

  // Befehlszeile ausführen (siehe `cmd.rs`)
  async cmd(line: string) {
    try {
      await call('command', line)
      this.history = [line, ...this.history.filter((h) => h !== line)].slice(0, 50)
      this.say(line)
    } catch (e) {
      this.say(String(e), true)
    }
  }

  // Eingabedialog; liefert `null` bei Abbruch
  ask(title: string, value = ''): Promise<string | null> {
    return new Promise((resolve) => (this.dialog = { title, value, resolve }))
  }

  // Rechtsklick-Menü an der Mausposition öffnen
  menu(e: MouseEvent, items: MenuItem[]) {
    e.preventDefault()
    e.stopPropagation()
    this.ctx = { x: e.clientX, y: e.clientY, items }
  }

  // Fenster öffnen oder nach vorn holen; `arg` ist Index/ID des bearbeiteten Objekts bzw. der Ansicht
  openWin(kind: Win['kind'], arg = 0) {
    const id = kind + ':' + arg
    const ex = this.wins.find((w) => w.id === id)
    if (ex) { ex.z = ++this.z; return }
    const n = this.wins.length
    const v = kind === 'view' ? WIN_VIEWS[arg] : null
    const big = kind === 'keypad' ? { w: 260, h: 430 } : kind === 'typedit' ? { w: 400, h: 470 } : v ? { w: v.w, h: v.h } : { w: 340, h: 300 }
    this.wins.push({ id, kind, arg, x: 380 + n * 28, y: 90 + n * 28, ...big, z: ++this.z })
  }
  toggleWin(kind: Win['kind'], arg = 0) {
    const id = kind + ':' + arg
    if (this.wins.some((w) => w.id === id)) this.closeWin(id)
    else this.openWin(kind, arg)
  }
  closeWin(id: string) { this.wins = this.wins.filter((w) => w.id !== id) }
  // Fenster nach vorn holen
  focusWin(id: string) { const w = this.wins.find((x) => x.id === id); if (w) w.z = ++this.z }

  get hasProgrammer() {
    return !!this.live && Object.keys(this.live.programmer).length > 0
  }

  // Wie am Pult: vor jedem Cue-Wechsel wird der Programmer gelöscht.
  async play(kind: 'Go' | 'Back' | 'ReleaseList', list: number) {
    if (kind !== 'ReleaseList' && this.hasProgrammer) await this.act('ClearProgrammer')
    await this.act({ [kind]: list })
  }
}
export const S = new Store()
