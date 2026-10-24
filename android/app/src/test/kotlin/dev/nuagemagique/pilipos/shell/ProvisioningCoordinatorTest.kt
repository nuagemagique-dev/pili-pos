package dev.nuagemagique.pilipos.shell

import dev.nuagemagique.pilipos.engine.CloseResult
import dev.nuagemagique.pilipos.engine.EngineState
import dev.nuagemagique.pilipos.provisioning.BundleSource
import dev.nuagemagique.pilipos.provisioning.ImportResult
import dev.nuagemagique.pilipos.provisioning.ProvisioningProblem
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.mockk
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

class ProvisioningCoordinatorTest {
    private val source = BundleSource.Directory(File("inbox"))

    @Test
    fun closesImportsThenOpens() =
        runTest {
            val fixture = ShellFixture(initial = EngineState.Ready("1.0.0"))
            val success = ImportResult.Success(mockk(relaxed = true))
            coEvery { fixture.importer.import(source) } coAnswers
                {
                    fixture.calls += "import"
                    success
                }
            val result = fixture.coordinator.import(source)
            assertEquals(success, result)
            assertEquals(listOf("close", "import", "open"), fixture.calls)
        }

    @Test
    fun aFailedImportReopensThePreviousEngine() =
        runTest {
            val fixture = ShellFixture(initial = EngineState.Ready("1.0.0"))
            val failure = ImportResult.Failure(ProvisioningProblem.of("bundle_signature_invalid"))
            coEvery { fixture.importer.import(source) } coAnswers
                {
                    fixture.calls += "import"
                    failure
                }
            assertEquals(failure, fixture.coordinator.import(source))
            assertEquals(listOf("close", "import", "open"), fixture.calls)
        }

    @Test
    fun anEngineThatStaysOpenBlocksTheImport() =
        runTest {
            val fixture = ShellFixture(initial = EngineState.Ready("1.0.0"))
            coEvery { fixture.holder.close() } returns CloseResult.StillOpen("busy")
            val result = fixture.coordinator.import(source)
            assertTrue(result is ImportResult.Failure)
            assertEquals("engine_open", (result as ImportResult.Failure).problem.code)
            coVerify(exactly = 0) { fixture.importer.import(any()) }
            coVerify(exactly = 0) { fixture.holder.open() }
        }

    @Test
    fun theEngineIsReopenedWhenTheImporterThrows() =
        runTest {
            val fixture = ShellFixture()
            coEvery { fixture.importer.import(source) } throws IllegalStateException("disk")
            val thrown = runCatching { fixture.coordinator.import(source) }.exceptionOrNull()
            assertTrue(thrown is IllegalStateException)
            assertEquals(listOf("close", "open"), fixture.calls)
        }
}
