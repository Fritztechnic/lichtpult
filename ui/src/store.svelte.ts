import { call } from './api'
import type { Live, Show, FixtureType } from './lib/types'

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
  quick = $state(false)
  dialog = $state<{ title: string; value: string; resolve: (v: string | null) => void } | null>(null)
  private busy = false
  private showRev = -1

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

  async act(...a: unknown[]) {
    try {
      await call('dispatch', a.length === 1 ? a[0] : a)
    } catch (e) {
      this.say(String(e), true)
    }
  }

  async cmd(line: string) {
    try {
      await call('command', line)
      this.history = [line, ...this.history.filter((h) => h !== line)].slice(0, 50)
      this.say(line)
    } catch (e) {
      this.say(String(e), true)
    }
  }

  ask(title: string, value = ''): Promise<string | null> {
    return new Promise((resolve) => (this.dialog = { title, value, resolve }))
  }

  get hasProgrammer() {
    return !!this.live && Object.keys(this.live.programmer).length > 0
  }

  // Playback semantics as in the console: programmer is cleared before cue changes.
  async play(kind: 'Go' | 'Back' | 'ReleaseList', list: number) {
    if (kind !== 'ReleaseList' && this.hasProgrammer) await this.act('ClearProgrammer')
    await this.act({ [kind]: list })
  }
}
export const S = new Store()
