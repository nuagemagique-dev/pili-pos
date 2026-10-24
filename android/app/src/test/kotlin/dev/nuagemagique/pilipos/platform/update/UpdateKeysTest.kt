package dev.nuagemagique.pilipos.platform.update

import android.util.Xml
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.io.File
import java.io.StringReader

@RunWith(RobolectricTestRunner::class)
class UpdateKeysTest {
    private fun parse(
        xml: String,
        debug: Boolean,
    ) = UpdateKeys.parse(
        Xml.newPullParser().apply { setInput(StringReader(xml)) },
        debug,
    )

    private val both = "<plat-update-keys><key build=\"debug\">RWdebug</key><key build=\"release\">RWrelease</key></plat-update-keys>"

    @Test
    fun theKeyOfTheBuildTypeIsPicked() {
        assertEquals("RWdebug", parse(both, debug = true).current())
        assertEquals("RWrelease", parse(both, debug = false).current())
    }

    @Test
    fun anEmptyKeyIsNoKey() {
        val xml = "<plat-update-keys><key build=\"debug\">RWdebug</key><key build=\"release\"></key></plat-update-keys>"
        assertNull(parse(xml, debug = false).current())
        assertEquals("RWdebug", parse(xml, debug = true).current())
    }

    @Test
    fun anAbsentKeyIsNoKey() {
        assertNull(parse("<plat-update-keys/>", debug = true).current())
        assertNull(parse("<plat-update-keys><key build=\"debug\">RWdebug</key></plat-update-keys>", debug = false).current())
    }

    @Test
    fun whitespaceAroundTheKeyIsIgnoredAndTheAttributeIsAFallback() {
        val xml = "<plat-update-keys><key build=\"debug\">\n  RWdebug \n</key><key build=\"release\" key=\"RWattr\"/></plat-update-keys>"
        assertEquals("RWdebug", parse(xml, debug = true).current())
        assertEquals("RWattr", parse(xml, debug = false).current())
    }

    @Test
    fun anElementWithoutABuildIsIgnored() {
        assertNull(parse("<plat-update-keys><key>RWx</key></plat-update-keys>", debug = true).current())
    }

    @Test
    fun aBrokenFileIsKeyMissing() {
        val error = assertThrows(UpdateException::class.java) { parse("<plat-update-keys><key", debug = true) }
        assertEquals(UpdateRefusalCode.KEY_MISSING, error.code)
    }

    @Test
    fun theResourceHoldsTheDevelopmentKeyForDebugAndNothingForRelease() {
        val keys = UpdateKeys.load(ApplicationProvider.getApplicationContext())
        val committed = File("provisioning/dev-minisign.pub").readLines()[1].trim()
        assertEquals(committed, keys.current())
        assertEquals(UpdateFixtures.DEV_KEY, keys.current())
        val resource =
            ApplicationProvider.getApplicationContext<android.content.Context>().resources.getXml(
                dev.nuagemagique.pilipos.R.xml.plat_update_keys,
            )
        assertNull(UpdateKeys.parse(resource, debug = false).current())
    }
}
