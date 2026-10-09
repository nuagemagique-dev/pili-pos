package dev.nuagemagique.pilipos.platform.audit

/** Wall clock of the platform package, a seam so that tests control the time. */
fun interface PlatformClock {
    /** Milliseconds since the epoch. */
    fun nowMillis(): Long
}

/** The real wall clock. */
object SystemPlatformClock : PlatformClock {
    override fun nowMillis(): Long = System.currentTimeMillis()
}
