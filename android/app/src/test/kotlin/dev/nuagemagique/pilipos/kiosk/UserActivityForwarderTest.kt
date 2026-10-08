package dev.nuagemagique.pilipos.kiosk

import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.engine.LaunchResult
import io.mockk.Runs
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.just
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import uniffi.pos_ffi.PosException

@OptIn(ExperimentalCoroutinesApi::class)
class UserActivityForwarderTest {
    @get:Rule
    val tmp = TemporaryFolder()

    @Test
    fun fiftyInteractionsInOneSecondMakeAtMostTwoCalls() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            coEvery { f.engine.userActivity() } just Runs
            f.holder.open()
            val forwarder = UserActivityForwarder(f.holder, f.gateway(this), backgroundScope)
            repeat(50) {
                forwarder.onInteraction()
                advanceTimeBy(20)
            }
            runCurrent()
            coVerify(exactly = 2) { f.engine.userActivity() }
            assertEquals(2L, forwarder.forwardedCount)
        }

    @Test
    fun theSecondInteractionWaitsForTheEndOfTheWindow() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            coEvery { f.engine.userActivity() } just Runs
            f.holder.open()
            val forwarder = UserActivityForwarder(f.holder, f.gateway(this), backgroundScope)
            forwarder.onInteraction()
            runCurrent()
            forwarder.onInteraction()
            advanceTimeBy(900)
            coVerify(exactly = 1) { f.engine.userActivity() }
            advanceTimeBy(200)
            coVerify(exactly = 2) { f.engine.userActivity() }
        }

    @Test
    fun anEngineThatIsNotReadyReceivesNoCall() =
        runTest {
            val f = EngineFixture(this, tmp.root, LaunchResult.NotProvisioned)
            coEvery { f.engine.userActivity() } just Runs
            f.holder.open()
            val forwarder = UserActivityForwarder(f.holder, f.gateway(this), backgroundScope)
            repeat(10) {
                forwarder.onInteraction()
                advanceTimeBy(500)
            }
            coVerify(exactly = 0) { f.engine.userActivity() }
            assertEquals(0L, forwarder.forwardedCount)
        }

    @Test
    fun aGatewayFailureIsSwallowedAndTheNextWindowStillCalls() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            coEvery { f.engine.userActivity() } throws PosException.EngineClosed() andThen Unit
            f.holder.open()
            val forwarder = UserActivityForwarder(f.holder, f.gateway(this), backgroundScope)
            forwarder.onInteraction()
            advanceTimeBy(1_100)
            forwarder.onInteraction()
            advanceTimeBy(1_100)
            coVerify(exactly = 2) { f.engine.userActivity() }
            assertEquals(1L, forwarder.forwardedCount)
        }

    @Test
    fun forwardedCountStartsAtZeroAndCountsEachCall() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            coEvery { f.engine.userActivity() } just Runs
            f.holder.open()
            val forwarder = UserActivityForwarder(f.holder, f.gateway(this), backgroundScope)
            assertEquals(0L, forwarder.forwardedCount)
            repeat(3) {
                forwarder.onInteraction()
                advanceTimeBy(1_100)
            }
            assertEquals(3L, forwarder.forwardedCount)
        }
}
