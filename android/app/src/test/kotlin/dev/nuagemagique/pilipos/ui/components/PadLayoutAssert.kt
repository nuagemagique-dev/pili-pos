package dev.nuagemagique.pilipos.ui.components

import androidx.compose.ui.test.SemanticsNodeInteractionsProvider
import androidx.compose.ui.test.getUnclippedBoundsInRoot
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.unit.DpRect
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue

/**
 * UX audit C10: the numeric pad is a calculator, 7 8 9 on top, then 4 5 6, 1 2 3, and the zero on the bottom row
 * under the middle column. Reads the unclipped position of the `numpad_key_N` tags, so it holds for every screen
 * that shows a [NumPad] (login, PIN dialog, checkout, cash, amount and price dialogs...).
 */
fun SemanticsNodeInteractionsProvider.assertCalculatorLayout() {
    val box: Map<Char, DpRect> =
        ('0'..'9').associateWith { onNodeWithTag("numpad_key_$it").getUnclippedBoundsInRoot() }
    val rows = listOf("789", "456", "123")
    rows.forEach { row ->
        val keys = row.map { box.getValue(it) }
        assertEquals("row $row on one line", keys[0].top.value, keys[1].top.value, 0.5f)
        assertEquals("row $row on one line", keys[0].top.value, keys[2].top.value, 0.5f)
        assertTrue("row $row left to right", keys[0].left.value < keys[1].left.value && keys[1].left.value < keys[2].left.value)
    }
    assertTrue("4 5 6 under 7 8 9", box.getValue('4').top.value > box.getValue('7').top.value)
    assertTrue("1 2 3 under 4 5 6", box.getValue('1').top.value > box.getValue('4').top.value)
    listOf("741", "852", "963").forEach { col ->
        val keys = col.map { box.getValue(it) }
        assertEquals("column $col aligned", keys[0].left.value, keys[1].left.value, 0.5f)
        assertEquals("column $col aligned", keys[0].left.value, keys[2].left.value, 0.5f)
    }
    assertTrue("0 on the bottom row", box.getValue('0').top.value > box.getValue('1').top.value)
    assertEquals("0 under the middle column", box.getValue('2').left.value, box.getValue('0').left.value, 0.5f)
}
