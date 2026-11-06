package dev.nuagemagique.pilipos.ui.state

import dev.nuagemagique.pilipos.feature.tables.table
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** The labels and the grouping of the tables by zone (`utils/tables.ts`, UX audit T1). */
class TableLabelsTest {
    @Test
    fun aTableInAZoneIsWrittenWithItsLetterAndOneWithoutIsAPlainNumber() {
        assertEquals("A-19", tableLabel("A", 19))
        assertEquals("TE-3", tableLabel("TE", 3))
        assertEquals("19", tableLabel(null, 19))
        assertEquals("19", tableLabel("", 19))
    }

    @Test
    fun aZoneIsTitledWithItsNameWhenItHasOne() {
        assertEquals("A · Salle", zoneTitle("A", "Salle"))
        assertEquals("B", zoneTitle("B", null))
        assertEquals("B", zoneTitle("B", ""))
    }

    @Test
    fun aPlanWithoutLettersIsOneGroupWithNoZone() {
        val tables = listOf(table(1), table(2))

        assertFalse(hasZones(tables))
        val groups = groupTables(tables)
        assertEquals(1, groups.size)
        assertNull(groups.single().zone)
        assertEquals(listOf(1, 2), groups.single().tables.map { it.tableNumber })
    }

    @Test
    fun theZonesComeInAlphabeticalOrderAndKeepThePlanOrderInside() {
        val tables =
            listOf(
                table(21, zone = "B", zoneName = "Terrasse"),
                table(1, zone = "A", zoneName = "Salle"),
                table(22, zone = "B", zoneName = "Terrasse"),
                table(2, zone = "A", zoneName = "Salle"),
            )

        assertTrue(hasZones(tables))
        val groups = groupTables(tables)
        assertEquals(listOf("A", "B"), groups.map { it.zone })
        assertEquals("Salle", groups[0].name)
        assertEquals(listOf(1, 2), groups[0].tables.map { it.tableNumber })
        assertEquals(listOf(21, 22), groups[1].tables.map { it.tableNumber })
    }

    @Test
    fun aTableTheZonedPlanLeftOutComesLast() {
        val groups = groupTables(listOf(table(9), table(1, zone = "A"), table(21, zone = "B")))

        assertEquals(listOf("A", "B", null), groups.map { it.zone })
        assertEquals(9, groups[2].tables.single().tableNumber)
    }

    @Test
    fun theLabelerWritesZonesWhereThePlanHasThem() {
        val labeler = TableLabeler(mapOf(2 to "A", 21 to "B"))

        assertEquals("A-2", labeler.label(2))
        assertEquals("9", labeler.label(9))
        assertEquals("A-2", labeler.short(2))
        assertEquals("T9", labeler.short(9))
        assertEquals("T9", TableLabeler().short(9))
    }
}
