// Every date the POS shows is Belgian time, whatever time zone the
// device runs in (tickets and reports are fiscally in Europe/Brussels).
export const TIME_ZONE = 'Europe/Brussels'

/** Parse a backend timestamp. SQLite's datetime('now') gives
 * "YYYY-MM-DD HH:MM:SS" in UTC, without a zone; ISO strings with a
 * zone are taken as they are. */
export function parseTimestamp(iso: string): Date | null {
  const hasZone = /(?:Z|[+-]\d{2}:?\d{2})$/u.test(iso)
  const d = new Date(iso.replace(' ', 'T') + (hasZone ? '' : 'Z'))
  return Number.isNaN(d.getTime()) ? null : d
}

function format(d: Date, options: Intl.DateTimeFormatOptions): string {
  return d.toLocaleString('fr-BE', { ...options, timeZone: TIME_ZONE })
}

/** "14:05" */
export function formatTime(d: Date): string {
  return format(d, { hour: '2-digit', minute: '2-digit' })
}

/** "25/09/2026" */
export function formatDate(d: Date): string {
  return format(d, { day: '2-digit', month: '2-digit', year: 'numeric' })
}

/** "25/09 14:05" */
export function formatDayTime(d: Date): string {
  return format(d, { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' })
}

/** A backend timestamp through `formatter`, or the raw text when it
 * does not parse (never an "Invalid Date" on screen). */
export function formatTimestamp(iso: string, formatter: (d: Date) => string): string {
  const d = parseTimestamp(iso)
  return d ? formatter(d) : iso
}

/** Whole minutes from a backend timestamp to `now` (0 when unparsable). */
export function minutesSince(iso: string, now: Date = new Date()): number {
  const d = parseTimestamp(iso)
  return d ? Math.max(0, Math.floor((now.getTime() - d.getTime()) / 60_000)) : 0
}
