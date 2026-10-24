@file:Suppress("MatchingDeclarationName")

package dev.nuagemagique.pilipos.ui.badge

import android.app.Activity
import android.content.Context
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.i18n.ErrorTexts
import dev.nuagemagique.pilipos.i18n.FdmTexts
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.SessionStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.test.TestScope
import uniffi.pos_ffi.BadgeOutcome
import uniffi.pos_ffi.BadgeOutcomeKind
import uniffi.pos_ffi.BadgeOverview
import uniffi.pos_ffi.BadgeSettingsView
import uniffi.pos_ffi.User

/** A source that reads nothing by itself: the test calls [scan], and the calls it received are recorded. */
internal class FakeBadgeSource : BadgeSource {
    private val shared = MutableSharedFlow<String>(extraBufferCapacity = 8)
    val calls: MutableList<String> = mutableListOf()
    var lastConfig: WedgeConfig? = null

    override val scans: Flow<String> = shared

    override fun start() {
        calls += "start"
    }

    override fun stop() {
        calls += "stop"
    }

    override fun configure(config: WedgeConfig) {
        lastConfig = config
    }

    fun scan(token: String) {
        shared.tryEmit(token)
    }
}

/** A device whose NFC the test controls; the reader mode calls are recorded. */
internal class FakeNfcSupport(
    override var hasNfc: Boolean = true,
    override var isEnabled: Boolean = true,
) : NfcSupport {
    val calls: MutableList<String> = mutableListOf()
    var onTag: ((ByteArray) -> Unit)? = null

    override fun enableReaderMode(
        activity: Activity,
        onTagId: (ByteArray) -> Unit,
    ) {
        calls += "enable"
        onTag = onTagId
    }

    override fun disableReaderMode(activity: Activity) {
        calls += "disable"
        onTag = null
    }
}

/** A cue that records what would have been played. */
internal class FakeBadgeCue : BadgeCue {
    val played: MutableList<String> = mutableListOf()

    override fun ok() {
        played += "ok"
    }

    override fun refused() {
        played += "refused"
    }
}

@Suppress("LongParameterList")
internal fun badgeView(
    reader: String = "off",
    loginEnabled: Boolean = false,
    gap: Long = 50L,
    min: Long = 6L,
    max: Long = 32L,
    relink: Boolean = false,
): BadgeSettingsView = BadgeSettingsView(reader, loginEnabled, gap, min, max, relink)

internal fun noBadges(): BadgeOverview = BadgeOverview(emptyList(), emptyList())

@Suppress("LongParameterList")
internal fun outcome(
    kind: BadgeOutcomeKind,
    insz: String? = null,
    name: String? = null,
    at: String? = null,
    error: String? = null,
    user: User? = null,
): BadgeOutcome = BadgeOutcome(kind, insz, name, at, error, user)

/** The badge store over real stores on [f], with fake sources, device NFC and cue. */
@Suppress("LongParameterList")
internal class BadgeRig(
    val store: BadgeStore,
    val usb: FakeBadgeSource,
    val nfc: FakeBadgeSource,
    val support: FakeNfcSupport,
    val cue: FakeBadgeCue,
    val toasts: ToastStore,
    val session: SessionStore,
    val catalog: MessageCatalog,
)

/** Builds the store before the engine opens (it subscribes to the events and the ready flag first). */
internal fun TestScope.badgeRig(
    f: EngineFixture,
    context: Context,
    support: FakeNfcSupport = FakeNfcSupport(),
): BadgeRig {
    val gateway = f.gateway(this)
    val toasts = ToastStore(backgroundScope)
    val catalog = MessageCatalog(context)
    val errors = ErrorPresenter(toasts, ErrorTexts(catalog, FdmTexts(catalog)))
    val session = SessionStore(gateway, f.events, backgroundScope)
    val settings = PosSettingsStore(context, gateway, f.events, backgroundScope)
    return badgeRigOver(f, gateway, session, toasts, errors, settings, catalog, support)
}

/** The store over stores a screen test already built, so both see the same session and toasts. */
@Suppress("LongParameterList")
internal fun TestScope.badgeRigOver(
    f: EngineFixture,
    gateway: EngineGateway,
    session: SessionStore,
    toasts: ToastStore,
    errors: ErrorPresenter,
    settings: PosSettingsStore,
    catalog: MessageCatalog,
    support: FakeNfcSupport = FakeNfcSupport(),
): BadgeRig {
    val usb = FakeBadgeSource()
    val nfc = FakeBadgeSource()
    val cue = FakeBadgeCue()
    val store =
        BadgeStore(gateway, f.events, session, toasts, errors, settings, catalog, cue, BadgeSources(usb, nfc), support, backgroundScope)
    return BadgeRig(store, usb, nfc, support, cue, toasts, session, catalog)
}
