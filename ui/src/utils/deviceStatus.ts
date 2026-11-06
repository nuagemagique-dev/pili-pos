// What the status sheet (UX audit L2) shows about one device: the same
// shape for the header lights and the start-up rows of the login screen.

/** The colour of a device light (UX guide §8.1). */
export type Light = 'ok' | 'warn' | 'err' | 'none'

export interface DeviceStatus {
  /** Stable key: fdm, ticket, kitchen, terminal, scale, hubrise, mail. */
  key: string
  /** Full device name ("Imprimante ticket"). */
  name: string
  state: Light
  /** Why the device is in that state, in words (never only a tooltip). */
  reason: string
  /** When it was last checked (ms since epoch); null before the first check. */
  checkedAt: number | null
}

/** Vue key of the state in words ("Fonctionne", "Ne répond pas"...). */
export function stateWordKey(state: Light): string {
  return `status_state_${state}`
}
