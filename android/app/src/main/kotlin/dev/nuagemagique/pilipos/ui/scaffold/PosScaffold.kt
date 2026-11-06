package dev.nuagemagique.pilipos.ui.scaffold

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.lifecycle.viewmodel.compose.viewModel
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.feature.order.TablePickerDialog
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.ui.components.IdleLockBanner
import dev.nuagemagique.pilipos.ui.components.PinDialog
import dev.nuagemagique.pilipos.ui.components.PinMode
import dev.nuagemagique.pilipos.ui.components.PinPadTexts
import dev.nuagemagique.pilipos.ui.components.TrainingBanner
import dev.nuagemagique.pilipos.ui.di.rememberEntryPoint
import dev.nuagemagique.pilipos.ui.fdm.FdmBlockingVeil
import dev.nuagemagique.pilipos.ui.fdm.FdmUiState
import dev.nuagemagique.pilipos.ui.format.PosDateTime
import dev.nuagemagique.pilipos.ui.text.PosUiRoot
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import java.time.Instant

/** How often the clock of the header is refreshed, in milliseconds (10 s in `AppHeader.vue`). */
private const val CLOCK_TICK_MS = 10_000L

/**
 * The frame of every staff screen: the header, the training banner and the idle lock countdown under it and the [content], with the FDM
 * blocking veil over the content of the sales screens (PD209: Reports, Settings and the online orders stay
 * reachable) and the [overlay] slot above everything for the dialogs of the screen (PD214).
 *
 * It takes its dependencies from [ScaffoldEntryPoint], builds [HeaderViewModel] with `viewModel { }` and performs the
 * moves the header asks for through [navigator]. It renders with the engine closed (no user, grey lights) and never
 * navigates when it starts. The theme and the texts come from [PosUiRoot], so a screen needs no wrapper of its own.
 */
@Composable
fun PosScaffold(
    area: HeaderArea,
    navigator: AppNavigator,
    modifier: Modifier = Modifier,
    overlay: @Composable BoxScope.() -> Unit = {},
    content: @Composable () -> Unit,
) {
    val entry = rememberEntryPoint<ScaffoldEntryPoint>()
    val model = viewModel(key = "header_" + area.name) { HeaderViewModel(entry, area) }
    val header by model.state.collectAsState()
    val fdm by entry.fdm().state.collectAsState()
    val training by entry.settings().trainingMode.collectAsState()
    val eft by entry.eft().overlay.collectAsState()
    val lights = remember(entry) { DeviceLights(GatewayLightProbe(entry.gateway())) }
    val sheet = remember { DeviceSheetState() }
    val fdmCheckedAt by entry.fdm().checkedAt.collectAsState()
    val scope = rememberCoroutineScope()
    LaunchedEffect(model) {
        model.effects.collect {
            when (it) {
                is HeaderEffect.Navigate -> navigator.navigate(it.route)
                HeaderEffect.ToLogin -> navigator.toLogin()
            }
        }
    }
    val actions = headerActions(model)
    PosUiRoot {
        PosScaffoldContent(
            header = header,
            actions = actions,
            fdm = fdm,
            onCheckFdm = { scope.launch { entry.fdm().checkNow() } },
            training = training,
            clock = rememberClockText(),
            modifier = modifier,
            lights = {
                StatusLights(fdm, lights, header.hubriseVisible, eftPhase = eft.phase, onOpenDevice = { sheet.openKey = it })
            },
            deviceSheet = {
                DeviceStatusSheetHost(
                    sheet = sheet,
                    fdm = fdm,
                    fdmCheckedAt = fdmCheckedAt,
                    lights = lights,
                    hubriseVisible = header.hubriseVisible,
                    onCheckFdm = { scope.launch { entry.fdm().checkNow() } },
                )
            },
            overlay = overlay,
            content = content,
        )
    }
}

/** What the header's buttons do, all through [model]. */
private fun headerActions(model: HeaderViewModel): HeaderActions =
    HeaderActions(
        onTables = model::goTables,
        onNewTakeaway = model::newTakeaway,
        onOnlineOrders = model::goOnlineOrders,
        onReports = model::goReports,
        onSettings = model::goSettings,
        onLock = model::lock,
        onOpenPin = model::openPin,
        onMoveTable = model::openTablePicker,
        onPickTable = model::pickTable,
        onCloseTablePicker = model::closeTablePicker,
        onAddCustomer = model::openCustomer,
        onCustomerName = model::submitCustomerName,
        onCustomerPhone = model::submitCustomerPhone,
        onCloseCustomer = model::closeCustomer,
        pin =
            HeaderPinActions(
                onSubmit = model::submitPin,
                describeError = model::describeError,
                onDone = model::closePin,
                onCancel = model::closePin,
            ),
    )

/**
 * The stateless scaffold: [PosScaffold] with its state and actions given. The veil is drawn over [content] when
 * `header.area.isSales` is true and [fdm] is blocking; the own-PIN dialog, the table picker of the table chip, the
 * [deviceSheet] of a status light and then [overlay] come above the header.
 * Test tags: `scaffold_content` (the box over which the veil lies).
 */
@Composable
@Suppress("LongParameterList") // The scaffold's whole state and its slots; a holder class would only rename them.
internal fun PosScaffoldContent(
    header: HeaderUiState,
    actions: HeaderActions,
    fdm: FdmUiState,
    onCheckFdm: () -> Unit,
    training: Boolean,
    clock: String,
    modifier: Modifier = Modifier,
    lights: @Composable () -> Unit = {},
    deviceSheet: @Composable BoxScope.() -> Unit = {},
    overlay: @Composable BoxScope.() -> Unit = {},
    content: @Composable () -> Unit,
) {
    Box(modifier = modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary)) {
        Column(Modifier.fillMaxSize()) {
            AppHeader(header, actions, clock, lights = lights)
            if (training) TrainingBanner()
            header.idleWarning?.let { IdleLockBanner(it) }
            Box(Modifier.weight(1f).fillMaxSize().testTag("scaffold_content")) {
                content()
                if (header.area.isSales) FdmBlockingVeil(fdm, onCheckNow = onCheckFdm)
            }
        }
        if (header.pinOpen) OwnPinDialog(header.userName, actions.pin)
        val picker = header.tablePicker
        val from = header.tableNumber
        if (picker != null && from != null) {
            TablePickerDialog(from, picker, onPick = actions.onPickTable, onCancel = actions.onCloseTablePicker)
        }
        when (header.customerStep) {
            CustomerStep.NAME ->
                CustomerNameDialog(header.customerName.orEmpty(), actions.onCustomerName, actions.onCloseCustomer)
            CustomerStep.PHONE ->
                CustomerPhoneDialog(header.customerPhone, actions.onCustomerPhone, actions.onCloseCustomer)
            null -> Unit
        }
        deviceSheet()
        overlay()
    }
}

/** The own-PIN dialog (`PinDialog.vue` without `firstLogin`): the new PIN typed twice, saved by [HeaderPinActions.onSubmit]. */
@Composable
private fun OwnPinDialog(
    userName: String,
    actions: HeaderPinActions,
) {
    val texts =
        PinPadTexts(
            title = vueText("pin_enter"),
            hint = userName,
            enterPrompt = vueText("pin_enter"),
            confirmPrompt = vueText("pin_confirm"),
            mismatch = vueText("pin_mismatch"),
            cancel = vueText("back"),
            clearLabel = uiText(R.string.ui_common_numpad_clear),
            backspaceLabel = uiText(R.string.ui_common_numpad_backspace),
        )
    PinDialog(
        mode = PinMode.Choose,
        texts = texts,
        onSubmit = actions.onSubmit,
        describeError = actions.describeError,
        onDone = actions.onDone,
        onCancel = actions.onCancel,
    )
}

/** The time of the header in Belgian time (`HH:mm`), refreshed every 10 s while the header is composed. */
@Composable
private fun rememberClockText(): String {
    var now by remember { mutableStateOf(Instant.now()) }
    LaunchedEffect(Unit) {
        while (true) {
            delay(CLOCK_TICK_MS)
            now = Instant.now()
        }
    }
    return PosDateTime.formatTime(now)
}
