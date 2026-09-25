import { onMounted, onUnmounted } from 'vue'

/**
 * Global idle timer - auto-fires `onIdle` after `timeoutMs` of no
 * user interaction (touch, mouse, keyboard). Used by App.vue to
 * auto-lock the POS session after 5 minutes of inactivity so an
 * unattended terminal can't be operated under the previous
 * operator's INSZ. UX is silent: just redirects to login, no modal.
 */
export function useIdleTimer(timeoutMs: number, onIdle: () => void) {
  let timer: ReturnType<typeof setTimeout> | null = null

  function reset() {
    if (timer !== null) clearTimeout(timer)
    timer = setTimeout(onIdle, timeoutMs)
  }

  const events: (keyof DocumentEventMap)[] = [
    'touchstart',
    'mousedown',
    'keydown',
  ]

  onMounted(() => {
    for (const evt of events) {
      document.addEventListener(evt, reset, { passive: true })
    }
    reset() // start the first timer
  })

  onUnmounted(() => {
    if (timer !== null) clearTimeout(timer)
    for (const evt of events) {
      document.removeEventListener(evt, reset)
    }
  })
}
