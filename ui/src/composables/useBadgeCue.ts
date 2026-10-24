// Sound feedback of a badge scan (plan L5-D14), synthesised with Web
// Audio like the online order chime: no audio file. A high two-tone for a
// registered arrival, departure or login; a low tone for any refusal.

let audioCtx: AudioContext | null = null

interface Note {
  freq: number
  start: number
  length: number
}

function tone(ctx: AudioContext, { freq, start, length }: Note, volume: number) {
  const osc = ctx.createOscillator()
  const gain = ctx.createGain()
  osc.type = 'sine'
  osc.frequency.value = freq
  gain.gain.setValueAtTime(volume, start)
  gain.gain.exponentialRampToValueAtTime(0.001, start + length)
  osc.connect(gain).connect(ctx.destination)
  osc.start(start)
  osc.stop(start + length)
}

function play(notes: [number, number, number][]) {
  try {
    audioCtx ??= new AudioContext()
    const now = audioCtx.currentTime
    for (const [freq, at, length] of notes) tone(audioCtx, { freq, start: now + at, length }, 0.3)
  } catch {
    // Blocked by the autoplay policy or no audio device: the toast is the cue.
  }
}

export function useBadgeCue() {
  return {
    /** A2 then E6: arrival, departure or login registered. */
    ok: () => {
      play([
        [880, 0, 0.12],
        [1318.5, 0.12, 0.2],
      ])
    },
    /** A low tone: refused or failed. */
    refused: () => {
      play([[196, 0, 0.35]])
    },
  }
}
