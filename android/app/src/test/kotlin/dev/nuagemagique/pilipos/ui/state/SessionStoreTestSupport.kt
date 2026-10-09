@file:Suppress("MatchingDeclarationName")

package dev.nuagemagique.pilipos.ui.state

import dev.nuagemagique.pilipos.engine.EngineFixture
import io.mockk.coEvery
import kotlinx.coroutines.channels.Channel
import uniffi.pos_ffi.Integrations
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PosSettings
import uniffi.pos_ffi.User
import uniffi.pos_ffi.UserRole

/** Engine events the test sends by hand: the fixture stream answers them one by one through the real pump. */
internal class EventFeed(
    fixture: EngineFixture,
) {
    private val channel = Channel<PosEvent?>(Channel.UNLIMITED)

    init {
        coEvery { fixture.stream.next() } coAnswers { channel.receive() }
    }

    fun send(event: PosEvent) {
        channel.trySend(event)
    }
}

internal fun user(
    role: UserRole = UserRole.ADMIN,
    insz: String = "85010100123",
): User = User(insz = insz, name = "Test", role = role, active = true, hasPin = true)

internal fun posSettings(training: Boolean = false): PosSettings =
    PosSettings(
        trainingMode = training,
        printerAddress = null,
        fdmUrl = "http://fdm",
        posId = "POS1",
        terminalId = "T1",
        restaurantName = "Test",
        language = "fr",
        configProblems = emptyList(),
        posSwVersion = "pili-pos-android/test",
    )

internal fun integrations(
    enabled: Boolean = false,
    open: Long = 0L,
): Integrations = Integrations(hubriseEnabled = enabled, onlineOrdersOpen = open, hubriseLog = emptyList())
