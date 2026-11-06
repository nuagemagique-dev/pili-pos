package dev.nuagemagique.pilipos.ui.state

import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import uniffi.pos_ffi.TableInfo

/**
 * How the tables are written and grouped on screen (UX audit T1), port of `ui/src/utils/tables.ts`. A plan can name
 * zones with letters ("A 1-20, B 21-40"): a table in a zone reads "A-19", one outside any zone "19". The table is
 * still its number: the engine signs and prints on the fiscal ticket the number alone.
 */
internal fun inZone(zone: String?): Boolean = !zone.isNullOrEmpty()

/** "A-19" for a table in a zone, "19" for a table without one. */
internal fun tableLabel(
    zone: String?,
    number: Int,
): String = if (inZone(zone)) "$zone-$number" else number.toString()

/** "A · Salle", or "A" for a zone without a name. */
internal fun zoneTitle(
    zone: String,
    name: String?,
): String = if (name.isNullOrEmpty()) zone else "$zone · $name"

/** The tables of one zone, in the order the plan gives them. */
internal data class TableGroup(
    /** The zone letters, null for the tables outside every zone. */
    val zone: String?,
    /** The name a manager gave the zone, null when it has none. */
    val name: String?,
    val tables: List<TableInfo>,
)

/**
 * The tables grouped by zone, zones in alphabetical order. A plan without letters is one group with no zone (the
 * screen then draws no header). The tables outside every zone (an open order on a table the plan left out) come last.
 */
internal fun groupTables(tables: List<TableInfo>): List<TableGroup> {
    val byZone = LinkedHashMap<String?, MutableList<TableInfo>>()
    for (table in tables) {
        val key = table.zone.takeIf { inZone(it) }
        byZone.getOrPut(key) { mutableListOf() }.add(table)
    }
    val zoned =
        byZone.entries
            .filter { it.key != null }
            .sortedBy { it.key }
            .map { (zone, list) -> TableGroup(zone, list.firstNotNullOfOrNull { it.zoneName }, list) }
    val plain = byZone[null]?.let { TableGroup(null, null, it) }
    return if (plain != null) zoned + plain else zoned
}

/** Whether the plan has zones: the screens then show the headers. */
internal fun hasZones(tables: List<TableInfo>): Boolean = tables.any { inZone(it.zone) }

/**
 * How a screen that only knows a table number writes it (the reports lists, the unsettled card warning): the zones of the
 * plan, given by the scaffold. Without zones (a stateless screen in a test, a plan without letters) it writes plain numbers.
 */
@Immutable
internal class TableLabeler(
    private val zones: Map<Int, String> = emptyMap(),
) {
    /** "A-19" in a zone, "19" without one. */
    fun label(number: Int): String = tableLabel(zones[number], number)

    /** The compact form of the ticket lists: "A-19" in a zone, "T19" without one. */
    fun short(number: Int): String = zones[number]?.let { "$it-$number" } ?: "T$number"
}

/** The labeler of the screen on show (`useTablePlanStore` of the Vue app). */
internal val LocalTableLabeler = staticCompositionLocalOf { TableLabeler() }
