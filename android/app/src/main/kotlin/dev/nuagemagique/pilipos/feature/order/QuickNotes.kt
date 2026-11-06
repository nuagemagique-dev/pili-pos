package dev.nuagemagique.pilipos.feature.order

/** The most chips the list holds (`QUICK_NOTES_MAX` of `utils/quickNotes.ts`, `MAX_QUICK_NOTES` of the engine). */
internal const val QUICK_NOTES_MAX: Int = 24

private const val SEPARATOR = ", "

/** The parts of a note, trimmed, empty ones dropped ("sans wasabi, bien cuit"). */
private fun noteParts(note: String): List<String> = note.split(',').map { it.trim() }.filter { it.isNotEmpty() }

/** Whether [chip] is already one of the parts of [note] (ignoring case). */
internal fun noteHasChip(
    note: String,
    chip: String,
): Boolean {
    val joined = (SEPARATOR + noteParts(note).joinToString(SEPARATOR) + SEPARATOR).lowercase()
    return joined.contains((SEPARATOR + chip.trim() + SEPARATOR).lowercase())
}

/**
 * The note after a tap on [chip] (`appendQuickNote` of `utils/quickNotes.ts`): the chip alone on an empty note, else
 * the note, a comma and the chip. Null when the tap changes nothing: the chip is already in the note, or the result
 * would pass [max].
 */
internal fun appendQuickNote(
    note: String,
    chip: String,
    max: Int = NOTE_MAX,
): String? {
    val text = chip.trim()
    if (text.isEmpty() || noteHasChip(note, text)) return null
    val base = noteParts(note).joinToString(SEPARATOR)
    val next = if (base.isEmpty()) text else base + SEPARATOR + text
    return if (next.length > max) null else next
}

/** [list] with the item at [index] moved by [delta] places (clamped); the same list when nothing moves. */
internal fun moveQuickNote(
    list: List<String>,
    index: Int,
    delta: Int,
): List<String> {
    val to = (index + delta).coerceIn(0, maxOf(list.lastIndex, 0))
    if (index !in list.indices || to == index) return list
    return list.toMutableList().also { it.add(to, it.removeAt(index)) }
}

/** [list] with the item at [index] removed. */
internal fun removeQuickNote(
    list: List<String>,
    index: Int,
): List<String> = list.filterIndexed { i, _ -> i != index }

/**
 * [list] with [text] added at the end, trimmed (`addQuickNote` of `utils/quickNotes.ts`). Null when it is blank, too
 * long, already there (ignoring case) or the list is full.
 */
internal fun addQuickNote(
    list: List<String>,
    text: String,
    maxCount: Int = QUICK_NOTES_MAX,
): List<String>? {
    val note = text.trim()
    val refused = note.isEmpty() || note.length > NOTE_MAX || list.size >= maxCount || list.any { it.equals(note, ignoreCase = true) }
    return if (refused) null else list + note
}
