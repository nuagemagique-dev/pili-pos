import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test'
import { expiryOf, MAX_VISIBLE_TOASTS, pickVisible, useToastStore } from './toast'

describe('expiryOf', () => {
  it('lets info, success and warning toasts expire after their duration', () => {
    expect(expiryOf('info', 1000, 3000)).toBe(4000)
    expect(expiryOf('success', 1000, 500)).toBe(1500)
    expect(expiryOf('warning', 1000, 6000)).toBe(7000)
  })

  it('never expires an error toast, whatever duration was given', () => {
    expect(expiryOf('error', 1000, 3000)).toBe(Number.POSITIVE_INFINITY)
    expect(expiryOf('error', 1000, 1)).toBe(Number.POSITIVE_INFINITY)
  })
})

describe('toast store (C5)', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    // The store uses window.setInterval; the node test runtime has no window.
    vi.stubGlobal('window', globalThis)
    setActivePinia(createPinia())
  })

  afterEach(() => {
    vi.useRealTimers()
    vi.unstubAllGlobals()
  })

  it('drops info and success toasts after their duration', () => {
    const toast = useToastStore()
    toast.show('saved', 'success', 2000)
    toast.show('hello', 'info')
    expect(toast.toasts).toHaveLength(2)
    vi.advanceTimersByTime(2300)
    expect(toast.toasts.map((t) => t.message)).toEqual(['hello'])
    vi.advanceTimersByTime(1000)
    expect(toast.toasts).toHaveLength(0)
  })

  it('keeps an error until it is dismissed', () => {
    const toast = useToastStore()
    const id = toast.show('boom', 'error', 5000)
    toast.show('ok', 'info', 1000)
    vi.advanceTimersByTime(60_000)
    expect(toast.toasts.map((t) => t.message)).toEqual(['boom'])
    toast.dismiss(id)
    expect(toast.toasts).toHaveLength(0)
  })

  it('stops its cleanup loop when only errors are left, and restarts it', () => {
    const toast = useToastStore()
    toast.show('boom', 'error')
    toast.show('ok', 'info', 1000)
    vi.advanceTimersByTime(2000)
    expect(toast.cleanup_handle).toBeNull()
    toast.show('later', 'warning', 1000)
    expect(toast.cleanup_handle).not.toBeNull()
    vi.advanceTimersByTime(2000)
    expect(toast.toasts.map((t) => t.message)).toEqual(['boom'])
  })

  it('draws at most the newest three toasts', () => {
    const toast = useToastStore()
    for (let i = 1; i <= 5; i++) toast.show(`m${i}`, 'info', 60_000)
    expect(MAX_VISIBLE_TOASTS).toBe(3)
    expect(toast.visible.map((t) => t.message)).toEqual(['m3', 'm4', 'm5'])
    expect(toast.toasts).toHaveLength(5)
  })

  it('keeps an error in view when three newer messages arrive', () => {
    const toast = useToastStore()
    toast.show('boom', 'error')
    for (let i = 1; i <= 3; i++) toast.show(`m${i}`, 'info', 60_000)
    expect(toast.visible.map((t) => t.message)).toEqual(['boom', 'm2', 'm3'])
  })

  it('picks the newest errors first, then the newest others, in arrival order', () => {
    const mk = (id: number, variant: 'info' | 'error') => ({
      id,
      message: `t${id}`,
      variant,
      expires_at: 0,
    })
    const all = [
      mk(1, 'error'),
      mk(2, 'info'),
      mk(3, 'error'),
      mk(4, 'info'),
      mk(5, 'error'),
      mk(6, 'error'),
    ]
    expect(pickVisible(all).map((t) => t.id)).toEqual([3, 5, 6])
    expect(pickVisible(all.slice(0, 4)).map((t) => t.id)).toEqual([1, 3, 4])
    expect(pickVisible([])).toEqual([])
  })

  it('runs the action of a toast once and removes the toast (C8)', () => {
    const toast = useToastStore()
    const run = vi.fn()
    const id = toast.showWithAction('Table 4 déplacée en 9', { label: 'Annuler', run }, 5000)
    expect(toast.toasts[0].action?.label).toBe('Annuler')

    toast.runAction(id)
    toast.runAction(id)

    expect(run).toHaveBeenCalledTimes(1)
    expect(toast.toasts).toHaveLength(0)
  })

  it('keeps the action toast for its 5 s and then drops it without running the action', () => {
    const toast = useToastStore()
    const run = vi.fn()
    toast.showWithAction('moved', { label: 'Annuler', run }, 5000)

    vi.advanceTimersByTime(4700)
    expect(toast.toasts).toHaveLength(1)
    vi.advanceTimersByTime(600)
    expect(toast.toasts).toHaveLength(0)
    expect(run).not.toHaveBeenCalled()
  })
})
