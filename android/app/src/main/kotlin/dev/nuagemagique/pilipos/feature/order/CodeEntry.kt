package dev.nuagemagique.pilipos.feature.order

import uniffi.pos_ffi.MenuItem

/*
 * Order entry by menu code (`ui/src/utils/codeEntry.ts`): the operator types the code printed on the menu ("A12",
 * "105"), the habit of the tills most Asian restaurants used before.
 *
 *   A12      one A12
 *   3*A12    three A12 (the number before x is the quantity)
 *   A12*3    three A12
 *   3*       three of the next product, typed or tapped in the grid
 *
 * While a code is typed the grid shows the products whose code starts with it ([searchByCode]); removing a product is
 * the cart's job. Pure: the screen keeps its own rules (modes, dialogs); this file only reads the text.
 */

/** Longest text the pad takes. */
internal const val CODE_ENTRY_MAX = 12

/** Largest quantity a multiplier gives (two digits). */
internal const val CODE_QUANTITY_MAX = 99

/** The multiply sign as it is kept in the typed text. */
internal const val TIMES = "*"

private const val CODE = "[A-Z0-9]+"
private const val QTY = "\\d{1,2}"
private val Multiplier = Regex("^($QTY)\\*$")
private val QtyFirst = Regex("^($QTY)\\*($CODE)$")
private val CodeFirst = Regex("^($CODE)\\*($QTY)$")
private val Plain = Regex("^($CODE)$")
private val Blanks = Regex("\\s+")
private val KeyChar = Regex("^[A-Z0-9]$")

/** What the typed text asks for. */
internal sealed interface CodeEntry {
    data object Empty : CodeEntry

    /** "3*": waits for a code or a tap in the grid. */
    data class Multiplier(
        val quantity: Int,
    ) : CodeEntry

    data class Add(
        val code: String,
        val quantity: Int,
    ) : CodeEntry

    data object Invalid : CodeEntry
}

/** A code as it is compared: no blanks, upper case. */
internal fun normalizeCode(code: String): String = code.replace(Blanks, "").uppercase()

private fun quantity(digits: String): Int? = digits.toInt().takeIf { it in 1..CODE_QUANTITY_MAX }

/** Reads the typed [text]. */
internal fun parseCodeEntry(text: String): CodeEntry {
    val s = normalizeCode(text)
    return if (s.isEmpty()) CodeEntry.Empty else Parsers.firstNotNullOfOrNull { it(s) } ?: CodeEntry.Invalid
}

/** An addition of [code] with the quantity [digits], invalid when the quantity is out of range. */
private fun addition(
    code: String,
    digits: String,
): CodeEntry = quantity(digits)?.let { CodeEntry.Add(code, it) } ?: CodeEntry.Invalid

/** The forms of an entry, in the order they are tried; a form whose quantity is out of range is invalid. */
private val Parsers: List<(String) -> CodeEntry?> =
    listOf(
        { s -> Multiplier.find(s)?.let { m -> quantity(m.groupValues[1])?.let { CodeEntry.Multiplier(it) } ?: CodeEntry.Invalid } },
        { s -> QtyFirst.find(s)?.let { m -> addition(m.groupValues[2], m.groupValues[1]) } },
        { s -> CodeFirst.find(s)?.let { m -> addition(m.groupValues[1], m.groupValues[2]) } },
        { s -> Plain.find(s)?.let { CodeEntry.Add(it.groupValues[1], 1) } },
    )

/** The code being typed, else null. */
internal fun CodeEntry.typedCode(): String? = (this as? CodeEntry.Add)?.code

/**
 * The text after one [key] of the pad: a digit, a letter or [TIMES]. A key that cannot make a valid entry is ignored:
 * a second multiply sign, one on an empty text, or more than [CODE_ENTRY_MAX] characters.
 */
internal fun typeKey(
    text: String,
    key: String,
): String {
    val k = key.uppercase()
    return when {
        text.length >= CODE_ENTRY_MAX -> text
        k == TIMES -> if (text.isEmpty() || text.contains(TIMES)) text else text + TIMES
        KeyChar.matches(k) -> text + k
        else -> text
    }
}

/** The product a code names. */
internal sealed interface CodeMatch {
    data class Found(
        val item: MenuItem,
    ) : CodeMatch

    data object Unknown : CodeMatch

    data object Ambiguous : CodeMatch
}

/** Looks [code] up among the products that have one (exact, any case). */
internal fun findByCode(
    items: List<MenuItem>,
    code: String,
): CodeMatch {
    val wanted = normalizeCode(code)
    val found = items.filter { item -> item.code?.let { normalizeCode(it) == wanted } == true }
    return when (found.size) {
        0 -> CodeMatch.Unknown
        1 -> CodeMatch.Found(found.single())
        else -> CodeMatch.Ambiguous
    }
}

/** The letters the menu codes use, sorted: the letter keys of the pad. */
internal fun codeLetters(items: List<MenuItem>): List<String> =
    items
        .flatMap { normalizeCode(it.code.orEmpty()).filter { ch -> ch in 'A'..'Z' }.toList() }
        .distinct()
        .sorted()
        .map { it.toString() }

/** A product added through a multiplier must take a plain quantity: not weighed, no options. */
internal fun MenuItem.takesQuantity(): Boolean = !soldByWeight && !hasModifiers

/** A product added at once, with no dialog in between. */
internal fun MenuItem.addsDirectly(): Boolean = !openPrice && !hasModifiers && !soldByWeight
