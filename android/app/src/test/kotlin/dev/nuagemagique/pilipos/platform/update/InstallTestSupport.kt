package dev.nuagemagique.pilipos.platform.update

import dev.nuagemagique.pilipos.engine.CloseResult
import dev.nuagemagique.pilipos.engine.EngineGateway
import io.mockk.coEvery
import io.mockk.mockk
import uniffi.pos_ffi.FiscalEventSummary
import uniffi.pos_ffi.PosEngineInterface
import uniffi.pos_ffi.User
import uniffi.pos_ffi.UserRole
import uniffi.pos_ffi.ZReportSummary
import java.io.File
import java.io.IOException

/** 2026-10-09 10:15:30 in Brussels. */
const val UPDATE_NOW: Long = 1_791_537_330_000L

/** A gateway whose `call` runs the block against [engine] and whose `close` answers [close]. */
fun fakeGateway(
    engine: PosEngineInterface,
    close: () -> CloseResult = { CloseResult.Closed },
): EngineGateway {
    val gateway = mockk<EngineGateway>()
    coEvery { gateway.call<Any?>(any()) } coAnswers { firstArg<suspend (PosEngineInterface) -> Any?>().invoke(engine) }
    coEvery { gateway.close() } answers { close() }
    return gateway
}

fun adminUser(role: UserRole = UserRole.ADMIN): User = User("00000000029", "Admin", role, true, true)

fun fiscalEvent(
    id: Long,
    type: String,
): FiscalEventSummary = FiscalEventSummary(id, type, id, null, null, null, type, null)

fun zReport(
    sequence: Long,
    eventId: Long?,
): ZReportSummary = ZReportSummary(sequence, "2026-10-0$sequence", sequence, "2026-10-0$sequence 22:00:00", eventId)

/** An engine that answers "everything is fine": an administrator, nothing open, a Z report that is the newest event. */
fun readyEngine(): PosEngineInterface {
    val engine = mockk<PosEngineInterface>()
    coEvery { engine.getCurrentUser() } returns adminUser()
    coEvery { engine.listOpenOrders() } returns emptyList()
    coEvery { engine.listOpenTakeaways() } returns emptyList()
    coEvery { engine.needsZReport() } returns false
    coEvery { engine.listZReports() } returns listOf(zReport(2, 20), zReport(1, 10))
    coEvery { engine.listFiscalEvents(200) } returns listOf(fiscalEvent(20, "R"), fiscalEvent(19, "N"))
    coEvery { engine.listFiscalEvents(1) } returns listOf(fiscalEvent(20, "R"))
    coEvery { engine.backupDatabase() } returns "/data/backup.db"
    return engine
}

fun releaseOf(
    versionCode: Long = 102,
    version: String = "1.2.0",
): DeclaredRelease =
    DeclaredRelease(
        packageName = "dev.nuagemagique.pilipos",
        version = version,
        versionCode = versionCode,
        asset = "pili-pos-android-$version.apk",
        sha1 = "a".repeat(40),
        sha256 = "b".repeat(64),
        signerSha256 = "c".repeat(64),
        declaredAt = "2026-10-09T10:00:00+02:00",
    )

fun verifiedOf(
    file: File,
    release: DeclaredRelease = releaseOf(),
): VerifiedUpdate = VerifiedUpdate(release, file, release.sha1, release.sha256)

/** Records every call in [log] and can fail on demand. */
class FakeSessions(
    private val log: MutableList<String>,
) : PackageSessions {
    var onWrite: (File) -> Unit = {}
    var onCommit: () -> Unit = {}
    var failCreate: IOException? = null
    val abandoned = mutableListOf<Int>()
    var closed = 0
    var committed = 0
    var lastSize: Long = -1
    var nextId = 7

    override fun create(sizeBytes: Long): InstallSession {
        log += "create"
        failCreate?.let { throw it }
        lastSize = sizeBytes
        val sessionId = nextId
        return object : InstallSession {
            override val id: Int = sessionId

            override fun write(apk: File) {
                log += "write"
                onWrite(apk)
            }

            override fun commit() {
                log += "commit"
                committed++
                onCommit()
            }

            override fun close() {
                closed++
            }
        }
    }

    override fun abandon(id: Int) {
        log += "abandon"
        abandoned += id
    }
}
