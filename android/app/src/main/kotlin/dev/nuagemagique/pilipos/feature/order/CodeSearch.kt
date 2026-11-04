package dev.nuagemagique.pilipos.feature.order

import uniffi.pos_ffi.MenuItem

/*
 * The search of the code pad (`searchByCode` and `pickByCode` of `ui/src/utils/codeEntry.ts`): while a code is typed
 * the grid shows the products whose code starts with it, and OK takes the exact code or the only result.
 */

private val Chunks = Regex("\\d+|\\D+")

/** Natural order of two codes of the same length: digit runs compare as numbers (A9B before A10). */
private val NaturalOrder =
    Comparator<String> { a, b ->
        Chunks
            .findAll(a)
            .map { it.value }
            .zip(Chunks.findAll(b).map { it.value })
            .map { (x, y) -> naturalChunk(x, y) }
            .firstOrNull { it != 0 } ?: a.compareTo(b)
    }

private fun naturalChunk(
    x: String,
    y: String,
): Int =
    if (x[0].isDigit() && y[0].isDigit()) {
        x.toBigInteger().compareTo(y.toBigInteger())
    } else {
        x.compareTo(y)
    }

/**
 * The products whose code starts with [code], for the grid while a code is typed: the shorter codes first (the exact
 * one leads), then in natural order (A2 before A10), as `searchByCode` of the Vue UI.
 */
internal fun searchByCode(
    items: List<MenuItem>,
    code: String,
): List<MenuItem> {
    val wanted = normalizeCode(code)
    if (wanted.isEmpty()) return emptyList()
    val byCode = compareBy<Pair<MenuItem, String>> { it.second.length }.thenBy(NaturalOrder) { it.second }
    return items
        .map { it to normalizeCode(it.code.orEmpty()) }
        .filter { (_, c) -> c.startsWith(wanted) }
        .sortedWith(byCode)
        .map { it.first }
}

/**
 * The product OK takes for [code]: the exact code, else the only product whose code starts with it. Several products
 * under the exact code stay ambiguous; nothing found, or several starting with it, is unknown.
 */
internal fun pickByCode(
    items: List<MenuItem>,
    code: String,
): CodeMatch =
    findByCode(items, code).takeIf { it != CodeMatch.Unknown }
        ?: searchByCode(items, code).singleOrNull()?.let { CodeMatch.Found(it) }
        ?: CodeMatch.Unknown
