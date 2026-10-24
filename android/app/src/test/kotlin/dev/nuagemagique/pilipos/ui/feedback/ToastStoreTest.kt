package dev.nuagemagique.pilipos.ui.feedback

import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class ToastStoreTest {
    private fun TestScope.store() = ToastStore(backgroundScope)

    @Test
    fun showAppendsToastsOldestFirstWithTheirVariant() =
        runTest {
            val store = store()
            store.show("a")
            store.show("b", ToastVariant.Error)
            assertEquals(
                listOf(Toast(1L, "a", ToastVariant.Info), Toast(2L, "b", ToastVariant.Error)),
                store.toasts.value,
            )
        }

    @Test
    fun idsIncreaseFromOne() =
        runTest {
            val store = store()
            val ids = List(3) { store.show("m$it") }
            assertEquals(listOf(1L, 2L, 3L), ids)
            store.dismiss(2L)
            assertEquals(4L, store.show("next"))
        }

    @Test
    fun aToastExpiresAfterTheDefaultTtl() =
        runTest {
            val store = store()
            store.show("a")
            advanceTimeBy(2999L)
            assertEquals(1, store.toasts.value.size)
            advanceTimeBy(2L)
            assertTrue(store.toasts.value.isEmpty())
        }

    @Test
    fun twoToastsExpireIndependently() =
        runTest {
            val store = store()
            store.show("long", ToastVariant.Warning, ttlMs = 6000L)
            advanceTimeBy(1000L)
            store.show("short", ToastVariant.Success, ttlMs = 2000L)
            advanceTimeBy(2001L)
            assertEquals(listOf("long"), store.toasts.value.map { it.message })
            advanceTimeBy(3000L)
            assertTrue(store.toasts.value.isEmpty())
        }

    @Test
    fun dismissRemovesOneToastAndIgnoresAnUnknownId() =
        runTest {
            val store = store()
            val first = store.show("a")
            store.show("b")
            store.dismiss(first)
            store.dismiss(99L)
            assertEquals(listOf("b"), store.toasts.value.map { it.message })
            advanceTimeBy(3001L)
            assertTrue(store.toasts.value.isEmpty())
        }
}
