package dev.nuagemagique.pilipos.engine

import io.mockk.mockk
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.asCoroutineDispatcher
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PaymentPart
import uniffi.pos_ffi.VatCode
import uniffi.pos_ffi.VatLine
import uniffi.pos_ffi.VatSplitRow
import java.util.concurrent.Executors

@OptIn(ExperimentalCoroutinesApi::class)
class EngineHelpersTest {
    private val parts = listOf(PaymentPart(PaymentMethod.CASH, null))

    @Test
    fun everyHelperDelegatesToTheNativesOnTheIoDispatcher() =
        runTest {
            val executor = Executors.newSingleThreadExecutor { Thread(it, "fake-io") }
            try {
                val natives = FakeNatives()
                natives.cashDue = 1234L
                natives.vatRows = listOf(mockk<VatSplitRow>())
                natives.qr = listOf(listOf(true, false))
                val helpers = EngineHelpers(natives, executor.asCoroutineDispatcher())

                assertSame(natives.preview, helpers.settlePreview(1000L, parts))
                assertEquals(1234L, helpers.cashDueCents(1000L, parts))
                assertEquals(natives.vatRows, helpers.vatSplit(listOf(VatLine(VatCode.A, 100L, 1L))))
                assertEquals(natives.qr, helpers.qrMatrix("https://example.invalid/v"))
                assertEquals("pili-pos-android/test", helpers.posSwVersion())

                assertEquals(5, natives.threadNames.size)
                assertTrue(natives.threadNames.all { it == "fake-io" })
            } finally {
                executor.shutdown()
            }
        }

    @Test
    fun aRefusalOfTheEnginePropagates() =
        runTest {
            val natives =
                object : EngineNatives by FakeNatives() {
                    override suspend fun cashDueCents(
                        totalCents: Long,
                        parts: List<PaymentPart>,
                    ): Long = throw uniffi.pos_ffi.PosException.Coded("payment_invalid", emptyMap())
                }
            val helpers = EngineHelpers(natives, UnconfinedTestDispatcher(testScheduler))
            val failure = runCatching { helpers.cashDueCents(1L, parts) }.exceptionOrNull()
            assertEquals("payment_invalid", (failure as uniffi.pos_ffi.PosException.Coded).code)
        }
}
