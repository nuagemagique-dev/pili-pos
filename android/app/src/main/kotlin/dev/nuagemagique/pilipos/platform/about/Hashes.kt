@file:Suppress("MatchingDeclarationName")

package dev.nuagemagique.pilipos.platform.about

import java.io.File
import java.io.InputStream
import java.security.MessageDigest

/**
 * Streamed message digests as lowercase hex, shared by the About screen and the cards that hash files later.
 *
 * Nothing is read into memory beyond one [BUFFER_SIZE] buffer, so an APK of tens of megabytes costs no heap.
 * The functions block: callers run them on the I/O dispatcher.
 */
object Hasher {
    /** Size of the read buffer: 64 KiB. */
    const val BUFFER_SIZE: Int = 64 * 1024

    /** SHA-1 of [file]. */
    fun sha1(file: File): String = file.inputStream().use { sha1(it) }

    /** SHA-256 of [file]. */
    fun sha256(file: File): String = file.inputStream().use { sha256(it) }

    /** SHA-1 of the rest of [input] (the stream is not closed). */
    fun sha1(input: InputStream): String = digests(input, SHA_1).single()

    /** SHA-256 of the rest of [input] (the stream is not closed). */
    fun sha256(input: InputStream): String = digests(input, SHA_256).single()

    /** SHA-1 and SHA-256 of [file] in one pass, in that order. */
    fun sha1AndSha256(file: File): Pair<String, String> =
        file.inputStream().use { input ->
            val (sha1, sha256) = digests(input, SHA_1, SHA_256)
            sha1 to sha256
        }

    /** SHA-256 of [bytes]. */
    fun sha256(bytes: ByteArray): String = hex(MessageDigest.getInstance(SHA_256).digest(bytes))

    /** The digests of the rest of [input] for each of [algorithms], in order, as lowercase hex. */
    fun digests(
        input: InputStream,
        vararg algorithms: String,
    ): List<String> {
        val all = algorithms.map { MessageDigest.getInstance(it) }
        val buffer = ByteArray(BUFFER_SIZE)
        while (true) {
            val read = input.read(buffer)
            if (read < 0) break
            all.forEach { it.update(buffer, 0, read) }
        }
        return all.map { hex(it.digest()) }
    }

    /** [bytes] as lowercase hex. */
    fun hex(bytes: ByteArray): String = bytes.toLowerHex()

    private const val SHA_1 = "SHA-1"
    private const val SHA_256 = "SHA-256"
}

private const val BYTE_MASK = 0xff
private const val NIBBLE = 4
private const val NIBBLE_MASK = 0x0f
private const val HEX_DIGITS = "0123456789abcdef"

/** The bytes as lowercase hex, two digits per byte. */
fun ByteArray.toLowerHex(): String {
    val out = StringBuilder(size * 2)
    for (b in this) {
        val v = b.toInt() and BYTE_MASK
        out.append(HEX_DIGITS[v ushr NIBBLE]).append(HEX_DIGITS[v and NIBBLE_MASK])
    }
    return out.toString()
}
