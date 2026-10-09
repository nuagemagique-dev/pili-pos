package dev.nuagemagique.pilipos.ui.components

import androidx.compose.ui.test.assertHeightIsEqualTo
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertWidthIsEqualTo
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.engine.EngineNatives
import dev.nuagemagique.pilipos.engine.FakeNatives
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import io.mockk.coEvery
import io.mockk.mockk
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class QrCodeTest {
    @get:Rule
    val compose = createComposeRule()

    private val matrix: List<List<Boolean>> = List(21) { row -> List(21) { column -> (row + column) % 3 == 0 } }

    private fun matrices(natives: EngineNatives): QrMatrices = QrMatrices(EngineHelpers(natives, Dispatchers.Unconfined))

    @Test
    fun canvasHasTheSizeOfTheModulesPlusTheQuietZone() {
        compose.setContent { PiliTheme { QrCanvas(matrix, moduleSize = 6.dp) } }
        // (21 + 8) * 6 = 174
        compose.onNodeWithTag("qr_canvas").assertWidthIsEqualTo(174.dp).assertHeightIsEqualTo(174.dp)
    }

    @Test
    fun anEmptyMatrixDrawsNothing() {
        compose.setContent { PiliTheme { QrCanvas(emptyList()) } }
        compose.onNodeWithTag("qr_canvas").assertDoesNotExist()
    }

    @Test
    fun matricesAnswerWhatTheEngineComputes() =
        runBlocking {
            val natives = FakeNatives().apply { qr = matrix }
            assertEquals(matrix, matrices(natives).of("https://example.invalid/v"))
        }

    @Test
    fun aFailingEngineGivesNull() =
        runBlocking {
            val natives = mockk<EngineNatives>()
            coEvery { natives.qrMatrix(any()) } throws IllegalStateException("no qr")
            assertNull(matrices(natives).of("x"))
        }

    @Test
    fun theCodeShowsTheCanvasAfterLoading() {
        val natives = FakeNatives().apply { qr = matrix }
        compose.setContent { PiliTheme { QrCode("https://example.invalid/v", matrices(natives)) } }
        compose.waitUntil(5_000L) { compose.onAllNodesWithTag("qr_canvas").fetchSemanticsNodes().size == 1 }
        compose.onNodeWithTag("qr_canvas").assertIsDisplayed()
        compose.onNodeWithTag("qr_code").assertIsDisplayed()
    }

    @Test
    fun theCodeKeepsAnEmptySquareWhenTheEngineFails() {
        val natives = mockk<EngineNatives>()
        coEvery { natives.qrMatrix(any()) } throws IllegalStateException("no qr")
        compose.setContent { PiliTheme { QrCode("x", matrices(natives), moduleSize = 6.dp) } }
        compose.waitForIdle()
        compose.onNodeWithTag("qr_canvas").assertDoesNotExist()
        compose.onNodeWithTag("qr_code").assertWidthIsEqualTo(174.dp).assertHeightIsEqualTo(174.dp)
    }
}
