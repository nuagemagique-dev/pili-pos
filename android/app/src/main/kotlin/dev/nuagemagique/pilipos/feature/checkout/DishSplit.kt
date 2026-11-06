package dev.nuagemagique.pilipos.feature.checkout

import dev.nuagemagique.pilipos.ui.format.CashDisplay
import dev.nuagemagique.pilipos.ui.state.DishUnits
import uniffi.pos_ffi.CartLine

/**
 * The split by dishes of the checkout (UX audit P3, `utils/dishSplit.ts` of the Vue UI): the cart lines are shared
 * between the friends unit by unit, a part is the sum of the units checked for it. The line nets already carry
 * every discount, offered item and offered line (the engine spreads an order discount over the lines), so they are
 * shared pro rata by construction. The shares of a line add up to its net exactly (the remainder cents go to the
 * first units, as for equal parts) and so the shares of the lines add up to the order total. The units never reach
 * the engine: the payment list is the one of equal parts.
 */
internal data class DishLine(
    val id: Long,
    val quantity: Int,
    val netCents: Long,
)

/** One row of the dish list: what a line has left, what was paid and what is checked for the current part. */
internal data class DishRow(
    val line: CartLine,
    val left: Int,
    val paid: Int,
    val picked: Int,
)

/** The cart lines as the split sees them. */
internal fun dishLinesOf(lines: List<CartLine>): List<DishLine> =
    lines.map {
        val net = CashDisplay.lineNetCents(it.unitPriceCents, it.weighedAmountCents, it.quantity, it.discountCents)
        DishLine(it.id, it.quantity, net)
    }

/** The units given to the parts taken, per line. */
internal fun paidUnits(parts: List<TakenPart>): Map<Long, Int> {
    val paid = mutableMapOf<Long, Int>()
    parts.forEach { part -> part.units.forEach { paid[it.lineId] = (paid[it.lineId] ?: 0) + it.count } }
    return paid
}

/** The units of [line] not given to a part yet. */
internal fun unitsLeft(
    line: DishLine,
    paid: Map<Long, Int>,
): Int = maxOf(0, line.quantity - (paid[line.id] ?: 0))

/** The amount of the units checked, after the units already given to the parts taken. */
internal fun pickedCents(
    lines: List<DishLine>,
    paid: Map<Long, Int>,
    pick: Map<Long, Int>,
): Long =
    CashDisplay.sumCents(
        lines.map { CashDisplay.unitsCents(it.netCents, it.quantity, paid[it.id] ?: 0, minOf(pick[it.id] ?: 0, unitsLeft(it, paid))) },
    )

/** Every unit not given yet is checked: the part pays the whole rest. */
internal fun allLeftPicked(
    lines: List<DishLine>,
    paid: Map<Long, Int>,
    pick: Map<Long, Int>,
): Boolean = lines.all { (pick[it.id] ?: 0) >= unitsLeft(it, paid) }

/** The checks with [count] units on a line that has [left] units, clamped (0 removes the entry). */
internal fun withUnits(
    pick: Map<Long, Int>,
    lineId: Long,
    left: Int,
    count: Int,
): Map<Long, Int> {
    val clamped = count.coerceIn(0, maxOf(0, left))
    return if (clamped == 0) pick - lineId else pick + (lineId to clamped)
}

/** A tap on the line: every unit left is checked, or none when all of them already were. */
internal fun toggledUnits(
    pick: Map<Long, Int>,
    lineId: Long,
    left: Int,
): Map<Long, Int> = withUnits(pick, lineId, left, if ((pick[lineId] ?: 0) >= left) 0 else left)

/** The units checked, as a list (lines in cart order). */
internal fun pickedUnits(
    lines: List<DishLine>,
    pick: Map<Long, Int>,
): List<DishUnits> = lines.filter { (pick[it.id] ?: 0) > 0 }.map { DishUnits(it.id, pick.getValue(it.id)) }

/** The parts kept still fit the cart: every line they name exists and has the units they took. */
internal fun unitsFit(
    lines: List<DishLine>,
    parts: List<List<DishUnits>>,
): Boolean {
    val paid = mutableMapOf<Long, Int>()
    parts.flatten().forEach { paid[it.lineId] = (paid[it.lineId] ?: 0) + it.count }
    return paid.all { (lineId, count) -> lines.firstOrNull { it.id == lineId }?.let { count <= it.quantity } == true }
}
