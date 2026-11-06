package dev.nuagemagique.pilipos.ui.state

import dev.nuagemagique.pilipos.di.ApplicationScope
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.launch
import javax.inject.Inject
import javax.inject.Singleton

/**
 * Makes the texts follow the user who is logged in (UX audit R5, `watch(currentUser)` of `App.vue`): their own language
 * when they chose one, else the terminal's; the login screen, where nobody is logged in, uses the terminal language.
 *
 * It only exists to be created once (the root of every screen asks the graph for it): the link is made in the
 * constructor, undispatched, so a language is applied in the same step as the login that asks for it.
 */
@Singleton
class UserLanguageBinder
    @Inject
    constructor(
        session: SessionStore,
        settings: PosSettingsStore,
        @ApplicationScope scope: CoroutineScope,
    ) {
        init {
            scope.launch(start = CoroutineStart.UNDISPATCHED) {
                session.currentUser.collect { user -> settings.applyUser(user?.insz) }
            }
        }
    }
