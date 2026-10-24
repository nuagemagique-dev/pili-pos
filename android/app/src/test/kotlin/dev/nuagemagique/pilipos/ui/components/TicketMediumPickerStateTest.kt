package dev.nuagemagique.pilipos.ui.components

import androidx.compose.runtime.saveable.SaverScope
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.RequestedMedium

class TicketMediumPickerStateTest {
    @Test
    fun startsClosedAndOpensForAnEmailMedium() {
        val state = TicketMediumPickerState()
        assertNull(state.pendingMedium)
        state.open(RequestedMedium.PAPER_DIGITAL)
        assertEquals(RequestedMedium.PAPER_DIGITAL, state.pendingMedium)
        state.open(RequestedMedium.DIGITAL)
        assertEquals(RequestedMedium.DIGITAL, state.pendingMedium)
        state.close()
        assertNull(state.pendingMedium)
    }

    @Test
    fun refusesThePaperMedium() {
        assertThrows(IllegalArgumentException::class.java) { TicketMediumPickerState().open(RequestedMedium.PAPER) }
    }

    private fun roundTrip(state: TicketMediumPickerState): TicketMediumPickerState? =
        with(TicketMediumPickerState.Saver) {
            val saved = SaverScope { true }.save(state)
            restore(checkNotNull(saved))
        }

    @Test
    fun theSaverKeepsTheOpenMedium() {
        assertNull(roundTrip(TicketMediumPickerState())?.pendingMedium)
        assertEquals(RequestedMedium.DIGITAL, roundTrip(TicketMediumPickerState(RequestedMedium.DIGITAL))?.pendingMedium)
        assertEquals(
            RequestedMedium.PAPER_DIGITAL,
            roundTrip(TicketMediumPickerState(RequestedMedium.PAPER_DIGITAL))?.pendingMedium,
        )
    }

    @Test
    fun digitalOnlyNeedsTheSwitchAndAReachableService() {
        assertTrue(MailStatus(configured = true, reachable = true, allowDigitalOnly = true).offersDigitalOnly())
        assertFalse(MailStatus(configured = true, reachable = false, allowDigitalOnly = true).offersDigitalOnly())
        assertFalse(MailStatus(configured = true, reachable = true, allowDigitalOnly = false).offersDigitalOnly())
    }

    @Test
    fun theNoteNeedsAConfiguredButUnreachableService() {
        assertTrue(MailStatus(configured = true, reachable = false, allowDigitalOnly = false).showsUnreachableNote())
        assertFalse(MailStatus(configured = true, reachable = true, allowDigitalOnly = false).showsUnreachableNote())
        assertFalse(MailStatus(configured = false, reachable = false, allowDigitalOnly = false).showsUnreachableNote())
    }
}
