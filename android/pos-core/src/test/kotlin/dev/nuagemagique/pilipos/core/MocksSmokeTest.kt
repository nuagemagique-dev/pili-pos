package dev.nuagemagique.pilipos.core

import java.net.HttpURLConnection
import java.net.URI
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class MocksSmokeTest {
    @Test
    fun printerMockStartsAnswersHttpAndStops() {
        val printer = Mocks.startPrinter()
        try {
            val (tcp, http) = printer.ports
            assertTrue("printer tcp port accepts", Mocks.accepts(tcp))
            val conn = URI("http://127.0.0.1:$http/jobs").toURL().openConnection() as HttpURLConnection
            try {
                conn.connectTimeout = 3000
                conn.readTimeout = 3000
                // Any status will do: the HTTP server answered.
                assertTrue("HTTP status ${conn.responseCode}", conn.responseCode in 100..599)
            } finally {
                conn.disconnect()
            }
        } finally {
            printer.stop()
        }
        assertFalse("printer process stopped", printer.isAlive)
        assertFalse("printer http port closed", Mocks.accepts(printer.ports[1]))
    }

    @Test
    fun fdmMockStartsAnswersGraphqlPostAndStops() {
        val fdm = Mocks.startFdm()
        try {
            val port = fdm.ports.single()
            val conn = URI("http://127.0.0.1:$port/graphql").toURL().openConnection() as HttpURLConnection
            try {
                conn.connectTimeout = 3000
                conn.readTimeout = 3000
                conn.requestMethod = "POST"
                conn.doOutput = true
                conn.setRequestProperty("Content-Type", "application/json")
                conn.outputStream.use { it.write("""{"query":"{ __typename }"}""".toByteArray()) }
                assertTrue("HTTP status ${conn.responseCode}", conn.responseCode in 100..599)
            } finally {
                conn.disconnect()
            }
        } finally {
            fdm.stop()
        }
        assertFalse("fdm process stopped", fdm.isAlive)
        assertFalse("fdm port closed", Mocks.accepts(fdm.ports.single()))
    }

    @Test
    fun bothMocksRunSideBySideOnDistinctPortsAndStop() {
        val fdm = Mocks.startFdm()
        val printer = Mocks.startPrinter()
        try {
            val all = fdm.ports + printer.ports
            assertTrue("ports are distinct: $all", all.toSet().size == all.size)
            assertTrue(all.all { Mocks.accepts(it) })
        } finally {
            printer.stop()
            fdm.stop()
        }
        assertFalse(fdm.isAlive)
        assertFalse(printer.isAlive)
    }
}
