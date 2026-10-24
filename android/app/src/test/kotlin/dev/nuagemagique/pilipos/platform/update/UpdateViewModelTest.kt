package dev.nuagemagique.pilipos.platform.update

import android.content.Intent
import android.net.Uri
import dev.nuagemagique.pilipos.platform.export.FakeVolumes
import dev.nuagemagique.pilipos.platform.export.RemovableVolume
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.mockk
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.io.File

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class UpdateViewModelTest {
    private val preconditions = mockk<UpdatePreconditions>()
    private val installer = mockk<UpdateInstaller>()
    private val verifier = mockk<UpdateVerifier>()
    private val ghcr = mockk<UpdateSource>()
    private val usb = mockk<UpdateSource>()
    private val picker = Intent("pick")
    private val volumes = FakeVolumes(intent = picker)
    private val stick = RemovableVolume("1234-ABCD", "USB stick")
    private val verified = verifiedOf(File("pili-pos-android-1.2.0.apk"))
    private var failures: List<PreconditionFailure> = emptyList()
    private var usbError: UpdateException? = null
    private val installed = InstalledInfo("1.0.1", 101, "f".repeat(40))

    @Before
    fun setUp() {
        Dispatchers.setMain(UnconfinedTestDispatcher())
        coEvery { preconditions.check() } answers { failures }
        coEvery { verifier.verify(ghcr) } returns verified
        coEvery { verifier.verify(usb) } returns verified
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private fun viewModel(info: suspend () -> InstalledInfo? = { installed }) =
        UpdateViewModel(
            preconditions,
            installer,
            verifier,
            info,
            object : UpdateSources {
                override fun ghcr(): UpdateSource = ghcr

                override fun usb(tree: Uri): UpdateSource = usbError?.let { throw it } ?: usb
            },
            volumes,
        )

    @Test
    fun beforeTheFirstRefreshNothingCanBeDone() {
        val vm = viewModel()
        assertFalse(vm.state.value.canCheck)
        assertFalse(vm.state.value.canInstall)
        vm.checkOnline()
        assertEquals(UpdatePhase.Idle, vm.state.value.phase)
        assertNull(vm.pickerIntent())
    }

    @Test
    fun refreshReadsTheInstalledVersionAndThePreconditions() {
        failures = listOf(PreconditionFailure.OPEN_ORDER)
        val vm = viewModel().also { it.refresh() }
        assertEquals(installed, vm.state.value.installed)
        assertEquals(failures, vm.state.value.failures)
        assertTrue(vm.state.value.canCheck)
    }

    @Test
    fun anInstalledVersionThatCannotBeReadIsNotFatal() {
        val vm = viewModel { error("no package") }.also { it.refresh() }
        assertNull(vm.state.value.installed)
        assertTrue(vm.state.value.checked)
    }

    @Test
    fun aNonManagerCannotCheckAnything() {
        failures = listOf(PreconditionFailure.NOT_MANAGER)
        val vm = viewModel().also { it.refresh() }
        assertTrue(vm.state.value.notManager)
        vm.checkOnline()
        assertNull(vm.pickerIntent())
        assertEquals(UpdatePhase.Idle, vm.state.value.phase)
    }

    @Test
    fun checkOnlineVerifiesTheRegistryAndOffersTheInstall() {
        val vm = viewModel().also { it.refresh() }
        vm.checkOnline()
        assertEquals(UpdatePhase.Verified(verified), vm.state.value.phase)
        assertTrue(vm.state.value.canInstall)
    }

    @Test
    fun aRefusalIsShownAndNothingCanBeInstalled() {
        val refusal = UpdateRefusal(UpdateRefusalCode.BAD_SIGNATURE, "x")
        coEvery { verifier.verify(ghcr) } returns refusal
        val vm = viewModel().also { it.refresh() }
        vm.checkOnline()
        assertEquals(UpdatePhase.Refused(refusal), vm.state.value.phase)
        assertFalse(vm.state.value.canInstall)
    }

    @Test
    fun aVerifiedUpdateWithAPreconditionFailureCannotBeInstalled() {
        failures = listOf(PreconditionFailure.Z_DUE)
        val vm = viewModel().also { it.refresh() }
        vm.checkOnline()
        assertTrue(vm.state.value.phase is UpdatePhase.Verified)
        assertFalse(vm.state.value.canInstall)
        vm.install()
        coVerify(exactly = 0) { installer.install(any()) }
    }

    @Test
    fun theUsbButtonNeedsAMountedVolume() {
        val vm = viewModel().also { it.refresh() }
        assertNull(vm.pickerIntent())
        assertEquals(UpdatePhase.NoStick, vm.state.value.phase)
        volumes.volumes = listOf(stick)
        assertSame(picker, vm.pickerIntent())
    }

    @Test
    fun aPickedFolderIsVerifiedThroughTheUsbSource() {
        val vm = viewModel().also { it.refresh() }
        vm.onFolderPicked(Uri.parse("content://tree/1234-ABCD%3Aupdate"))
        assertEquals(UpdatePhase.Verified(verified), vm.state.value.phase)
        coVerify { verifier.verify(usb) }
    }

    @Test
    fun aFolderThatCannotBeReadIsARefusal() {
        usbError = UpdateException(UpdateRefusalCode.USB_MISSING_FILE, "the chosen folder cannot be read")
        val vm = viewModel().also { it.refresh() }
        vm.onFolderPicked(Uri.parse("content://tree/x"))
        val phase = vm.state.value.phase as UpdatePhase.Refused
        assertEquals(UpdateRefusalCode.USB_MISSING_FILE, phase.refusal.code)
    }

    @Test
    fun installShowsInstallingAndStaysThereWhenCommitted() =
        runTest {
            val gate = CompletableDeferred<InstallOutcome>()
            coEvery { installer.install(verified) } coAnswers { gate.await() }
            val vm = viewModel().also { it.refresh() }
            vm.checkOnline()
            vm.install()
            assertEquals(UpdatePhase.Installing, vm.state.value.phase)
            assertFalse(vm.state.value.canCheck)
            gate.complete(InstallOutcome.Committed)
            assertEquals(UpdatePhase.Installing, vm.state.value.phase)
        }

    @Test
    fun aBlockedInstallGoesBackToTheVerifiedUpdateWithTheFailures() {
        coEvery { installer.install(verified) } returns InstallOutcome.Blocked(listOf(PreconditionFailure.OPEN_ORDER))
        val vm = viewModel().also { it.refresh() }
        vm.checkOnline()
        vm.install()
        assertEquals(UpdatePhase.Verified(verified), vm.state.value.phase)
        assertEquals(listOf(PreconditionFailure.OPEN_ORDER), vm.state.value.failures)
        assertFalse(vm.state.value.canInstall)
    }

    @Test
    fun aRefusedInstallShowsItsReason() {
        coEvery { installer.install(verified) } returns InstallOutcome.Refused(InstallRefusal.ENGINE_BUSY, "busy")
        val vm = viewModel().also { it.refresh() }
        vm.checkOnline()
        vm.install()
        assertEquals(UpdatePhase.InstallFailed(InstallRefusal.ENGINE_BUSY, "busy"), vm.state.value.phase)
    }

    @Test
    fun installDoesNothingWithoutAVerifiedUpdate() {
        val vm = viewModel().also { it.refresh() }
        vm.install()
        coVerify(exactly = 0) { installer.install(any()) }
    }
}
