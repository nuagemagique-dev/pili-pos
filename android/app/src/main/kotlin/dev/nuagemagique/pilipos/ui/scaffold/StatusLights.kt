package dev.nuagemagique.pilipos.ui.scaffold

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.ui.fdm.FdmUiState
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliColors
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import timber.log.Timber
import uniffi.pos_ffi.FdmSeverity
import uniffi.pos_ffi.HubriseStatus

/** How often the two printers are checked, in milliseconds (30 s in `StatusLights.vue`). */
internal const val PRINTER_POLL_MS: Long = 30_000L

/** How often HubRise is checked, in milliseconds (60 s in `StatusLights.vue`). */
internal const val HUBRISE_POLL_MS: Long = 60_000L

/** Length of the one-shot pulse of the FDM dot when it degrades, in milliseconds (180 ms in `StatusLights.vue`). */
private const val PULSE_MS = 180L

private const val PULSE_SCALE = 1.6f
private const val NONE_ALPHA = 0.4f
private val DotSize = 10.dp

/** The colour of a device light (UX guide section 8.1): green is OK, amber a warning, red an error, grey unknown. */
internal enum class LightState {
    OK,
    WARN,
    ERR,
    NONE,
}

/** The FDM light follows the engine's reading: blocking is red, a warning or a missing connection is amber. */
internal fun fdmLight(state: FdmUiState): LightState =
    when {
        state.severity == FdmSeverity.BLOCKING -> LightState.ERR
        state.severity == FdmSeverity.WARNING || !state.connected -> LightState.WARN
        else -> LightState.OK
    }

/**
 * `hubriseLight` of `StatusLights.vue`. Red: HubRise refuses the token, the last catalog was refused, or
 * something is parked (an event or an update HubRise refused). Amber: not connected, online orders waiting over
 * 5 minutes, a menu to take over, or updates waiting after a failed send.
 */
internal fun hubriseLight(status: HubriseStatus): LightState {
    val catalog = status.catalog
    val rejected = catalog?.rejectedAt
    val applied = catalog?.appliedAt
    // The two timestamps compare as text, as in Vue.
    val catalogRefused = !rejected.isNullOrEmpty() && (applied.isNullOrEmpty() || rejected >= applied)
    return when {
        !status.health.authErrorAt.isNullOrEmpty() || catalogRefused || status.parkedEvents > 0L || status.outbox.parked > 0L ->
            LightState.ERR
        !status.health.connected ||
            status.staleOrders > 0L ||
            status.catalogAwaitingAdoption ||
            (status.outbox.pending > 0L && !status.outboxError.isNullOrEmpty()) -> LightState.WARN
        else -> LightState.OK
    }
}

/** What the lights ask of the engine; the seam of the tests. A failure of any call turns its light grey. */
internal interface LightProbe {
    /** True while the engine is ready: the only time the lights are polled. */
    val ready: StateFlow<Boolean>

    /** True when the ticket printer answers (`check_printer`). */
    suspend fun ticketPrinter(): Boolean

    /** True or false when the kitchen printer answers or not, null when none is configured (`check_kitchen_printer`). */
    suspend fun kitchenPrinter(): Boolean?

    /** The health of the HubRise link (`hubrise_status`). */
    suspend fun hubrise(): HubriseStatus
}

/** The [LightProbe] of the running app: every question is one engine command through the gateway. */
internal class GatewayLightProbe(
    private val gateway: EngineGateway,
) : LightProbe {
    override val ready: StateFlow<Boolean> get() = gateway.ready

    override suspend fun ticketPrinter(): Boolean = gateway.call { engine -> engine.checkPrinter() }

    override suspend fun kitchenPrinter(): Boolean? = gateway.call { engine -> engine.checkKitchenPrinter() }

    override suspend fun hubrise(): HubriseStatus = gateway.call { engine -> engine.hubriseStatus() }
}

/**
 * The lights of the ticket printer, the kitchen printer and HubRise, and the loops that keep them current.
 *
 * Nothing here runs by itself: the loops are suspend functions that [StatusLights] starts with a `LaunchedEffect`
 * only while it is on screen and the engine is ready, so there is no polling with the engine closed or on a
 * screen without a header. Every failure turns the light grey, never an exception.
 */
internal class DeviceLights(
    private val probe: LightProbe,
) {
    private val mutableTicket = MutableStateFlow(LightState.NONE)
    private val mutableKitchen = MutableStateFlow(LightState.NONE)
    private val mutableHubrise = MutableStateFlow(LightState.NONE)

    /** True while the engine is ready: the only time the lights are polled. */
    val ready: StateFlow<Boolean> get() = probe.ready

    /** The ticket printer. */
    val ticket: StateFlow<LightState> = mutableTicket.asStateFlow()

    /** The kitchen printer; grey when none is configured. */
    val kitchen: StateFlow<LightState> = mutableKitchen.asStateFlow()

    /** HubRise. */
    val hubrise: StateFlow<LightState> = mutableHubrise.asStateFlow()

    /** Checks the two printers every [PRINTER_POLL_MS] until cancelled. */
    suspend fun pollPrinters() {
        while (true) {
            checkPrinters()
            delay(PRINTER_POLL_MS)
        }
    }

    /** Checks HubRise every [HUBRISE_POLL_MS] until cancelled. */
    suspend fun pollHubrise() {
        while (true) {
            checkHubrise()
            delay(HUBRISE_POLL_MS)
        }
    }

    /** One check of the ticket and the kitchen printer (`checkPrinters`). */
    suspend fun checkPrinters() {
        mutableTicket.value =
            attempt { probe.ticketPrinter() }.fold(
                onSuccess = { if (it) LightState.OK else LightState.ERR },
                onFailure = { LightState.NONE },
            )
        mutableKitchen.value =
            attempt { probe.kitchenPrinter() }.fold(
                onSuccess = { reachable ->
                    when (reachable) {
                        null -> LightState.NONE
                        true -> LightState.OK
                        false -> LightState.ERR
                    }
                },
                onFailure = { LightState.NONE },
            )
    }

    /** One check of HubRise (`checkHubrise`). */
    suspend fun checkHubrise() {
        mutableHubrise.value =
            attempt { probe.hubrise() }.fold(
                onSuccess = ::hubriseLight,
                onFailure = { LightState.NONE },
            )
    }

    /** The printers are not known any more (the engine went away). */
    fun clearPrinters() {
        mutableTicket.value = LightState.NONE
        mutableKitchen.value = LightState.NONE
    }

    /** HubRise is not known any more (the engine went away or HubRise is hidden). */
    fun clearHubrise() {
        mutableHubrise.value = LightState.NONE
    }

    @Suppress("TooGenericExceptionCaught")
    private suspend fun <T> attempt(block: suspend () -> T): Result<T> =
        try {
            Result.success(block())
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            Timber.d(e, "Status light check failed")
            Result.failure(e)
        }
}

/**
 * The device lights of the header (`StatusLights.vue`, UX guide section 8.1): FDM, ticket printer, kitchen printer
 * and, while HubRise is visible, HubRise. The printers are checked every 30 s and HubRise every 60 s, only while
 * this is composed and the engine is ready. The FDM dot pulses once (180 ms) when the FDM degrades. Test tags:
 * `status_light_<fdm|ticket|kitchen|hubrise>` on a light and `status_dot_<name>_<ok|warn|err|none>` on its dot.
 */
@Composable
internal fun StatusLights(
    fdm: FdmUiState,
    lights: DeviceLights,
    hubriseVisible: Boolean,
    modifier: Modifier = Modifier,
) {
    val ready by lights.ready.collectAsState()
    LaunchedEffect(lights, ready) {
        if (ready) lights.pollPrinters() else lights.clearPrinters()
    }
    LaunchedEffect(lights, ready, hubriseVisible) {
        if (ready && hubriseVisible) lights.pollHubrise() else lights.clearHubrise()
    }
    val ticket by lights.ticket.collectAsState()
    val kitchen by lights.kitchen.collectAsState()
    val hubrise by lights.hubrise.collectAsState()
    val fdmState = fdmLight(fdm)
    Row(
        modifier = modifier.testTag("status_lights"),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        StatusLight("fdm", uiText(R.string.ui_common_light_fdm), fdmState, pulseOnDegrade = true)
        StatusLight("ticket", vueText("status_printer"), ticket)
        StatusLight("kitchen", vueText("status_kitchen"), kitchen)
        if (hubriseVisible) StatusLight("hubrise", uiText(R.string.ui_common_light_hubrise), hubrise)
    }
}

/** One light: a dot and its label in the figure font. The FDM label turns red with its light: sales are blocked. */
@Composable
private fun StatusLight(
    name: String,
    label: String,
    state: LightState,
    pulseOnDegrade: Boolean = false,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val pulse = pulseOnDegrade && rememberDegradePulse(state)
    val dotScale by animateFloatAsState(
        targetValue = if (pulse) PULSE_SCALE else 1f,
        animationSpec = tween(durationMillis = (PULSE_MS / 2).toInt()),
        label = "lightPulse",
    )
    Row(
        modifier = Modifier.testTag("status_light_$name"),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            modifier =
                Modifier
                    .size(DotSize)
                    .scale(dotScale)
                    .alpha(if (state == LightState.NONE) NONE_ALPHA else 1f)
                    .background(lightColor(c, state), CircleShape)
                    .testTag("status_dot_${name}_${state.name.lowercase()}"),
        )
        Text(
            text = label.uppercase(),
            style = t.body.copy(fontSize = t.sizeSm, fontFamily = t.familyFigures, fontWeight = t.weightBold, letterSpacing = 0.04.em),
            color = if (pulseOnDegrade && state == LightState.ERR) c.accentDanger else c.textSecondary,
            maxLines = 1,
            overflow = TextOverflow.Clip,
        )
    }
}

/** True for [PULSE_MS] after [state] turned amber or red (one pulse, UX guide section 4.2: no continuous loop). */
@Composable
private fun rememberDegradePulse(state: LightState): Boolean {
    var previous by remember { mutableStateOf(state) }
    var pulse by remember { mutableStateOf(false) }
    LaunchedEffect(state) {
        if (state != previous && (state == LightState.WARN || state == LightState.ERR)) {
            pulse = true
            delay(PULSE_MS)
            pulse = false
        }
        previous = state
    }
    return pulse
}

private fun lightColor(
    colors: PiliColors,
    state: LightState,
): Color =
    when (state) {
        LightState.OK -> colors.accentSuccess
        LightState.WARN -> colors.accentWarning
        LightState.ERR -> colors.accentDanger
        LightState.NONE -> colors.textSecondary
    }
