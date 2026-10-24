import { watch } from 'vue'
import { useWebshopOrdersStore } from '../stores/webshopOrders'

/**
 * Plays a two-tone chime when the pending webshop order count
 * increases - draws the operator's attention even if they're
 * heads-down on a dine-in flow. Uses the Web Audio API, no file
 * assets needed. UX Guide §4.2 allows informative audio.
 *
 * The chime is C5 (523 Hz) → E5 (659 Hz), 150ms each, sine wave.
 * Same pattern used by the tsb-dashboard.
 */
export function useOrderChime() {
  const store = useWebshopOrdersStore()
  let audioCtx: AudioContext | null = null

  function playChime() {
    try {
      audioCtx ??= new AudioContext()
      const now = audioCtx.currentTime

      // Note 1: C5 (523 Hz)
      const osc1 = audioCtx.createOscillator()
      const gain1 = audioCtx.createGain()
      osc1.type = 'sine'
      osc1.frequency.value = 523.25
      gain1.gain.setValueAtTime(0.3, now)
      gain1.gain.exponentialRampToValueAtTime(0.001, now + 0.15)
      osc1.connect(gain1).connect(audioCtx.destination)
      osc1.start(now)
      osc1.stop(now + 0.15)

      // Note 2: E5 (659 Hz), starts after note 1
      const osc2 = audioCtx.createOscillator()
      const gain2 = audioCtx.createGain()
      osc2.type = 'sine'
      osc2.frequency.value = 659.25
      gain2.gain.setValueAtTime(0.3, now + 0.15)
      gain2.gain.exponentialRampToValueAtTime(0.001, now + 0.35)
      osc2.connect(gain2).connect(audioCtx.destination)
      osc2.start(now + 0.15)
      osc2.stop(now + 0.35)
    } catch {
      // AudioContext may be blocked by browser autoplay policy
      // until the first user interaction. Silently degrade.
    }
  }

  // Skip the first emission (initial fetchOrders loads existing
  // orders from DB - that's not a "new" order arriving). Only
  // chime on subsequent increments.
  let initialized = false
  watch(
    () => store.pendingCount,
    (next, prev) => {
      if (!initialized) {
        initialized = true
        return
      }
      if (next > (prev ?? 0)) {
        playChime()
      }
    },
  )
}
