package dev.nuagemagique.pilipos.platform.export

import android.content.ContentResolver
import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import android.net.Uri
import android.provider.DocumentsContract
import dagger.hilt.android.qualifiers.ApplicationContext
import timber.log.Timber
import javax.inject.Inject
import javax.inject.Singleton

/** The persisted tree grants of the content resolver, a seam so that tests do not need a provider. */
interface UriGrants {
    /** Keeps read and write access to [uri] across restarts. */
    fun take(uri: Uri)

    /** Drops the persisted access to [uri]. */
    fun release(uri: Uri)

    /** True when read and write access to [uri] is still persisted. */
    fun isHeld(uri: Uri): Boolean
}

/** [UriGrants] over [ContentResolver.takePersistableUriPermission]. */
class ResolverGrants(
    private val resolver: ContentResolver,
) : UriGrants {
    override fun take(uri: Uri) =
        resolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION)

    override fun release(uri: Uri) =
        resolver.releasePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION)

    override fun isHeld(uri: Uri): Boolean =
        resolver.persistedUriPermissions.any { it.uri == uri && it.isReadPermission && it.isWritePermission }
}

/** What [ExportFolderStore.take] did. */
sealed interface TakeResult {
    /** The tree is stored and its grant is persisted. */
    data class Taken(
        val tree: Uri,
    ) : TakeResult

    /** The tree was refused for [problem]; nothing was stored and no grant was taken. */
    data class Refused(
        val problem: ExportProblem,
    ) : TakeResult
}

/**
 * The folder the manager chose on each removable volume (Q17): the tree Uri per volume uuid in the
 * app-private preferences `platform_export`, with its persisted grant.
 *
 * The root of a volume cannot be exported to (the system refuses to grant it, and the SPF folder should be
 * an obvious one), so [take] refuses a tree whose document id ends with ':' with [ExportProblem.ROOT].
 */
@Singleton
class ExportFolderStore internal constructor(
    private val prefs: SharedPreferences,
    private val grants: UriGrants,
) {
    @Inject
    constructor(
        @ApplicationContext context: Context,
    ) : this(context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE), ResolverGrants(context.contentResolver))

    /** The stored tree of the volume [uuid], or null when none is stored or its grant is gone. */
    fun folder(uuid: String): Uri? {
        val stored = prefs.getString(key(uuid), null)?.let(Uri::parse) ?: return null
        return stored.takeIf { grants.isHeld(it) }
    }

    /** Validates [tree] for the volume [uuid], persists its grant, stores it and releases the previous grant. */
    fun take(
        uuid: String,
        tree: Uri,
    ): TakeResult {
        val problem = validate(uuid, tree)
        if (problem != null) return TakeResult.Refused(problem)
        val previous = prefs.getString(key(uuid), null)?.let(Uri::parse)
        grants.take(tree)
        prefs.edit().putString(key(uuid), tree.toString()).apply()
        if (previous != null && previous != tree) release(previous)
        return TakeResult.Taken(tree)
    }

    /** "Change folder": releases the grant of the volume [uuid] and forgets its folder. */
    fun change(uuid: String) {
        val previous = prefs.getString(key(uuid), null)?.let(Uri::parse)
        prefs.edit().remove(key(uuid)).apply()
        if (previous != null) release(previous)
    }

    private fun release(uri: Uri) {
        try {
            grants.release(uri)
        } catch (
            @Suppress("TooGenericExceptionCaught") e: RuntimeException,
        ) {
            Timber.w(e, "ExportFolderStore: the grant of %s could not be released", uri)
        }
    }

    private fun validate(
        uuid: String,
        tree: Uri,
    ): ExportProblem? {
        val id =
            try {
                DocumentsContract.getTreeDocumentId(tree)
            } catch (
                @Suppress("TooGenericExceptionCaught") e: IllegalArgumentException,
            ) {
                Timber.w(e, "ExportFolderStore: not a tree Uri: %s", tree)
                return ExportProblem.WRONG_VOLUME
            }
        return when {
            !id.startsWith("$uuid:", ignoreCase = true) -> ExportProblem.WRONG_VOLUME
            id.endsWith(":") -> ExportProblem.ROOT
            else -> null
        }
    }

    private fun key(uuid: String) = "tree.$uuid"

    companion object {
        /** Name of the preferences file (app-private). */
        const val PREFS_NAME = "platform_export"

        /** The name the manager sees for a tree: the path after the volume part of its document id. */
        fun displayName(tree: Uri): String =
            try {
                DocumentsContract.getTreeDocumentId(tree).substringAfter(':')
            } catch (
                @Suppress("TooGenericExceptionCaught") e: IllegalArgumentException,
            ) {
                Timber.w(e, "ExportFolderStore: not a tree Uri: %s", tree)
                tree.toString()
            }
    }
}
