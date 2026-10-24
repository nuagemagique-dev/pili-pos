package dev.nuagemagique.pilipos.platform

import dev.nuagemagique.pilipos.platform.export.ExportOutcome
import dev.nuagemagique.pilipos.platform.export.FileTarget
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import java.io.File

/** The `export` step: the SPF export onto the removable volume, then every `MANIFEST.sha256` entry hashed again. */
internal object PlatformExportStep {
    /** Name of the folder on the volume; the host script deletes it. */
    const val CHOSEN_FOLDER = "plat-e2e-export"

    fun run(env: PlatformE2eEnv) {
        env.login()
        val (appDir, uuid) = env.removableAppDir()
        val root = File(appDir, CHOSEN_FOLDER)
        root.deleteRecursively()
        assertTrue("cannot create $root", root.mkdirs())
        val outcome = runBlocking { env.export.exporter().run(FileTarget(root, uuid), CHOSEN_FOLDER) }
        val done =
            when (outcome) {
                is ExportOutcome.Done -> outcome.result
                is ExportOutcome.Failed -> throw AssertionError("the export failed: ${outcome.failure}")
            }
        val folder = File(root, done.exportFolder)
        val manifest = File(folder, "MANIFEST.sha256")
        assertTrue("MANIFEST.sha256 is missing in $folder", manifest.isFile)
        val entries = manifest.readLines().filter { it.isNotBlank() }.map { it.substringBefore("  ") to it.substringAfter("  ") }
        assertTrue("the manifest is empty", entries.isNotEmpty())
        for ((hash, path) in entries) {
            val file = File(folder, path)
            assertTrue("$path is listed in the manifest but missing", file.isFile)
            assertEquals("SHA-256 of $path", hash, PlatformE2eEnv.sha256(file))
        }
        val onDisk = folder.walkTopDown().filter { it.isFile && it.name != "MANIFEST.sha256" }.count()
        assertEquals("every exported file is in the manifest", onDisk, entries.size)
        assertEquals("files reported by the export", onDisk + 1, done.files)
        assertEquals("SHA-256 of the manifest", done.manifestSha256, PlatformE2eEnv.sha256(manifest))
        PlatformE2eEnv.line("PLATE2E_EXPORT ok files=${done.files} bytes=${done.bytes} dir=${folder.path}")
    }
}
