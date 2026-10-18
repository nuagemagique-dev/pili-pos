package dev.nuagemagique.pilipos.platform.update

import dev.nuagemagique.pilipos.engine.EngineNatives
import dev.nuagemagique.pilipos.engine.FakeNatives
import uniffi.pos_ffi.MinisignCheck
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.security.KeyFactory
import java.security.MessageDigest
import java.security.Signature
import java.security.spec.X509EncodedKeySpec
import java.util.Base64

/** The fixtures of `src/test/resources/platform/`, signed with the development minisign key. */
object UpdateFixtures {
    /** The development public key line (`android/app/provisioning/dev-minisign.pub`). */
    const val DEV_KEY = "RWRrbVE2do8gX6jV4FXRQpiK3QIKlruHZzxVT/P81uXp7QShDib1tTBj"

    /** The bytes that `declared.json` describes: its SHA-1 and SHA-256 are in the fixture. */
    val ASSET_BYTES: ByteArray = "pili-pos-android-1.2.0 test payload\n".toByteArray()

    const val ASSET_NAME = "pili-pos-android-1.2.0.apk"
    const val SIGNER = "5bd3a6bd00a3455c54f313b2152f4847233460085a56f8630986b043b52cd7ba"
    const val VERSION_CODE = 102L

    val json: ByteArray get() = read("declared.json")
    val tamperedJson: ByteArray get() = read("declared.tampered.json")
    val minisig: String get() = String(read("declared.json.minisig"), Charsets.UTF_8)

    private fun read(name: String): ByteArray =
        checkNotNull(UpdateFixtures::class.java.getResourceAsStream("/platform/$name")) { "missing fixture $name" }
            .use { it.readBytes() }
}

/** A `declared.json` whose values are the fixture's, with [overrides] replacing or removing (null) keys. */
fun descriptorJson(vararg overrides: Pair<String, Any?>): ByteArray {
    val json = org.json.JSONObject(String(UpdateFixtures.json, Charsets.UTF_8))
    overrides.forEach { (key, value) -> if (value == null) json.remove(key) else json.put(key, value) }
    return json.toString().toByteArray()
}

/**
 * [EngineNatives] whose `verifyMinisign` really checks a minisign signature on the JVM: prehashed (`ED`)
 * signatures, Ed25519 over BLAKE2b-512, and the global signature of the trusted comment. Everything else is
 * the [FakeNatives] behaviour. It stands in for the Rust function so that the fixtures are verified for real.
 */
class RealMinisignNatives(
    private val base: FakeNatives = FakeNatives(),
) : EngineNatives by base {
    var calls = 0

    override suspend fun verifyMinisign(
        publicKey: String,
        data: ByteArray,
        signature: String,
    ): MinisignCheck {
        calls++
        val key = decode(publicKey.lines().last { it.isNotBlank() }.trim(), 42)
        return when {
            key == null || key[0] != 'E'.code.toByte() || key[1] != 'd'.code.toByte() -> failed("minisign_key_invalid")
            verifies(key, data, signature.lines()) -> MinisignCheck(valid = true, trustedComment = comment(signature), problem = null)
            else -> failed("minisign_signature_invalid")
        }
    }

    private fun comment(signature: String): String? = signature.lines().getOrNull(2)?.removePrefix("trusted comment: ")

    private fun verifies(
        key: ByteArray,
        data: ByteArray,
        lines: List<String>,
    ): Boolean {
        val sig = lines.getOrNull(1)?.let { decode(it.trim(), 74) }
        val comment = lines.getOrNull(2)?.removePrefix("trusted comment: ")
        val global = lines.getOrNull(3)?.let { decode(it.trim(), 64) }
        if (sig == null || comment == null || global == null) return false
        val prehashed = sig[0] == 'E'.code.toByte() && sig[1] == 'D'.code.toByte()
        val publicBytes = key.copyOfRange(10, 42)
        val signatureBytes = sig.copyOfRange(10, 74)
        return prehashed && sig.copyOfRange(2, 10).contentEquals(key.copyOfRange(2, 10)) &&
            ed25519(publicBytes, Blake2b.hash512(data), signatureBytes) &&
            ed25519(publicBytes, signatureBytes + comment.toByteArray(), global)
    }

    private fun failed(problem: String) = MinisignCheck(valid = false, trustedComment = null, problem = problem)

    private fun decode(
        text: String,
        size: Int,
    ): ByteArray? = runCatching { Base64.getDecoder().decode(text) }.getOrNull()?.takeIf { it.size == size }

    private fun ed25519(
        publicKey: ByteArray,
        message: ByteArray,
        signature: ByteArray,
    ): Boolean {
        val spki = byteArrayOf(0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00) + publicKey
        val key = KeyFactory.getInstance("Ed25519").generatePublic(X509EncodedKeySpec(spki))
        return Signature.getInstance("Ed25519").run {
            initVerify(key)
            update(message)
            runCatching { verify(signature) }.getOrDefault(false)
        }
    }
}

/** BLAKE2b-512 (RFC 7693), unkeyed: the JDK has none and minisign prehashes with it. */
object Blake2b {
    private val IV =
        longArrayOf(
            0x6a09e667f3bcc908L,
            -0x4498517a7b3558c5L,
            0x3c6ef372fe94f82bL,
            -0x5ab00ac5a0e2c90fL,
            0x510e527fade682d1L,
            -0x64fa9773d4c193e1L,
            0x1f83d9abfb41bd6bL,
            0x5be0cd19137e2179L,
        )
    private val SIGMA =
        arrayOf(
            intArrayOf(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15),
            intArrayOf(14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3),
            intArrayOf(11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4),
            intArrayOf(7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8),
            intArrayOf(9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13),
            intArrayOf(2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9),
            intArrayOf(12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11),
            intArrayOf(13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10),
            intArrayOf(6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5),
            intArrayOf(10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0),
        )

    /** The 64-byte digest of [data]. */
    fun hash512(data: ByteArray): ByteArray {
        val h = IV.copyOf()
        h[0] = h[0] xor 0x01010040L
        val blocks = maxOf(1, (data.size + 127) / 128)
        for (index in 0 until blocks) {
            val block = ByteArray(128)
            val from = index * 128
            val length = minOf(128, data.size - from).coerceAtLeast(0)
            System.arraycopy(data, from, block, 0, length)
            val last = index == blocks - 1
            compress(h, block, if (last) data.size.toLong() else (from + 128).toLong(), last)
        }
        return ByteBuffer.allocate(64).order(ByteOrder.LITTLE_ENDIAN).apply { h.forEach { putLong(it) } }.array()
    }

    private fun compress(
        h: LongArray,
        block: ByteArray,
        counter: Long,
        last: Boolean,
    ) {
        val m = LongArray(16)
        ByteBuffer.wrap(block).order(ByteOrder.LITTLE_ENDIAN).asLongBuffer().get(m)
        val v = LongArray(16)
        System.arraycopy(h, 0, v, 0, 8)
        System.arraycopy(IV, 0, v, 8, 8)
        v[12] = v[12] xor counter
        if (last) v[14] = v[14].inv()
        for (round in 0 until 12) {
            val s = SIGMA[round % 10]
            mix(v, 0, 4, 8, 12, m[s[0]], m[s[1]])
            mix(v, 1, 5, 9, 13, m[s[2]], m[s[3]])
            mix(v, 2, 6, 10, 14, m[s[4]], m[s[5]])
            mix(v, 3, 7, 11, 15, m[s[6]], m[s[7]])
            mix(v, 0, 5, 10, 15, m[s[8]], m[s[9]])
            mix(v, 1, 6, 11, 12, m[s[10]], m[s[11]])
            mix(v, 2, 7, 8, 13, m[s[12]], m[s[13]])
            mix(v, 3, 4, 9, 14, m[s[14]], m[s[15]])
        }
        for (i in 0 until 8) h[i] = h[i] xor v[i] xor v[i + 8]
    }

    @Suppress("LongParameterList")
    private fun mix(
        v: LongArray,
        a: Int,
        b: Int,
        c: Int,
        d: Int,
        x: Long,
        y: Long,
    ) {
        v[a] = v[a] + v[b] + x
        v[d] = java.lang.Long.rotateRight(v[d] xor v[a], 32)
        v[c] = v[c] + v[d]
        v[b] = java.lang.Long.rotateRight(v[b] xor v[c], 24)
        v[a] = v[a] + v[b] + y
        v[d] = java.lang.Long.rotateRight(v[d] xor v[a], 16)
        v[c] = v[c] + v[d]
        v[b] = java.lang.Long.rotateRight(v[b] xor v[c], 63)
    }
}

/** Hex of the SHA-256 of [bytes]. */
fun sha256Hex(bytes: ByteArray): String = MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { "%02x".format(it) }
