package dev.nuagemagique.pilipos.ui.state

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.i18n.AppLanguage
import io.mockk.coEvery
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.LockOutcome
import uniffi.pos_ffi.UserRole

/** The texts follow the logged-in user (UX audit R5): their language at login, the terminal's on the login screen. */
@RunWith(RobolectricTestRunner::class)
class UserLanguageBinderTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    private class Setup(
        val f: EngineFixture,
        val session: SessionStore,
        val settings: PosSettingsStore,
    )

    private suspend fun TestScope.setup(): Setup {
        val f = EngineFixture(this, tmp.root)
        val gateway = f.gateway(this)
        val session = SessionStore(gateway, f.events, backgroundScope)
        val settings = PosSettingsStore(context, gateway, f.events, backgroundScope)
        UserLanguageBinder(session, settings, backgroundScope)
        f.holder.open()
        runCurrent()
        return Setup(f, session, settings)
    }

    @Test
    fun aLoginTakesTheLanguageOfThatUserAndALockGoesBackToTheTerminalOne() =
        runTest {
            val s = setup()
            s.settings.setUserLanguage("111", AppLanguage.ZH)
            coEvery { s.f.engine.login("111", "1234") } returns user(UserRole.ADMIN, "111")
            coEvery { s.f.engine.lockSession() } returns LockOutcome(orderError = null)
            assertEquals(AppLanguage.FR, s.settings.language.value)

            s.session.login("111", "1234")
            runCurrent()
            assertEquals(AppLanguage.ZH, s.settings.language.value)

            s.session.lock()
            runCurrent()
            assertEquals(AppLanguage.FR, s.settings.language.value)
        }

    @Test
    fun anotherUserWithoutAChoiceFollowsTheTerminal() =
        runTest {
            val s = setup()
            s.settings.setLanguage(AppLanguage.EN)
            s.settings.setUserLanguage("111", AppLanguage.ZH)
            coEvery { s.f.engine.login("222", null) } returns user(UserRole.CASHIER, "222")

            s.session.login("222", null)
            runCurrent()

            assertEquals(AppLanguage.EN, s.settings.language.value)
        }
}
