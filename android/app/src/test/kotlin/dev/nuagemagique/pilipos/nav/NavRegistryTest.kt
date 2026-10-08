package dev.nuagemagique.pilipos.nav

import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

class NavRegistryTest {
    private val login = FakeEntry("login", NavContract.START_ROUTE)

    @Test
    fun duplicateRouteFailsNamingBothAreas() {
        val error =
            assertThrows(IllegalStateException::class.java) {
                registryOf(setOf(login, FakeEntry("alpha", "shared"), FakeEntry("beta", "shared")))
            }
        val message = error.message.orEmpty()
        assertTrue(message, "alpha" in message && "beta" in message && "shared" in message)
    }

    @Test
    fun missingStartRouteFails() {
        val error =
            assertThrows(IllegalStateException::class.java) { registryOf(setOf(FakeEntry("tables", "tables"))) }
        assertTrue(error.message.orEmpty(), "start route" in error.message.orEmpty())
    }

    @Test
    fun twoStartEntriesFail() {
        val error =
            assertThrows(IllegalStateException::class.java) {
                registryOf(setOf(login, FakeEntry("other", NavContract.START_ROUTE)))
            }
        val message = error.message.orEmpty()
        assertTrue(message, "more than one" in message && "login" in message && "other" in message)
    }

    @Test
    fun duplicateSettingsIdFails() {
        val rows = listOf(SettingsEntry("a", 1, "tables", 1), SettingsEntry("a", 2, "tables", 2))
        val error =
            assertThrows(IllegalStateException::class.java) {
                registryOf(setOf(login, FakeEntry("tables", "tables")), rows.toSet())
            }
        assertTrue(error.message.orEmpty(), "\"a\"" in error.message.orEmpty())
    }

    @Test
    fun settingsRouteWithoutEntryFails() {
        val error =
            assertThrows(IllegalStateException::class.java) {
                registryOf(setOf(login), setOf(SettingsEntry("ghost", 1, "nowhere", 1)))
            }
        assertTrue(error.message.orEmpty(), "nowhere" in error.message.orEmpty())
    }

    @Test
    fun settingsAreSortedByOrderThenId() {
        val registry =
            registryOf(
                setOf(login),
                setOf(
                    SettingsEntry("z", 1, "login", 10),
                    SettingsEntry("b", 1, "login", 20),
                    SettingsEntry("a", 1, "login", 20),
                    SettingsEntry("first", 1, "login", 5, managerOnly = false),
                ),
            )
        assertEquals(listOf("first", "z", "a", "b"), registry.settingsEntries.map { it.id })
    }

    @Test
    fun overlaysAreSortedByOrder() {
        val registry = registryOf(setOf(login), overlays = setOf(FakeOverlay(30), FakeOverlay(-1), FakeOverlay(7)))
        assertEquals(listOf(-1, 7, 30), registry.overlays.map { it.order })
    }

    @Test
    fun entriesAreSortedByArea() {
        val registry = registryOf(setOf(FakeEntry("zeta", "zeta"), login, FakeEntry("alpha", "alpha")))
        assertEquals(listOf("alpha", "login", "zeta"), registry.entries.map { it.area })
    }

    @Test
    fun anExtraEntryNeedsNoEditOfTheRegistry() {
        val registry = registryOf(setOf(login, FakeEntry("extra", "extra/{id}")))
        assertEquals(setOf("login", "extra/{id}"), registry.entries.flatMap { it.routes }.toSet())
    }

    @Test
    fun settingsRowDefaultsToManagerOnly() {
        assertTrue(SettingsEntry("x", 1, "login", 1).managerOnly)
    }
}
