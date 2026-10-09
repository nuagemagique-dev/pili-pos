package dev.nuagemagique.pilipos.platform

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import javax.inject.Inject
import javax.inject.Singleton

/**
 * Names the long platform operations that are running ("update", "export"), so that the crash watchdog does
 * not reopen an engine that an operation closed on purpose.
 */
@Singleton
class PlatformOps
    @Inject
    constructor() {
        private val counts = mutableMapOf<String, Int>()
        private val _busy = MutableStateFlow<Set<String>>(emptySet())

        /** Names of the operations running now. */
        val busy: StateFlow<Set<String>> = _busy.asStateFlow()

        /** Runs [block] with [name] in [busy]; the name leaves the set when the block ends, however it ends. */
        suspend fun <T> run(
            name: String,
            block: suspend () -> T,
        ): T {
            enter(name)
            try {
                return block()
            } finally {
                leave(name)
            }
        }

        private fun enter(name: String) =
            synchronized(counts) {
                counts[name] = (counts[name] ?: 0) + 1
                _busy.value = counts.keys.toSet()
            }

        private fun leave(name: String) =
            synchronized(counts) {
                val left = (counts[name] ?: 1) - 1
                if (left <= 0) counts.remove(name) else counts[name] = left
                _busy.value = counts.keys.toSet()
            }
    }
