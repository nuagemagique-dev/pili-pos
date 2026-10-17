@file:Suppress("TooManyFunctions")

package dev.nuagemagique.pilipos.feature.settings

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import dagger.Lazy
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.feature.settings.alerts.NumberEntryActions
import dev.nuagemagique.pilipos.feature.settings.alerts.NumberEntryDialog
import dev.nuagemagique.pilipos.feature.settings.alerts.OwnerAlertsCallbacks
import dev.nuagemagique.pilipos.feature.settings.alerts.OwnerAlertsSection
import dev.nuagemagique.pilipos.feature.settings.alerts.OwnerAlertsViewModel
import dev.nuagemagique.pilipos.feature.settings.courses.CoursesCallbacks
import dev.nuagemagique.pilipos.feature.settings.courses.CoursesSection
import dev.nuagemagique.pilipos.feature.settings.courses.CoursesViewModel
import dev.nuagemagique.pilipos.feature.settings.vouchers.GiftVoucherCallbacks
import dev.nuagemagique.pilipos.feature.settings.vouchers.GiftVoucherSection
import dev.nuagemagique.pilipos.feature.settings.vouchers.GiftVoucherViewModel
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.nav.NavRegistry
import dev.nuagemagique.pilipos.nav.SettingsEntry
import dev.nuagemagique.pilipos.ui.components.ConfirmDialog
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PinDialog
import dev.nuagemagique.pilipos.ui.components.PinMode
import dev.nuagemagique.pilipos.ui.components.PinPadTexts
import dev.nuagemagique.pilipos.ui.di.rememberEntryPoint
import dev.nuagemagique.pilipos.ui.format.PosDateTime
import dev.nuagemagique.pilipos.ui.scaffold.HeaderArea
import dev.nuagemagique.pilipos.ui.scaffold.PosScaffold
import dev.nuagemagique.pilipos.ui.state.ProductLabel
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.DeliveryView
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.PosSettings
import uniffi.pos_ffi.SwitchChange
import uniffi.pos_ffi.User

/** Width from which the three columns of `SettingsView.vue` are drawn; narrower, the panels stack in one column. */
private val WideLayoutMin = 900.dp

/** Tallest the ticket deliveries panel grows in the third column before its list scrolls. */
private val DeliveriesMaxHeight = 420.dp

/** Height of a row of the settings rows section. */
private val RowHeight = 64.dp

/** The latest HubRise switch changes shown (`log.slice(0, 5)`). */
private const val LOG_SHOWN = 5

/** The languages in the order of the Vue buttons (French first: it is the default). */
private val Languages = listOf(AppLanguage.FR, AppLanguage.ZH, AppLanguage.EN)

/** Everything the operator can do on the settings screen, except the HubRise panel ([HubriseCallbacks]). */
internal data class SettingsCallbacks(
    val onToggleTraining: () -> Unit = {},
    val onLanguage: (AppLanguage) -> Unit = {},
    val onProductLabel: (ProductLabel) -> Unit = {},
    val onTablePlanInput: (String) -> Unit = {},
    val onSaveTablePlan: () -> Unit = {},
    val onRow: (SettingsEntry) -> Unit = {},
    val onNewName: (String) -> Unit = {},
    val onNewInsz: (String) -> Unit = {},
    val onNewRole: (NewRole) -> Unit = {},
    val onAddEmployee: () -> Unit = {},
    val onPin: (User) -> Unit = {},
    val onDeactivate: (String) -> Unit = {},
    val onToggleHubrise: () -> Unit = {},
    val onReloadDeliveries: () -> Unit = {},
)

/** What the PIN dialogs of the screen call (the view model's PIN flow). */
internal data class SettingsPinActions(
    val onChosen: suspend (String) -> Unit = {},
    val onManagerPin: suspend (String) -> Unit = {},
    val describeError: (Throwable) -> String = { it.message.orEmpty() },
    val onDone: () -> Unit = {},
    val onCancel: () -> Unit = {},
)

/** What the confirmation dialog of a deactivation calls. */
internal data class DeactivateActions(
    val onConfirm: (String) -> Unit = {},
    val onCancel: () -> Unit = {},
)

/**
 * The settings route (`SettingsView.vue`): builds [SettingsViewModel] and [HubriseViewModel] from the application
 * graph and draws [SettingsContent] in the staff scaffold. The rows are the settings rows of [registry] the role
 * may see; a tap opens `row.route` through [navigator] and nothing else (UI-PD308). The screen renders with the
 * engine closed, never navigates when it starts and asks the mail status once on open. The root carries the tag
 * `placeholder_settings` (PD204).
 */
@Composable
internal fun SettingsRoute(
    navigator: AppNavigator,
    registry: Lazy<NavRegistry>,
) {
    val entry = rememberEntryPoint<SettingsFeatureEntryPoint>()
    val model = viewModel { SettingsViewModel(entry) }
    val hubriseModel = viewModel { HubriseViewModel(entry) }
    val alertsModel = viewModel { OwnerAlertsViewModel(entry) }
    val coursesModel = viewModel { CoursesViewModel(entry) }
    val vouchersModel = viewModel { GiftVoucherViewModel(entry) }
    val loaded by model.state.collectAsState()
    val hubrise by hubriseModel.state.collectAsState()
    val alerts by alertsModel.state.collectAsState()
    val courses by coursesModel.state.collectAsState()
    val vouchers by vouchersModel.state.collectAsState()
    val pending by entry.webshop().pendingCount.collectAsState()
    LaunchedEffect(hubriseModel) {
        hubriseModel.effects.collect {
            when (it) {
                is HubriseEffect.ChannelSaved -> entry.webshop().fetchOrders()
            }
        }
    }
    val rows = remember(loaded.isManager) { visibleRows(registry.get().settingsEntries, loaded.isManager) }
    val state = loaded.copy(rows = rows)
    val callbacks = settingsCallbacks(model, navigator)
    val pinActions = pinActionsOf(model)
    val hubriseCallbacks = hubriseCallbacksOf(hubriseModel)
    val alertsCallbacks = alertsCallbacksOf(alertsModel)
    val coursesCallbacks = CoursesCallbacks(coursesModel::toggleEnabled, coursesModel::setCourse, coursesModel::save)
    val vouchersCallbacks =
        GiftVoucherCallbacks(vouchersModel::toggleEnabled, vouchersModel::setValidity, vouchersModel::setMax, vouchersModel::save)
    val connectActions = HubriseConnectActions(onSubmit = hubriseModel::submitCode, onCancel = hubriseModel::cancelConnect)
    val qr = remember(entry) { entry.qrMatrices() }
    LaunchedEffect(model) { entry.delivery().refreshMailStatus() }
    PosScaffold(
        HeaderArea.SETTINGS,
        navigator,
        modifier = Modifier.testTag("placeholder_settings"),
        overlay = {
            SettingsDialogs(
                state = state,
                pinActions = pinActions,
                deactivate = DeactivateActions(onConfirm = model::deactivate, onCancel = model::cancelDeactivate),
            )
            HubriseDialogs(
                state = hubrise,
                matrices = qr,
                connect = connectActions,
                onConfirmDisconnect = hubriseModel::confirmDisconnect,
                onCancelDisconnect = hubriseModel::cancelDisconnect,
            )
            alerts.entry?.let { number ->
                NumberEntryDialog(number, alertsActionsOf(alertsModel))
            }
        },
    ) {
        SettingsContent(
            state,
            callbacks,
            hubriseSection = { HubriseSection(hubrise, pending, hubriseCallbacks) },
            alertsSection = { OwnerAlertsSection(alerts, alertsCallbacks) },
            coursesSection = {
                Column(verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd)) {
                    CoursesSection(courses, coursesCallbacks)
                    GiftVoucherSection(vouchers, vouchersCallbacks)
                }
            },
        )
    }
}

/** What the screen calls on [model]; a row tap opens `row.route` on [navigator] (UI-PD308). */
private fun settingsCallbacks(
    model: SettingsViewModel,
    navigator: AppNavigator,
): SettingsCallbacks =
    SettingsCallbacks(
        onToggleTraining = model::toggleTraining,
        onLanguage = model::setLanguage,
        onProductLabel = model::setProductLabel,
        onTablePlanInput = model::setTablePlanInput,
        onSaveTablePlan = model::saveTablePlan,
        onRow = { navigator.navigate(it.route) },
        onNewName = model::setNewName,
        onNewInsz = model::setNewInsz,
        onNewRole = model::setNewRole,
        onAddEmployee = model::addEmployee,
        onPin = model::openPin,
        onDeactivate = model::deactivate,
        onToggleHubrise = model::toggleHubrise,
        onReloadDeliveries = model::reloadDeliveries,
    )

/** The PIN flow of [model] as the PIN dialogs call it. */
private fun pinActionsOf(model: SettingsViewModel): SettingsPinActions =
    SettingsPinActions(
        onChosen = model::submitChosenPin,
        onManagerPin = model::confirmWithManagerPin,
        describeError = model::describe,
        onDone = model::onPinDone,
        onCancel = model::closePin,
    )

/** What the owner alerts panel calls on [model]. */
private fun alertsCallbacksOf(model: OwnerAlertsViewModel): OwnerAlertsCallbacks =
    OwnerAlertsCallbacks(
        onToggleEnabled = model::toggleEnabled,
        onToggleKind = model::toggleKind,
        onLanguage = model::setLanguage,
        onLabel = model::setLabel,
        onEdit = model::openEntry,
        onSave = model::save,
        onTest = model::sendTest,
    )

/** What the number dialog of the owner alerts panel calls on [model]. */
private fun alertsActionsOf(model: OwnerAlertsViewModel): NumberEntryActions =
    NumberEntryActions(
        onKey = model::pressKey,
        onMinus = model::toggleMinus,
        onConfirm = model::confirmEntry,
        onCancel = model::cancelEntry,
    )

/** What the HubRise panel calls on [model]. */
private fun hubriseCallbacksOf(model: HubriseViewModel): HubriseCallbacks =
    HubriseCallbacks(
        onConnect = model::connect,
        onDisconnect = model::requestDisconnect,
        onSync = model::syncCatalog,
        onToggleAdoption = model::toggleAdoption,
        onAdopt = model::adoptCatalog,
        onChannelKind = model::setChannelKind,
        onChannelCharge = model::setChannelChargePolicy,
        onNewChannel = model::setNewChannel,
        onAddChannel = model::addChannel,
    )

/**
 * The settings screen, stateless: three columns for 1920 x 1080 dp (training, language, product label and the
 * rows of other features; the employees of a manager; the information, the HubRise switch, the ticket deliveries
 * and the HubRise panel), or one scrolling column on a narrow window. Test tags: `settings_content`,
 * `settings_training`, `settings_language`, `settings_labels`, `settings_rows`, `settings_row_<id>`,
 * `settings_workers`, `settings_info`, `integration_switch`, `settings_deliveries`.
 */
@Composable
internal fun SettingsContent(
    state: SettingsUiState,
    callbacks: SettingsCallbacks,
    modifier: Modifier = Modifier,
    hubriseSection: @Composable () -> Unit = {},
    alertsSection: @Composable () -> Unit = {},
    coursesSection: (@Composable () -> Unit)? = null,
) {
    BoxWithConstraints(modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary).testTag("settings_content")) {
        if (maxWidth >= WideLayoutMin) {
            WideLayout(state, callbacks, hubriseSection, alertsSection, coursesSection)
        } else {
            NarrowLayout(state, callbacks, hubriseSection, alertsSection, coursesSection)
        }
    }
}

@Composable
private fun WideLayout(
    state: SettingsUiState,
    callbacks: SettingsCallbacks,
    hubriseSection: @Composable () -> Unit,
    alertsSection: @Composable () -> Unit,
    coursesSection: (@Composable () -> Unit)?,
) {
    val gap = PiliTheme.dimens.spaceLg
    Row(Modifier.fillMaxSize().padding(gap), horizontalArrangement = Arrangement.spacedBy(gap)) {
        Column(Modifier.weight(1f).fillMaxHeight(), verticalArrangement = Arrangement.spacedBy(gap)) {
            TrainingPanel(state.training, callbacks.onToggleTraining)
            LanguagePanel(state.language, callbacks.onLanguage)
            ProductLabelPanel(state.productLabel, callbacks.onProductLabel)
            RowsPanel(state.rows, callbacks.onRow, Modifier.weight(1f), scroll = true)
        }
        if (state.isManager) {
            Column(Modifier.weight(1f).fillMaxHeight(), verticalArrangement = Arrangement.spacedBy(gap)) {
                EmployeesPanel(state, callbacks, Modifier.weight(1f), scrollList = true)
                if (coursesSection != null) CoursesSlot(coursesSection)
            }
        }
        Column(
            Modifier.weight(1f).fillMaxHeight().verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(gap),
        ) {
            if (state.isManager) TablePlanPanel(state.tablePlan, callbacks)
            if (state.isManager) alertsSection()
            InfoPanel(state.info)
            IntegrationPanel(state.hubrise, callbacks.onToggleHubrise)
            if (state.hubrise.visible) hubriseSection()
            DeliveriesPanel(state.deliveries, state.deliveriesLoading, state.mailStatus, callbacks.onReloadDeliveries)
        }
    }
}

@Composable
private fun NarrowLayout(
    state: SettingsUiState,
    callbacks: SettingsCallbacks,
    hubriseSection: @Composable () -> Unit,
    alertsSection: @Composable () -> Unit,
    coursesSection: (@Composable () -> Unit)?,
) {
    val gap = PiliTheme.dimens.spaceMd
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(gap),
        verticalArrangement = Arrangement.spacedBy(gap),
    ) {
        RowsPanel(state.rows, callbacks.onRow, Modifier, scroll = false)
        TrainingPanel(state.training, callbacks.onToggleTraining)
        LanguagePanel(state.language, callbacks.onLanguage)
        ProductLabelPanel(state.productLabel, callbacks.onProductLabel)
        if (state.isManager) TablePlanPanel(state.tablePlan, callbacks)
        if (state.isManager) alertsSection()
        if (state.isManager) EmployeesPanel(state, callbacks, Modifier, scrollList = false)
        if (state.isManager && coursesSection != null) coursesSection()
        InfoPanel(state.info)
        IntegrationPanel(state.hubrise, callbacks.onToggleHubrise)
        if (state.hubrise.visible) hubriseSection()
        DeliveriesPanel(state.deliveries, state.deliveriesLoading, state.mailStatus, callbacks.onReloadDeliveries)
    }
}

/** The courses panel under the employees: at most half of the column, scrolling inside when it needs more. */
@Composable
private fun CoursesSlot(coursesSection: @Composable () -> Unit) {
    Column(Modifier.fillMaxWidth().heightIn(max = CoursesMaxHeight).verticalScroll(rememberScrollState())) { coursesSection() }
}

private val CoursesMaxHeight = 480.dp

// ── Column 1: training, language, product label, rows ─────────────

@Composable
private fun TrainingPanel(
    training: Boolean,
    onToggle: () -> Unit,
) {
    Panel(
        modifier = Modifier.testTag("settings_training"),
        borderColor = if (training) PiliTheme.colors.accentTraining else PiliTheme.colors.border,
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            PanelTitle(vueText("training_mode"), Modifier.weight(1f))
            TrainingToggle(
                on = training,
                text = vueText(if (training) "settings_training_on" else "settings_training_off"),
                onClick = onToggle,
                modifier = Modifier.testTag("settings_training_toggle"),
            )
        }
        PanelText(vueText("settings_training_effect"))
    }
}

@Composable
private fun LanguagePanel(
    language: AppLanguage,
    onLanguage: (AppLanguage) -> Unit,
) {
    Panel(modifier = Modifier.testTag("settings_language")) {
        PanelTitle(vueText("language"))
        SegmentedControl(
            options = Languages.map { Triple(it, it.code, vueText("settings_lang_name_" + it.code)) },
            selected = language,
            onSelect = onLanguage,
            tag = "settings_lang",
        )
        PanelText(vueText("settings_language_note"))
    }
}

@Composable
private fun ProductLabelPanel(
    label: ProductLabel,
    onLabel: (ProductLabel) -> Unit,
) {
    Panel(modifier = Modifier.testTag("settings_labels")) {
        PanelTitle(vueText("settings_product_label"))
        SegmentedControl(
            options = ProductLabel.entries.map { Triple(it, it.code, vueText("settings_product_label_" + it.code)) },
            selected = label,
            onSelect = onLabel,
            tag = "settings_label",
        )
        PanelText(vueText("settings_product_label_note"))
    }
}

/** The table plan of a manager: the plan in use, a field and its save button (`.table-plan`). */
@Composable
private fun TablePlanPanel(
    plan: TablePlanState,
    callbacks: SettingsCallbacks,
) {
    Panel(modifier = Modifier.testTag("settings_tables")) {
        PanelTitle(vueText("settings_tables"))
        if (plan.plan != null) {
            Text(
                vueText("settings_tables_current", "count" to plan.count, "plan" to plan.plan),
                style = PiliTheme.typography.body,
                color = PiliTheme.colors.textPrimary,
                modifier = Modifier.testTag("settings_tables_current"),
            )
        }
        Row(
            Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            SettingsTextField(
                value = plan.input,
                onValueChange = callbacks.onTablePlanInput,
                placeholder = vueText("settings_tables_placeholder"),
                modifier = Modifier.weight(1f).testTag("settings_tables_input"),
                mono = true,
                onDone = callbacks.onSaveTablePlan,
            )
            ActionButton(vueText("settings_tables_save"), callbacks.onSaveTablePlan, Modifier.testTag("settings_tables_save"))
        }
        if (plan.error != null) {
            Text(
                plan.error,
                style = PiliTheme.typography.body,
                color = PiliTheme.colors.accentDanger,
                modifier = Modifier.testTag("settings_tables_error"),
            )
        }
        PanelText(vueText("settings_tables_note"))
    }
}

/**
 * The rows the other features contribute (UI-PD308): one button per row, in the order given, tagged
 * `settings_row_<id>`. Nothing here knows what a route is: a tap hands the row to [onRow].
 */
@Composable
private fun RowsPanel(
    rows: List<SettingsEntry>,
    onRow: (SettingsEntry) -> Unit,
    modifier: Modifier,
    scroll: Boolean,
) {
    Panel(modifier = modifier.testTag("settings_rows"), scroll = scroll) {
        PanelTitle(uiText(R.string.ui_settings_rows_title))
        rows.forEach { row -> SettingsRowButton(row, onRow) }
    }
}

@Composable
private fun SettingsRowButton(
    row: SettingsEntry,
    onRow: (SettingsEntry) -> Unit,
) {
    val colors = PiliTheme.colors
    Box(
        Modifier
            .fillMaxWidth()
            .heightIn(min = RowHeight)
            .background(colors.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusMd))
            .clickable { onRow(row) }
            .padding(horizontal = PiliTheme.dimens.spaceMd)
            .testTag("settings_row_" + row.id),
        contentAlignment = Alignment.CenterStart,
    ) {
        Text(
            uiText(row.titleRes),
            style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold),
            color = colors.textPrimary,
            maxLines = 2,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

// ── Column 2: employees ───────────────────────────────────────────

@Composable
private fun EmployeesPanel(
    state: SettingsUiState,
    callbacks: SettingsCallbacks,
    modifier: Modifier,
    scrollList: Boolean,
) {
    Panel(modifier = modifier.testTag("settings_workers")) {
        PanelTitle(vueText("employees"))
        val listModifier = if (scrollList) Modifier.weight(1f).verticalScroll(rememberScrollState()) else Modifier
        Column(listModifier.fillMaxWidth()) {
            state.users.forEach { user -> WorkerRow(user, state.isSelf(user.insz), callbacks) }
        }
        AddEmployeeForm(state.form, callbacks)
    }
}

@Composable
private fun WorkerRow(
    user: User,
    self: Boolean,
    callbacks: SettingsCallbacks,
) {
    val colors = PiliTheme.colors
    Column(Modifier.fillMaxWidth().testTag("worker_" + user.insz)) {
        Row(
            Modifier.fillMaxWidth().heightIn(min = 72.dp),
            horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Avatar(avatarInitial(user.name))
            Column(Modifier.weight(1f)) {
                Text(
                    user.name,
                    style =
                        PiliTheme.typography.body.copy(
                            fontWeight = PiliTheme.typography.weightBold,
                            fontSize = PiliTheme.typography.sizeLg,
                        ),
                    color = colors.textPrimary,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    vueText("role_" + user.role.name.lowercase()) + " · " +
                        vueText(if (user.hasPin) "settings_pin_set" else "settings_pin_missing"),
                    style = PiliTheme.typography.body,
                    color = if (user.hasPin) colors.textSecondary else colors.accentWarning,
                    modifier = Modifier.testTag("worker_" + user.insz + "_meta"),
                )
            }
            GapRow {
                ActionButton(vueText("settings_pin_button"), { callbacks.onPin(user) }, Modifier.testTag("worker_" + user.insz + "_pin"))
                if (!self) {
                    ActionButton(
                        vueText("deactivate"),
                        { callbacks.onDeactivate(user.insz) },
                        Modifier.testTag("worker_" + user.insz + "_deactivate"),
                    )
                }
            }
        }
        Hairline()
    }
}

@Composable
private fun AddEmployeeForm(
    form: NewEmployeeForm,
    callbacks: SettingsCallbacks,
) {
    Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        SectionLabel(vueText("add"))
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
            SettingsTextField(
                value = form.name,
                onValueChange = callbacks.onNewName,
                placeholder = vueText("settings_name_placeholder"),
                modifier = Modifier.weight(1f).testTag("settings_new_name"),
            )
            SettingsTextField(
                value = form.insz,
                onValueChange = callbacks.onNewInsz,
                placeholder = vueText("settings_insz_hint"),
                modifier = Modifier.weight(1f).testTag("settings_new_insz"),
                mono = true,
                numeric = true,
            )
        }
        SegmentedControl(
            options = NewRole.entries.map { Triple(it, it.code, vueText(it.labelKey)) },
            selected = form.role,
            onSelect = callbacks.onNewRole,
            tag = "settings_role",
            height = RoleSegmentHeight,
        )
        if (form.error != null) {
            Text(
                form.error,
                style = PiliTheme.typography.body,
                color = PiliTheme.colors.accentDanger,
                modifier = Modifier.testTag("settings_add_error"),
            )
        }
        ActionButton(
            vueText("settings_add_employee"),
            callbacks.onAddEmployee,
            Modifier.fillMaxWidth().heightIn(min = RowHeight).testTag("settings_add"),
        )
    }
}

// ── Column 3: information, HubRise switch, deliveries ─────────────

@Composable
private fun InfoPanel(info: PosSettings?) {
    Panel(modifier = Modifier.testTag("settings_info")) {
        PanelTitle(vueText("info"))
        if (info != null) {
            Column(Modifier.fillMaxWidth()) {
                InfoRow(vueText("settings_pos_id_label"), info.posId)
                InfoRow(vueText("settings_terminal_label"), info.terminalId)
                InfoRow(vueText("settings_restaurant_label"), info.restaurantName)
                InfoRow(vueText("settings_fdm_label"), info.fdmUrl)
                InfoRow(vueText("settings_printer_label"), info.printerAddress ?: vueText("printer_none"))
                InfoRow(uiText(R.string.ui_eft_terminal_label), paymentTerminalLabel(info.paymentTerminal))
            }
        }
    }
}

/** The HubRise switch (`IntegrationSwitch.vue`): its state, the button, the settle-first note and the log. */
@Composable
private fun IntegrationPanel(
    hubrise: HubriseSwitchState,
    onToggle: () -> Unit,
) {
    val colors = PiliTheme.colors
    Panel(modifier = Modifier.testTag("integration_switch")) {
        PanelTitle(vueText("integrations_title"))
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            GapRow(Modifier.weight(1f)) {
                Text("HubRise", style = PiliTheme.typography.title.copy(fontSize = PiliTheme.typography.sizeLg), color = colors.textPrimary)
                StatusPill(
                    vueText(if (hubrise.enabled) "integrations_on" else "integrations_off"),
                    if (hubrise.enabled) colors.accentSuccess else colors.textSecondary,
                    Modifier.testTag("integration_pill"),
                )
            }
            ActionButton(
                vueText(if (hubrise.enabled) "integrations_turn_off" else "integrations_turn_on"),
                onToggle,
                Modifier.testTag("integration_toggle"),
                enabled = !hubrise.busy,
                variant = if (hubrise.enabled) PiliButtonVariant.Secondary else PiliButtonVariant.Primary,
            )
        }
        PanelText(vueText("integrations_hubrise_text"))
        if (hubrise.onlineOrdersOpen > 0L) {
            PanelText(
                vueText("integrations_settle_first", "count" to hubrise.onlineOrdersOpen),
                Modifier.testTag("integration_settle"),
                color = colors.accentWarning,
            )
        }
        if (hubrise.log.isNotEmpty()) {
            Column(
                Modifier.fillMaxWidth().testTag("integration_log"),
                verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs),
            ) {
                SectionLabel(vueText("integrations_log"))
                hubrise.log.take(LOG_SHOWN).forEachIndexed { index, change -> LogLine(index, change) }
            }
        }
    }
}

@Composable
private fun LogLine(
    index: Int,
    change: SwitchChange,
) {
    val who =
        if (change.source == "admin") {
            vueText("integrations_source_admin", "who" to (change.changedBy ?: ""))
        } else {
            vueText("integrations_source_config")
        }
    Row(
        modifier = Modifier.semantics(mergeDescendants = true) {}.testTag("integration_log_$index"),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
    ) {
        val style = PiliTheme.typography.figure.copy(fontSize = PiliTheme.typography.sizeSm)
        Text(
            PosDateTime.formatTimestamp(change.changedAt, PosDateTime::formatDayTime),
            style = style,
            color = PiliTheme.colors.textSecondary,
        )
        Text(vueText(if (change.enabled) "integrations_on" else "integrations_off"), style = style, color = PiliTheme.colors.textPrimary)
        Text(who, style = style, color = PiliTheme.colors.textSecondary)
    }
}

/** The ticket deliveries (`.deliveries-section`): the mail status pills, then the last deliveries, times in Brussels. */
@Composable
private fun DeliveriesPanel(
    deliveries: List<DeliveryView>,
    loading: Boolean,
    mail: MailStatus?,
    onReload: () -> Unit,
) {
    val colors = PiliTheme.colors
    Panel(modifier = Modifier.heightIn(max = DeliveriesMaxHeight).testTag("settings_deliveries")) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            PanelTitle(vueText("settings_email_title"), Modifier.weight(1f))
            ActionButton(vueText("delivery_settings_refresh"), onReload, Modifier.testTag("deliveries_refresh"), enabled = !loading)
        }
        Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
            val configured = mail?.configured == true
            StatusPill(
                vueText(if (configured) "delivery_settings_configured" else "delivery_settings_not_configured"),
                if (configured) colors.accentSuccess else colors.accentWarning,
                Modifier.testTag("mail_status_configured"),
            )
            if (configured) {
                val reachable = mail?.reachable == true
                StatusPill(
                    vueText(if (reachable) "delivery_settings_reachable" else "delivery_settings_unreachable"),
                    if (reachable) colors.accentSuccess else colors.accentDanger,
                    Modifier.testTag("mail_status_reachable"),
                )
            }
        }
        if (deliveries.isEmpty()) {
            PanelText(vueText("delivery_settings_empty"), Modifier.testTag("deliveries_empty"))
        } else {
            Column(Modifier.weight(1f, fill = false).verticalScroll(rememberScrollState())) {
                deliveries.forEach { DeliveryLine(it) }
            }
        }
    }
}

@Composable
private fun DeliveryLine(delivery: DeliveryView) {
    val colors = PiliTheme.colors
    val statusColor =
        when (delivery.status) {
            "sent", "fallback_printed" -> colors.accentSuccess
            "failed" -> colors.accentDanger
            "sending", "pending" -> colors.accentWarning
            else -> colors.textSecondary
        }
    val small = PiliTheme.typography.figure.copy(fontSize = PiliTheme.typography.sizeSm)
    Column(Modifier.fillMaxWidth().padding(vertical = 12.dp).testTag("delivery_" + delivery.id)) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text(
                delivery.recipientHint,
                style = PiliTheme.typography.figure.copy(fontWeight = PiliTheme.typography.weightBold),
                color = colors.textPrimary,
            )
            Text(
                vueText("delivery_status_" + delivery.status),
                style = PiliTheme.typography.label,
                color = statusColor,
                modifier = Modifier.testTag("delivery_" + delivery.id + "_status"),
            )
        }
        Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd)) {
            Text(
                PosDateTime.formatTimestamp(delivery.createdAt, PosDateTime::formatDayTime),
                style = small,
                color = colors.textSecondary,
                modifier = Modifier.testTag("delivery_" + delivery.id + "_date"),
            )
            Text(vueText("delivery_attempts") + ": " + delivery.attempts, style = small, color = colors.textSecondary)
        }
        if (delivery.lastError != null) {
            Text(
                vueText("delivery_last_error") + ": " + delivery.lastError,
                style = small,
                color = colors.accentDanger,
                modifier = Modifier.testTag("delivery_" + delivery.id + "_error"),
            )
        }
        Hairline()
    }
}

// ── Dialogs ───────────────────────────────────────────────────────

/**
 * The dialogs of the settings screen, drawn in the scaffold's overlay slot (PD214): the PIN dialogs of the PIN
 * flow (own PIN, another employee's, a new manager's) and the confirmation of a deactivation.
 */
@Composable
internal fun SettingsDialogs(
    state: SettingsUiState,
    pinActions: SettingsPinActions,
    deactivate: DeactivateActions,
) {
    val flow = state.pin
    if (flow != null) key(flow.stage) { SettingsPinDialog(flow, pinActions) }
    val armed = state.confirmDeactivate
    if (armed != null) {
        val name = state.users.firstOrNull { it.insz == armed }?.name ?: armed
        ConfirmDialog(
            title = vueText("deactivate"),
            message = uiText(R.string.ui_settings_deactivate_message, "name" to name),
            confirmText = vueText("deactivate"),
            cancelText = vueText("back"),
            onConfirm = { deactivate.onConfirm(armed) },
            onCancel = deactivate.onCancel,
            destructive = true,
            modifier = Modifier.testTag("settings_deactivate_dialog"),
        )
    }
}

@Composable
private fun SettingsPinDialog(
    flow: SettingsPinFlow,
    actions: SettingsPinActions,
) {
    val confirming = flow.stage == PinStage.ConfirmWithManager
    val texts =
        PinPadTexts(
            title = vueText("pin_enter"),
            hint = if (confirming) vueText("pin_confirm_manager") else flow.name,
            enterPrompt = vueText("pin_enter"),
            confirmPrompt = vueText("pin_confirm"),
            mismatch = vueText("pin_mismatch"),
            cancel = vueText("back"),
            clearLabel = uiText(R.string.ui_common_numpad_clear),
            backspaceLabel = uiText(R.string.ui_common_numpad_backspace),
        )
    val own = flow.purpose == PinPurpose.Own
    PinDialog(
        mode = if (confirming) PinMode.Verify else PinMode.Choose,
        texts = texts,
        onSubmit = if (confirming) actions.onManagerPin else actions.onChosen,
        describeError = actions.describeError,
        // The choose dialog of another employee or of a new manager moved the flow on in onChosen: nothing to close.
        onDone = if (confirming || own) actions.onDone else ({}),
        onCancel = actions.onCancel,
        modifier = Modifier.testTag("settings_pin_dialog"),
    )
}

/** "Worldline", "CCV (cloud)", "CCV (reseau local)", or "Aucun (saisie manuelle)" for a till whose cards are typed by hand. */
@Composable
internal fun paymentTerminalLabel(kind: String?): String =
    when (kind) {
        null -> uiText(R.string.ui_eft_terminal_none)
        "worldline_terminal_api" -> uiText(R.string.ui_eft_kind_worldline_terminal_api)
        "ccv_payment_api" -> uiText(R.string.ui_eft_kind_ccv_payment_api)
        "ccv_opi" -> uiText(R.string.ui_eft_kind_ccv_opi)
        else -> kind
    }
