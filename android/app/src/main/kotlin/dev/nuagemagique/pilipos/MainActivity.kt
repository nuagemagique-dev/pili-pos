package dev.nuagemagique.pilipos

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import dagger.hilt.android.AndroidEntryPoint
import dev.nuagemagique.pilipos.shell.ShellRoot

/**
 * The single activity (kiosk home, Lock Task). It only hosts the Compose shell.
 *
 * P4-T10 adds the kiosk hooks (lock task entry, back and recents handling); P4-T11 replaces the
 * placeholder [ShellRoot] content.
 */
@AndroidEntryPoint
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent { ShellRoot() }
    }
}
