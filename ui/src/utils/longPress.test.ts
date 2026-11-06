import { describe, expect, it } from 'vite-plus/test'
import { createLongPress, LONG_PRESS_MS, type LongPressTimers } from './longPress'

/** A clock the test moves by hand. */
function fakeTimers() {
  let now = 0
  let next = 1
  const pending = new Map<number, { at: number; run: () => void }>()
  const timers: LongPressTimers = {
    set: (run, ms) => {
      const id = next++
      pending.set(id, { at: now + ms, run })
      return () => void pending.delete(id)
    },
  }
  return {
    timers,
    advance(ms: number) {
      now += ms
      for (const [id, t] of [...pending]) {
        if (t.at <= now) {
          pending.delete(id)
          t.run()
        }
      }
    },
  }
}

function setup() {
  const clock = fakeTimers()
  let fired = 0
  const press = createLongPress(() => fired++, { timers: clock.timers })
  return { clock, press, fired: () => fired }
}

describe('createLongPress', () => {
  it('fires after 500 ms and not before', () => {
    const { clock, press, fired } = setup()
    press.start(10, 10)
    clock.advance(LONG_PRESS_MS - 1)
    expect(fired()).toBe(0)
    clock.advance(1)
    expect(fired()).toBe(1)
  })

  it('does not fire when the finger lifts early, and that tap is a normal click', () => {
    const { clock, press, fired } = setup()
    press.start(10, 10)
    clock.advance(300)
    press.cancel()
    clock.advance(1000)
    expect(fired()).toBe(0)
    expect(press.consumeClick()).toBe(false)
  })

  it('cancels when the finger drifts further than the slop (a scroll)', () => {
    const { clock, press, fired } = setup()
    press.start(10, 10)
    press.move(14, 12)
    clock.advance(200)
    press.move(10, 60)
    clock.advance(1000)
    expect(fired()).toBe(0)
  })

  it('keeps going for a small drift', () => {
    const { clock, press, fired } = setup()
    press.start(10, 10)
    press.move(15, 14)
    clock.advance(LONG_PRESS_MS)
    expect(fired()).toBe(1)
  })

  it('swallows the click that follows a long press, once', () => {
    const { clock, press } = setup()
    press.start(0, 0)
    clock.advance(LONG_PRESS_MS)
    press.cancel() // the finger lifts
    expect(press.consumeClick()).toBe(true)
    expect(press.consumeClick()).toBe(false)
  })

  it('a new press forgets the previous long press', () => {
    const { clock, press, fired } = setup()
    press.start(0, 0)
    clock.advance(LONG_PRESS_MS)
    press.start(0, 0)
    expect(press.consumeClick()).toBe(false)
    clock.advance(LONG_PRESS_MS)
    expect(fired()).toBe(2)
  })

  it('reset forgets a long press whose click never came', () => {
    const { clock, press } = setup()
    press.start(0, 0)
    clock.advance(LONG_PRESS_MS)
    press.reset()
    expect(press.consumeClick()).toBe(false)
  })

  it('reset stops a press that is still waiting', () => {
    const { clock, press, fired } = setup()
    press.start(0, 0)
    press.reset()
    clock.advance(LONG_PRESS_MS * 2)
    expect(fired()).toBe(0)
  })
})
