// Ziehen von Trennlinien: `move` bekommt die Verschiebung seit dem Start in Pixeln.
export function startDrag(e: PointerEvent, move: (dx: number, dy: number) => void, done?: () => void) {
  e.preventDefault()
  const sx = e.clientX, sy = e.clientY
  const mv = (m: PointerEvent) => move(m.clientX - sx, m.clientY - sy)
  const up = () => {
    window.removeEventListener('pointermove', mv)
    window.removeEventListener('pointerup', up)
    done?.()
  }
  window.addEventListener('pointermove', mv)
  window.addEventListener('pointerup', up)
}
