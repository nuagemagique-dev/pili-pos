import { watch } from 'vue'
import { useOrderStore } from '../stores/order'
import { usePosStore } from '../stores/pos'

/** Pitch of the beep that confirms an add (A5). */
export const ADD_BEEP_HZ = 880
/** Length of the beep, in milliseconds: short enough to follow a fast series of taps. */
export const ADD_BEEP_MS = 60

/**
 * Plays a short beep on every successful add to the cart (grid tap, code
 * pad OK, digit after an add, option and price dialogs), when this
 * terminal's "Bip à chaque ajout" switch is on (default on). Web Audio, no
 * file asset, like `useOrderChime`; the cart line flashes on the same add.
 * The Compose UI has the same beep (`AddBeep.kt`).
 */
export function useAddBeep() {
  const order = useOrderStore()
  const pos = usePosStore()
  let audioCtx: AudioContext | null = null

  function playBeep() {
    try {
      audioCtx ??= new AudioContext()
      const now = audioCtx.currentTime
      const end = now + ADD_BEEP_MS / 1000
      const osc = audioCtx.createOscillator()
      const gain = audioCtx.createGain()
      osc.type = 'sine'
      osc.frequency.value = ADD_BEEP_HZ
      gain.gain.setValueAtTime(0.2, now)
      gain.gain.exponentialRampToValueAtTime(0.001, end)
      osc.connect(gain).connect(audioCtx.destination)
      osc.start(now)
      osc.stop(end)
    } catch {
      // Audio may be blocked until the first touch: the line flash still confirms the add.
    }
  }

  watch(
    () => order.lastAdd?.seq,
    (seq) => {
      if (seq !== undefined && pos.addBeep) playBeep()
    },
  )
}
