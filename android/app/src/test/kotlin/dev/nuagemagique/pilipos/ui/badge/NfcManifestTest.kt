package dev.nuagemagique.pilipos.ui.badge

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Test
import org.w3c.dom.Element
import java.io.File
import javax.xml.parsers.DocumentBuilderFactory

/**
 * Lane 5, manifest (plan L5-D13): the NFC permission is declared, and the NFC feature is NOT required, otherwise
 * Play and the package installer would filter out hardware without native NFC (the Sunmi D3 Pro main unit).
 */
class NfcManifestTest {
    private val manifest: Element =
        DocumentBuilderFactory
            .newInstance()
            .apply { isNamespaceAware = true }
            .newDocumentBuilder()
            .parse(File("src/main/AndroidManifest.xml"))
            .documentElement

    private fun named(
        tag: String,
        name: String,
    ): Element? {
        val nodes = manifest.getElementsByTagName(tag)
        return (0 until nodes.length)
            .map { nodes.item(it) as Element }
            .firstOrNull { it.getAttributeNS(ANDROID, "name") == name }
    }

    @Test
    fun theNfcPermissionIsDeclared() {
        assertNotNull(named("uses-permission", "android.permission.NFC"))
    }

    @Test
    fun theNfcFeatureIsDeclaredAndNotRequired() {
        val feature = named("uses-feature", "android.hardware.nfc")
        assertNotNull("uses-feature android.hardware.nfc is missing", feature)
        assertEquals("false", feature!!.getAttributeNS(ANDROID, "required"))
    }

    private companion object {
        const val ANDROID = "http://schemas.android.com/apk/res/android"
    }
}
