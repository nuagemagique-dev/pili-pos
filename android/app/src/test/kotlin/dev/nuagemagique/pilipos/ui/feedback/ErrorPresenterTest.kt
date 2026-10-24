package dev.nuagemagique.pilipos.ui.feedback

import dev.nuagemagique.pilipos.engine.EngineState
import dev.nuagemagique.pilipos.engine.EngineUnavailableException
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.PosException

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class ErrorPresenterTest {
    private val errors = I18nTestSupport.errorTexts()
    private val noLines = PosException.Coded("order_no_lines", emptyMap())

    private fun toastStore(scope: CoroutineScope) = ToastStore(scope)

    @Test
    fun aCodedErrorGivesItsTextInFrenchAndChinese() =
        runTest {
            val presenter = ErrorPresenter(toastStore(backgroundScope), errors)
            assertEquals(errors.describe(noLines, AppLanguage.FR), presenter.text(noLines, AppLanguage.FR))
            assertEquals(errors.describe(noLines, AppLanguage.ZH), presenter.text(noLines, AppLanguage.ZH))
            assertTrue(presenter.text(noLines, AppLanguage.FR) != presenter.text(noLines, AppLanguage.ZH))
        }

    @Test
    fun aPrefixGivesTheVueForm() =
        runTest {
            val presenter = ErrorPresenter(toastStore(backgroundScope), errors)
            val plain = presenter.text(noLines, AppLanguage.EN)
            assertEquals("Print failed : $plain", presenter.text(noLines, AppLanguage.EN, "Print failed"))
            assertEquals(plain, presenter.text(noLines, AppLanguage.EN, null))
            assertEquals(
                "Print failed : ${errors.describe("order_no_lines", AppLanguage.EN)}",
                presenter.textRaw("order_no_lines", AppLanguage.EN, "Print failed"),
            )
        }

    @Test
    fun anUnavailableEngineGivesTheStartingTextAndAnUnknownFailureTheUnexpectedText() =
        runTest {
            val presenter = ErrorPresenter(toastStore(backgroundScope), errors)
            val unavailable = EngineUnavailableException(EngineState.Opening)
            assertEquals("The till is starting, please wait.", presenter.text(unavailable, AppLanguage.EN))
            assertEquals(
                "Unexpected error. Contact your technician. (java.lang.RuntimeException)",
                presenter.text(RuntimeException("boom"), AppLanguage.EN),
            )
        }

    @Test
    fun showAddsAnErrorToastWithTheTtl() =
        runTest {
            val store = toastStore(backgroundScope)
            val presenter = ErrorPresenter(store, errors)
            val id = presenter.show(noLines, AppLanguage.FR, "Edition")
            val toast = store.toasts.value.single()
            assertEquals(id, toast.id)
            assertEquals(ToastVariant.Error, toast.variant)
            assertEquals("Edition : " + errors.describe(noLines, AppLanguage.FR), toast.message)
            advanceTimeBy(4999L)
            assertEquals(1, store.toasts.value.size)
            advanceTimeBy(2L)
            assertTrue(store.toasts.value.isEmpty())
        }

    @Test
    fun showRawAddsAnErrorToastAndHonoursAGivenTtl() =
        runTest {
            val store = toastStore(backgroundScope)
            val presenter = ErrorPresenter(store, errors)
            presenter.showRaw("order_no_lines", AppLanguage.EN, ttlMs = 6000L)
            presenter.showRaw(null, AppLanguage.EN)
            assertEquals(listOf(ToastVariant.Error, ToastVariant.Error), store.toasts.value.map { it.variant })
            assertEquals(errors.describe("order_no_lines", AppLanguage.EN), store.toasts.value[0].message)
            assertEquals(errors.describe(null as String?, AppLanguage.EN), store.toasts.value[1].message)
            advanceTimeBy(5001L)
            assertEquals(1, store.toasts.value.size)
            advanceTimeBy(1000L)
            assertTrue(store.toasts.value.isEmpty())
        }
}
