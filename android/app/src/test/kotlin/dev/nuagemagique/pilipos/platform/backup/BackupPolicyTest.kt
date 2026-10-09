package dev.nuagemagique.pilipos.platform.backup

import android.app.Application
import android.content.Context
import android.content.pm.ApplicationInfo
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.R
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.xmlpull.v1.XmlPullParser

/**
 * No cloud backup and no device transfer (PD220, plan P6): the fiscal database, the backups and
 * the keys stay on the till. The p6 gate checks the same facts statically (`plat-manifest`,
 * `plat-backup-rules`); this test checks what the platform sees at run time.
 */
@RunWith(RobolectricTestRunner::class)
@Config(application = Application::class)
class BackupPolicyTest {
    private val context: Context = ApplicationProvider.getApplicationContext()

    @Test
    fun applicationDoesNotAllowBackup() {
        val flags = context.applicationInfo.flags
        assertEquals("FLAG_ALLOW_BACKUP must be off", 0, flags and ApplicationInfo.FLAG_ALLOW_BACKUP)
    }

    @Test
    fun cloudBackupSectionExcludesEveryDomain() {
        assertEquals(EXPECTED_DOMAINS, parse().excludes.getValue("cloud-backup"))
    }

    @Test
    fun deviceTransferSectionExcludesEveryDomain() {
        assertEquals(EXPECTED_DOMAINS, parse().excludes.getValue("device-transfer"))
    }

    @Test
    fun rulesHaveExactlyTheTwoSectionsAndNoInclude() {
        val rules = parse()
        assertEquals(setOf("cloud-backup", "device-transfer"), rules.excludes.keys)
        assertEquals(0, rules.includes)
    }

    private class Rules(
        val excludes: Map<String, List<String>>,
        val includes: Int,
    )

    private fun parse(): Rules {
        val excludes = linkedMapOf<String, MutableList<String>>()
        var includes = 0
        var section: String? = null
        val xml = context.resources.getXml(R.xml.data_extraction_rules)
        try {
            var event = xml.eventType
            while (event != XmlPullParser.END_DOCUMENT) {
                if (event == XmlPullParser.START_TAG) {
                    when (xml.name) {
                        "cloud-backup", "device-transfer" -> {
                            section = xml.name
                            excludes[xml.name] = mutableListOf()
                        }
                        "exclude" -> {
                            val domain = xml.getAttributeValue(null, "domain")
                            assertTrue("<exclude> outside a section", section != null)
                            excludes.getValue(section!!).add(domain)
                        }
                        "include" -> includes++
                    }
                } else if (event == XmlPullParser.END_TAG && xml.name == section) {
                    section = null
                }
                event = xml.next()
            }
        } finally {
            xml.close()
        }
        return Rules(excludes, includes)
    }

    private companion object {
        val EXPECTED_DOMAINS = listOf("root", "file", "database", "sharedpref", "external")
    }
}
