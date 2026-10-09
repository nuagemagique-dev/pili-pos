package dev.nuagemagique.pilipos.platform.update

import android.content.Context
import dev.nuagemagique.pilipos.BuildConfig
import dev.nuagemagique.pilipos.R
import org.xmlpull.v1.XmlPullParser
import org.xmlpull.v1.XmlPullParserException
import java.io.IOException

/**
 * The public minisign keys the declared-only updater trusts (PD212), read from `res/xml/plat_update_keys.xml`:
 * one `<key build="debug">` and one `<key build="release">`. The debug key is the committed development key,
 * the release key stays empty until the declaration key exists. An empty key means that every update is
 * refused with `key_missing`.
 */
class UpdateKeys(
    private val keys: Map<String, String>,
    private val debug: Boolean,
) {
    /** The key of this build type, or null when it is empty or absent. */
    fun current(): String? = keys[if (debug) DEBUG else RELEASE]?.takeIf { it.isNotBlank() }

    companion object {
        /** Value of the `build` attribute of the debug key. */
        const val DEBUG = "debug"

        /** Value of the `build` attribute of the release key. */
        const val RELEASE = "release"

        /** Reads the resource; the build type is [BuildConfig.DEBUG]. */
        fun load(context: Context): UpdateKeys = parse(context.resources.getXml(R.xml.plat_update_keys), BuildConfig.DEBUG)

        /**
         * Reads [parser] (positioned anywhere before the root). The text of a `key` element is the key; its
         * `key` attribute is used when the text is empty. Elements without a `build` attribute are ignored.
         */
        fun parse(
            parser: XmlPullParser,
            debug: Boolean,
        ): UpdateKeys =
            try {
                UpdateKeys(readKeys(parser), debug)
            } catch (e: XmlPullParserException) {
                refuse(UpdateRefusalCode.KEY_MISSING, "plat_update_keys.xml is unreadable", e)
            } catch (e: IOException) {
                refuse(UpdateRefusalCode.KEY_MISSING, "plat_update_keys.xml is unreadable", e)
            }

        private fun readKeys(parser: XmlPullParser): Map<String, String> {
            val keys = mutableMapOf<String, String>()
            var event = parser.eventType
            while (event != XmlPullParser.END_DOCUMENT) {
                if (event == XmlPullParser.START_TAG && parser.name == "key") readKey(parser, keys)
                event = parser.next()
            }
            return keys
        }

        private fun readKey(
            parser: XmlPullParser,
            into: MutableMap<String, String>,
        ) {
            val build = parser.getAttributeValue(null, "build")
            val attribute = parser.getAttributeValue(null, "key").orEmpty()
            val text = parser.nextText().trim()
            if (build != null) into[build] = text.ifEmpty { attribute.trim() }
        }
    }
}
