package dev.nuagemagique.pilipos.nav

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag

/** Stand-in screen of a feature until its real view lands: shows the technical [route] only. */
@Composable
fun PlaceholderScreen(
    area: String,
    route: String,
) {
    Box(modifier = Modifier.fillMaxSize().testTag("placeholder_$area"), contentAlignment = Alignment.Center) {
        Text(text = route)
    }
}
