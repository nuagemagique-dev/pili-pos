// The quick notes (UX audit C11): chips under the text of a kitchen note. A tap appends the chip to
// the note, comma separated; the 60 character limit of the note still applies. Pure rules, mirrored by
// `QuickNotes.kt` on Android.

/** The longest kitchen note (the engine refuses more: `note_too_long`). */
export const NOTE_MAX = 60

/** The most chips the list holds (the engine refuses more: `quick_notes_too_many`). */
export const QUICK_NOTES_MAX = 24

const SEPARATOR = ', '

/** The parts of a note, trimmed, empty ones dropped ("sans wasabi, bien cuit"). */
function parts(note: string): string[] {
  return note
    .split(',')
    .map((p) => p.trim())
    .filter((p) => p !== '')
}

/** Whether [chip] is already one of the parts of [note] (ignoring case). */
export function noteHasChip(note: string, chip: string): boolean {
  const joined = `${SEPARATOR}${parts(note).join(SEPARATOR)}${SEPARATOR}`.toLowerCase()
  return joined.includes(`${SEPARATOR}${chip.trim()}${SEPARATOR}`.toLowerCase())
}

/**
 * The note after a tap on [chip]: the chip alone on an empty note, else the note, a comma and the chip.
 * Null when the tap changes nothing: the chip is already in the note, or the result would pass [max].
 */
export function appendQuickNote(note: string, chip: string, max = NOTE_MAX): string | null {
  const text = chip.trim()
  if (text === '' || noteHasChip(note, text)) return null
  const base = parts(note).join(SEPARATOR)
  const next = base === '' ? text : `${base}${SEPARATOR}${text}`
  return next.length > max ? null : next
}

/** [list] with the item at [index] moved by [delta] places (clamped); the same list when nothing moves. */
export function moveQuickNote(list: readonly string[], index: number, delta: number): string[] {
  const to = Math.min(Math.max(index + delta, 0), list.length - 1)
  const out = [...list]
  if (index < 0 || index >= list.length || to === index) return out
  const [item] = out.splice(index, 1)
  out.splice(to, 0, item as string)
  return out
}

/** [list] with the item at [index] removed. */
export function removeQuickNote(list: readonly string[], index: number): string[] {
  return list.filter((_, i) => i !== index)
}

/**
 * [list] with [text] added at the end, trimmed. Null when it is blank, too long, already there (ignoring
 * case) or the list is full.
 */
export function addQuickNote(
  list: readonly string[],
  text: string,
  maxCount = QUICK_NOTES_MAX,
): string[] | null {
  const note = text.trim()
  if (note === '' || note.length > NOTE_MAX || list.length >= maxCount) return null
  if (list.some((n) => n.toLowerCase() === note.toLowerCase())) return null
  return [...list, note]
}
