package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.engine.EngineHelpers
import kotlinx.coroutines.CancellationException
import timber.log.Timber
import javax.inject.Inject
import javax.inject.Singleton

/** Width of the white border around the modules, in modules (the QR specification asks for four). */
const val QR_QUIET_ZONE_MODULES: Int = 4

/** Default side of one module. */
private val DefaultModuleSize = 6.dp

/** The side of a [QrCanvas] of [modules] modules per side, quiet zone included. */
fun qrSide(
    modules: Int,
    moduleSize: Dp,
): Dp = moduleSize * (modules + 2 * QR_QUIET_ZONE_MODULES)

/**
 * Draws [matrix] (rows of modules, true is dark) black on white with a four-module white border, so a scanner
 * reads it on the dark UI. The side is `(n + 8) * moduleSize`. Nothing is drawn for an empty matrix. Black and
 * white are fixed on purpose (PD218): the code must stay readable whatever the theme.
 */
@Composable
fun QrCanvas(
    matrix: List<List<Boolean>>,
    modifier: Modifier = Modifier,
    moduleSize: Dp = DefaultModuleSize,
) {
    if (matrix.isEmpty()) return
    val side = qrSide(matrix.size, moduleSize)
    Canvas(modifier = modifier.size(side).testTag("qr_canvas")) {
        drawRect(Color.White, Offset.Zero, size)
        val module = moduleSize.toPx()
        val offset = QR_QUIET_ZONE_MODULES * module
        matrix.forEachIndexed { row, cells ->
            cells.forEachIndexed { column, dark ->
                if (dark) {
                    drawRect(
                        Color.Black,
                        Offset(offset + column * module, offset + row * module),
                        Size(module, module),
                    )
                }
            }
        }
    }
}

/** The QR matrix of a text, computed by the engine (a ticket verification URL). */
@Singleton
class QrMatrices
    @Inject
    constructor(
        private val helpers: EngineHelpers,
    ) {
        /** The modules of [text]; null when the engine cannot encode it (logged). */
        @Suppress("TooGenericExceptionCaught")
        suspend fun of(text: String): List<List<Boolean>>? =
            try {
                helpers.qrMatrix(text)
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (failure: Exception) {
                Timber.w(failure, "QR matrix failed")
                null
            }
    }

/**
 * The QR code of [text]. While the matrix loads, or when it cannot be built, an empty white square of the same
 * size is drawn (the real size is known only after loading, so the placeholder has the size of the smallest QR
 * code, 21 modules).
 */
@Composable
fun QrCode(
    text: String,
    matrices: QrMatrices,
    modifier: Modifier = Modifier,
    moduleSize: Dp = DefaultModuleSize,
) {
    val matrix by produceState<List<List<Boolean>>?>(initialValue = null, text) {
        value = matrices.of(text)
    }
    val shown = matrix
    Box(modifier = modifier.testTag("qr_code")) {
        if (shown.isNullOrEmpty()) {
            Box(Modifier.size(qrSide(QR_MIN_MODULES, moduleSize)).background(Color.White))
        } else {
            QrCanvas(shown, moduleSize = moduleSize)
        }
    }
}

/** Modules per side of the smallest QR code (version 1). */
private const val QR_MIN_MODULES: Int = 21
