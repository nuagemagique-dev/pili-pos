package dev.nuagemagique.pilipos.platform.about

import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.ByteArrayInputStream
import java.io.File

class HasherTest {
    @get:Rule
    val tmp = TemporaryFolder()

    @Test
    fun knownVectorsOfAbc() {
        val abc = "abc".toByteArray()
        assertEquals("a9993e364706816aba3e25717850c26c9cd0d89d", Hasher.sha1(ByteArrayInputStream(abc)))
        assertEquals(
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            Hasher.sha256(ByteArrayInputStream(abc)),
        )
        assertEquals("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", Hasher.sha256(abc))
    }

    @Test
    fun theEmptyInputHasTheEmptyDigest() {
        assertEquals("da39a3ee5e6b4b0d3255bfef95601890afd80709", Hasher.sha1(ByteArrayInputStream(ByteArray(0))))
    }

    @Test
    fun aFileLongerThanTheBufferIsHashedInChunks() {
        val data = ByteArray(Hasher.BUFFER_SIZE * 3 + 17) { (it % 251).toByte() }
        val file = File(tmp.root, "big.bin").apply { writeBytes(data) }
        val expected1 = java.security.MessageDigest.getInstance("SHA-1").digest(data)
        val expected256 = java.security.MessageDigest.getInstance("SHA-256").digest(data)
        assertEquals(Hasher.hex(expected1), Hasher.sha1(file))
        assertEquals(Hasher.hex(expected256), Hasher.sha256(file))
        assertEquals(Hasher.hex(expected1) to Hasher.hex(expected256), Hasher.sha1AndSha256(file))
    }

    @Test
    fun hexIsLowercaseAndPadded() {
        assertEquals("000fa0ff", Hasher.hex(byteArrayOf(0x00, 0x0f, 0xa0.toByte(), 0xff.toByte())))
    }
}
