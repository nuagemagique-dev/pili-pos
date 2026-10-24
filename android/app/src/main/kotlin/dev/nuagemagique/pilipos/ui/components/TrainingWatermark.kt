package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.wrapContentSize
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.nav.ShellOverlay
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import javax.inject.Inject

/** Stacking order of the watermark among the shell overlays: below the config problems banner, above the screens. */
const val TRAINING_WATERMARK_ORDER: Int = 50

/** The word across the screen. It is the same in every language, as in `App.vue`. */
private const val WATERMARK_TEXT = "TRAINING"

/** Size of the lettering (360 px in `App.vue`; 1 dp is 1 CSS px of the Vue design). */
private val WatermarkSize = 360.dp

/** Tilt of the lettering, counter-clockwise. */
private const val WATERMARK_ROTATION_DEGREES = -24f

/** The lettering is white at 6 percent. */
private const val WATERMARK_ALPHA = 0.06f

/**
 * The "TRAINING" lettering of `App.vue` (UX guide section 9): across the whole screen, tilted, almost
 * transparent. It has no pointer input, so taps go through to the screen below, and it is hidden from
 * accessibility. The caller draws it only in training mode. Test tag: `training_watermark`.
 */
@Composable
fun TrainingWatermark(modifier: Modifier = Modifier) {
    val typography = PiliTheme.typography
    val size = with(LocalDensity.current) { WatermarkSize.toSp() }
    Box(
        modifier = modifier.fillMaxSize().clipToBounds().testTag("training_watermark"),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = WATERMARK_TEXT,
            style =
                typography.body.copy(
                    fontFamily = typography.familyBrand,
                    fontSize = size,
                    fontWeight = typography.weightHeavy,
                    letterSpacing = 0.04.em,
                ),
            color = PiliTheme.colors.textPrimary.copy(alpha = WATERMARK_ALPHA),
            softWrap = false,
            maxLines = 1,
            overflow = TextOverflow.Visible,
            modifier =
                Modifier
                    .wrapContentSize(align = Alignment.Center, unbounded = true)
                    .graphicsLayer { rotationZ = WATERMARK_ROTATION_DEGREES }
                    .clearAndSetSemantics { },
        )
    }
}

/** The shell overlay that draws the [TrainingWatermark] while the till is in training mode. */
class TrainingWatermarkOverlay
    @Inject
    constructor(
        private val settings: PosSettingsStore,
    ) : ShellOverlay {
        override val order: Int = TRAINING_WATERMARK_ORDER

        @Composable
        override fun Content() {
            val training by settings.trainingMode.collectAsState()
            if (training) TrainingWatermark()
        }
    }

/** Contributes the training watermark to the shell overlays. */
@Module
@InstallIn(SingletonComponent::class)
abstract class TrainingWatermarkModule {
    @Binds
    @IntoSet
    abstract fun trainingWatermarkOverlay(overlay: TrainingWatermarkOverlay): ShellOverlay
}
