package dev.nuagemagique.pilipos.platform.update

import android.content.Context
import android.net.Uri
import dagger.hilt.android.qualifiers.ApplicationContext
import dev.nuagemagique.pilipos.BuildConfig
import javax.inject.Inject

/** Makes the two [UpdateSource]s of the screen (a seam: the view model tests use fakes). */
interface UpdateSources {
    /** The registry source of this build. */
    fun ghcr(): UpdateSource

    /**
     * The source for the folder [tree] chosen on a stick. Throws [UpdateException] (`usb_missing_file`) when the
     * folder cannot be read at all.
     */
    fun usb(tree: Uri): UpdateSource
}

/** The production [UpdateSources]: [GhcrSource.forApp] and a SAF [UsbSource]. */
class AppUpdateSources
    @Inject
    constructor(
        @ApplicationContext private val context: Context,
    ) : UpdateSources {
        override fun ghcr(): UpdateSource = GhcrSource.forApp(context.filesDir, BuildConfig.DEBUG)

        override fun usb(tree: Uri): UpdateSource = UsbSource(context, tree)
    }
