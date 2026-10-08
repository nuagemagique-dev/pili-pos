package dev.nuagemagique.pilipos.platform

import dev.nuagemagique.pilipos.R
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
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
    fun platformEntryRegistersTheAboutRoute() {
        assertEquals(listOf("platform/about"), PlatformEntry().routes)
        assertEquals("platform", PlatformEntry().area)
    }
}
