/**
 * An action that asks for a second tap (UX audit L1): the first tap arms it
 * and shows "Confirmer ... ?", a second tap within `ARM_MS` fires it, and
 * after that time the button is back to its first state.
 */

/** How long a first tap stays armed, in milliseconds. */
export const ARM_MS = 5000

export interface ArmedStep {
  /** The second tap came in time: do the action. */
  fire: boolean
  /** When the armed state ends (ms since epoch); null when not armed. */
  armedUntil: number | null
}

/** What a tap at `now` does, given when the button was armed until (null: not armed). */
export function tapArmed(armedUntil: number | null, now: number): ArmedStep {
  if (armedUntil !== null && now < armedUntil) return { fire: true, armedUntil: null }
  return { fire: false, armedUntil: now + ARM_MS }
}

/** Whether the button still shows its confirmation at `now`. */
export function isArmed(armedUntil: number | null, now: number): boolean {
  return armedUntil !== null && now < armedUntil
}
