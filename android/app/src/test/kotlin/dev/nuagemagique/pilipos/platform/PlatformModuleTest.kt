package dev.nuagemagique.pilipos.platform

import dev.nuagemagique.pilipos.R
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PlatformModuleTest {
    @Test
    fun aboutSettingsRowIsDeclared() {
        val row = PlatformModule.provideAboutSettings()
        assertEquals("platform.about", row.id)
        assertEquals(R.string.plat_about_title, row.titleRes)
        assertEquals(PlatformRoutes.ABOUT, row.route)
        assertEquals(900, row.order)
        assertFalse(row.managerOnly)
    }

    @Test
    fun exportSettingsRowIsDeclaredForManagersOnly() {
        val row = PlatformModule.provideExportSettings()
        assertEquals("platform.export", row.id)
        assertEquals(R.string.plat_export_title, row.titleRes)
        assertEquals(PlatformRoutes.EXPORT, row.route)
        assertEquals(910, row.order)
        assertTrue(row.managerOnly)
    }

    @Test
    fun platformEntryRegistersTheAboutRoute() {
        assertEquals("platform/about", PlatformEntry().routes.first())
        assertEquals("platform", PlatformEntry().area)
    }

    @Test
    fun platformEntryRegistersTheExportRoute() {
        assertEquals(listOf("platform/about", "platform/export"), PlatformEntry().routes)
    }

    @Test
    fun theExportRouteIsTheOneOfTheSettingsRow() {
        assertEquals("platform/export", PlatformRoutes.EXPORT)
        assertTrue(PlatformModule.provideExportSettings().route in PlatformEntry().routes)
    }
}
