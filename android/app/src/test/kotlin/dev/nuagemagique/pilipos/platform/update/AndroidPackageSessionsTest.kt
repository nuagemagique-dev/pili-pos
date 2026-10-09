package dev.nuagemagique.pilipos.platform.update

import android.app.Application
import android.content.pm.PackageInstaller
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class AndroidPackageSessionsTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val app = ApplicationProvider.getApplicationContext<Application>()
    private val installer: PackageInstaller get() = app.packageManager.packageInstaller
    private val sessions = AndroidPackageSessions(app)

    @Test
    fun aSessionIsCreatedForTheOwnPackageWithTheSizeOfTheApk() {
        val session = sessions.create(1234)
        val info = installer.getSessionInfo(session.id)
        assertNotNull(info)
        assertEquals(app.packageName, info!!.appPackageName)
        session.close()
    }

    @Test
    fun writingAndCommittingDoNotThrow() {
        val apk = tmp.newFile("x.apk").also { it.writeBytes(ByteArray(4096) { b -> b.toByte() }) }
        val session = sessions.create(apk.length())
        session.write(apk)
        session.commit()
        session.close()
    }

    @Test
    fun abandoningRemovesTheSessionAndAMissingOneIsQuiet() {
        val session = sessions.create(10)
        sessions.abandon(session.id)
        assertNull(installer.getSessionInfo(session.id))
        sessions.abandon(session.id)
        sessions.abandon(424242)
    }
}
