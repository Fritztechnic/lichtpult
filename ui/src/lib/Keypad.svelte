<script lang="ts">
  // Tastenfeld für die Befehlszeile; die Tasten schreiben nur in `S.cmdText`.
  import { S } from '../store.svelte'

  const word = (t: string) => (S.cmdText = S.cmdText.trimEnd() + (S.cmdText ? ' ' : '') + t + ' ')
  const raw = (t: string) => (S.cmdText += t)
  async function enter() {
    const c = S.cmdText.trim()
    if (c) await S.cmd(c)
    S.cmdText = ''
  }
  function clear() {
    if (S.cmdText) S.cmdText = ''
    else S.act('ClearProgrammer')
  }

  type K = { l: string; run: () => unknown; acc?: string; on?: boolean }
  const live = $derived(S.live!)
  const top: K[] = $derived([
    { l: 'Highlight', run: () => S.act({ Highlight: !live.highlight }), acc: 'var(--yellow)', on: live.highlight },
    { l: 'Blind', run: () => S.act({ SetBlind: !live.blind }), acc: 'var(--yellow)', on: live.blind },
    { l: 'Last', run: () => S.act({ SelectStep: -1 }), acc: 'var(--blue)' },
    { l: 'Next', run: () => S.act({ SelectStep: 1 }), acc: 'var(--green)' },
    { l: 'Undo', run: () => S.act('Undo') },
    { l: 'Redo', run: () => S.act('Redo') },
    { l: 'Clear', run: clear, acc: 'var(--red-hi)' },
    { l: 'Delete', run: () => word('delete'), acc: 'var(--red-hi)' },
    { l: 'Record', run: () => word('record'), acc: 'var(--red-hi)' },
    { l: 'Group', run: () => word('group') },
    { l: 'Cue', run: () => word('cue') },
    { l: 'Release', run: () => word('release') },
  ])
  const rows = [['7', '8', '9', 'thru'], ['4', '5', '6', 'full'], ['1', '2', '3', '@'], ['0', '.', '+', '-']]
</script>

<div class="kp">
  <div class="disp mono">{S.cmdText || ' '}</div>
  <div class="grid">
    {#each top as k}
      <button class="tool" class:on={k.on} style={k.acc ? `--acc:${k.acc}` : ''} onclick={k.run}>{k.l}</button>
    {/each}
  </div>
  <div class="grid nums">
    {#each rows as r}{#each r as k}
      <button class="btn" onclick={() => (k.length === 1 && k !== '@' && k !== '+' && k !== '-' ? raw(k) : word(k))}>{k === 'thru' ? 'Thru' : k === 'full' ? 'Full' : k}</button>
    {/each}{/each}
  </div>
  <button class="btn enter" onclick={enter}>Enter</button>
</div>

<style>
  .kp { padding-top: 2px; }
  .disp { margin: 4px; padding: 6px; min-height: 28px; background: #000; border: 1px solid var(--line-hi); color: var(--orange); font-size: 13px; white-space: pre-wrap; word-break: break-all; }
  .grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 3px; padding: 0 4px 4px; }
  .grid .tool { min-width: 0; padding: 8px 2px 6px; font-size: 11px; }
  .nums .btn { padding: 12px 0; font-size: 15px; }
  .enter { margin: 0 4px 4px; width: calc(100% - 8px); padding: 12px; background: var(--btn-green); border-color: #50d068; font-weight: 700; }
</style>
