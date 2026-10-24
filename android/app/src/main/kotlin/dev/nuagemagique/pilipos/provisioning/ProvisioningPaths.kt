package dev.nuagemagique.pilipos.provisioning

import android.content.Context
import dagger.hilt.android.qualifiers.ApplicationContext
import java.io.File
import javax.inject.Inject
import javax.inject.Singleton

/**
 * Where provisioning keeps its files, all under the app-private files directory:
 *
 * ```
 * provisioning/bundle/           the verified bundle in use (pos.json, MANIFEST.json, ...)
 * provisioning/staging-<millis>/ an import in progress
 * provisioning/record.json       the device binding of the installed bundle
 * engine/                        the engine data directory (pos.db, backups, engine.lock)
 * logs/                          the engine log directory
 * ```
 */
@Singleton
class ProvisioningPaths
    @Inject
    constructor(
        @param:ApplicationContext private val context: Context,
    ) {
        /** Parent of the bundle, the staging directories and the record. */
        val provisioningDir: File get() = File(context.filesDir, "provisioning")

        /** The installed bundle. */
        val bundleDir: File get() = File(provisioningDir, "bundle")

        /** The configuration file the engine opens with. */
        val posJson: File get() = File(bundleDir, "pos.json")

        /** The binding record of the installed bundle. */
        val recordFile: File get() = File(provisioningDir, "record.json")

        /** The engine data directory. */
        val dataDir: File get() = File(context.filesDir, "engine")

        /** The engine database inside [dataDir]. */
        val databaseFile: File get() = File(dataDir, "pos.db")

        /** The engine log directory. */
        val logDir: File get() = File(context.filesDir, "logs")

        /** A fresh staging directory name for an import started at [millis]. */
        fun stagingDir(millis: Long): File = File(provisioningDir, "$STAGING_PREFIX$millis")

        /** The name the previous bundle takes while a new one is swapped in. */
        fun oldBundleDir(millis: Long): File = File(provisioningDir, "$OLD_PREFIX$millis")

        /** Leftovers of an interrupted import (staging directories). */
        fun staleStagingDirs(): List<File> = provisioningDir.listFiles { f -> f.name.startsWith(STAGING_PREFIX) }.orEmpty().toList()

        /** Leftovers of an interrupted swap (previous bundles). */
        fun staleOldBundles(): List<File> = provisioningDir.listFiles { f -> f.name.startsWith(OLD_PREFIX) }.orEmpty().toList()

        /**
         * Debug builds only: a folder `adb push` can write without permissions, an alternative to the
         * folder picker for the emulator. Null when the platform gives no external files directory.
         */
        fun debugInboxDir(): File? = context.getExternalFilesDir(INBOX_NAME)

        private companion object {
            const val STAGING_PREFIX = "staging-"
            const val OLD_PREFIX = "bundle-old-"
            const val INBOX_NAME = "provisioning-inbox"
        }
    }
