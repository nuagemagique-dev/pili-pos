//! The background loops of the engine: the automatic Z, the digital
//! ticket outbox, the nightly export, the presence worker, the idle
//! lock, the FDM status monitor and, with the `hubrise` feature, the
//! `HubRise` poller and outbox. The shell calls [`Engine::start_background`] once the runtime
//! is up and [`Engine::shutdown`] when it closes; the loops run on the
//! runtime the shell gives and report through the engine's
//! [`crate::events::EventSink`].

use std::sync::Arc;
use std::sync::atomic::Ordering;

use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::commands;
use crate::commands::session::{LockReason, PresenceJob};
use crate::engine::Engine;
use crate::state::AppState;

/// The idle lock (PD35): locks the session once `timers.idle_lock` has
/// passed since the last activity (`SessionRules::touch`, called by
/// `user_activity` and by every login), through the same code as the
/// lock button with reason idle. Without a session, and after a lock,
/// it waits for the next touch instead of polling. Returns when the
/// shutdown token is cancelled.
pub async fn run_idle_lock(state: Arc<AppState>) {
    let rules = &state.session_rules;
    loop {
        // Enable the wait before reading the clock: a touch between the
        // two would otherwise be lost.
        let mut touched = std::pin::pin!(rules.next_activity());
        touched.as_mut().enable();

        let session_open = state.session.lock().is_ok_and(|s| s.is_some());
        if !session_open {
            tokio::select! {
                () = state.shutdown_token.cancelled() => return,
                () = &mut touched => continue,
            }
        }

        let deadline = rules.last_activity() + state.timers.idle_lock;
        if Instant::now() < deadline {
            tokio::select! {
                () = state.shutdown_token.cancelled() => return,
                () = &mut touched => {}
                () = tokio::time::sleep_until(deadline) => {}
            }
            // Re-read the clock: a touch moved the deadline, a sleep
            // that ran out leaves the lock below.
            continue;
        }

        tracing::info!("idle lock: locking the session");
        let locking = commands::session::lock(&state, LockReason::Idle);
        tokio::select! {
            () = state.shutdown_token.cancelled() => return,
            result = locking => {
                if let Err(e) = result {
                    tracing::warn!(%e, "idle lock failed");
                }
            }
        }
        // Whatever happened, wait for the next touch: a failed lock must
        // not spin on a deadline that already passed.
        tokio::select! {
            () = state.shutdown_token.cancelled() => return,
            () = &mut touched => {}
        }
    }
}

/// The FDM status monitor (PD36). While no session is open it waits and
/// sends nothing. A session that opens is checked at once; then it
/// waits `poll_delay(timers.fdm_poll, failures)` before each check, or
/// until an explicit check (`fdm_check_now`) or a session change makes it
/// recompute from now. When the session closes it stops polling.
/// Returns when the shutdown token is cancelled.
pub async fn run_fdm_monitor(state: Arc<AppState>) {
    let monitor = &state.fdm_monitor;
    let mut polling = false;
    loop {
        // Enable the waits before reading the session: a change between
        // the two would otherwise be lost (the permit would also keep it,
        // but a spurious pass costs nothing).
        let mut changed = std::pin::pin!(state.session_rules.session_changed());
        changed.as_mut().enable();
        let mut woken = std::pin::pin!(monitor.woken());
        woken.as_mut().enable();

        let session_open = state.session.lock().is_ok_and(|s| s.is_some());
        if !session_open {
            if polling {
                polling = false;
                monitor.clear_next_check();
            }
            tokio::select! {
                () = state.shutdown_token.cancelled() => return,
                () = &mut changed => continue,
            }
        }

        if !polling {
            // The session just opened: check at once, backoff restarted.
            polling = true;
            monitor.reset_failures();
            tokio::select! {
                () = state.shutdown_token.cancelled() => return,
                result = commands::fdm_monitor::check(&state, commands::fdm_monitor::Wake::No) => {
                    if let Err(e) = result {
                        tracing::warn!(%e, "FDM monitor check failed");
                    }
                }
            }
            continue;
        }

        let delay = commands::fdm_monitor::poll_delay(state.timers.fdm_poll, monitor.failures());
        tokio::select! {
            () = state.shutdown_token.cancelled() => return,
            () = &mut changed => {}
            () = &mut woken => {}
            () = tokio::time::sleep(delay) => {
                tokio::select! {
                    () = state.shutdown_token.cancelled() => return,
                    result = commands::fdm_monitor::check(&state, commands::fdm_monitor::Wake::No) => {
                        if let Err(e) = result {
                            tracing::warn!(%e, "FDM monitor check failed");
                        }
                    }
                }
            }
        }
    }
}

impl Engine {
    /// Spawn the loops on `rt`: all of them stop when
    /// [`Engine::shutdown`] cancels the shutdown token.
    #[expect(
        clippy::too_many_lines,
        reason = "spawns each background loop in turn; a helper per loop would only hide the list"
    )]
    pub fn start_background(&self, rt: &tokio::runtime::Handle) {
        let state = Arc::clone(self.state());
        let mut spawned: Vec<JoinHandle<()>> = Vec::new();

        // Automatic Z of a business day left open (producers FAQ
        // Q501c): at start-up, then every minute. A failure (FDM
        // unreachable) is retried on the next tick; meanwhile new
        // events wait (`commands::booking`).
        {
            let state = Arc::clone(&state);
            let ticks = Arc::clone(&self.ticks);
            spawned.push(rt.spawn(async move {
                let mut tick = tokio::time::interval(state.timers.auto_z);
                loop {
                    tokio::select! {
                        () = state.shutdown_token.cancelled() => return,
                        _ = tick.tick() => {
                            if let Err(e) = commands::report::close_overdue_day(&state).await {
                                tracing::warn!(%e, "automatic Z not made yet");
                            }
                            ticks.auto_z.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                }
            }));
        }

        // Digital ticket outbox: retries queued emails and clears
        // addresses after 72 h. Runs in every build; it does nothing
        // when no email client is configured.
        {
            let state = Arc::clone(&state);
            let ticks = Arc::clone(&self.ticks);
            spawned.push(rt.spawn(async move {
                let mut tick = tokio::time::interval(state.timers.mail_outbox);
                loop {
                    tokio::select! {
                        () = state.shutdown_token.cancelled() => return,
                        _ = tick.tick() => {
                            commands::delivery::run_outbox_once(&state).await;
                            ticks.mail_outbox.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                }
            }));
        }

        // Nightly export: one pass at start-up (it catches a Z made
        // before the last shutdown), then one at every
        // `timers.export_outbox` tick and at every wake-up (a real Z, the
        // run-now command). Spawned only when the configuration has a
        // usable `export` section, so a terminal without export runs the
        // same loops as before. A pass in flight is dropped at shutdown:
        // what it uploaded is already recorded, the rest is retried at the
        // next start.
        if state.export.is_enabled() {
            let state = Arc::clone(&state);
            let ticks = Arc::clone(&self.ticks);
            spawned.push(rt.spawn(async move {
                tokio::select! {
                    () = state.shutdown_token.cancelled() => return,
                    () = commands::export::run_startup_pass(&state) => {}
                }
                ticks.export_outbox.fetch_add(1, Ordering::SeqCst);
                let period = state.timers.export_outbox;
                let mut tick = tokio::time::interval_at(Instant::now() + period, period);
                loop {
                    tokio::select! {
                        () = state.shutdown_token.cancelled() => return,
                        _ = tick.tick() => {}
                        () = state.export.woken() => {}
                    }
                    tokio::select! {
                        () = state.shutdown_token.cancelled() => return,
                        () = commands::export::run_pass(&state) => {}
                    }
                    ticks.export_outbox.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }

        // Peppol delivery queue: one pass at start-up, then at every
        // `timers.peppol_outbox` tick and every wake-up (an invoice signed,
        // a parked delivery re-queued). Spawned only when the configuration
        // has a `peppol` section.
        if state.config.lock().is_ok_and(|c| c.peppol.is_some()) {
            let state = Arc::clone(&state);
            let ticks = Arc::clone(&self.ticks);
            spawned.push(rt.spawn(async move {
                let mut tick = tokio::time::interval(state.timers.peppol_outbox);
                loop {
                    tokio::select! {
                        () = state.shutdown_token.cancelled() => return,
                        _ = tick.tick() => {}
                        () = state.peppol_wake.notified() => {}
                    }
                    // Never dropped mid-delivery: the pass checks the
                    // token between deliveries and finishes the one in
                    // flight (bounded by the submit timeout).
                    commands::peppol_queue::run_pass_until(
                        &state,
                        chrono::Utc::now(),
                        &state.shutdown_token,
                    )
                    .await;
                    ticks.peppol_outbox.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }

        // Presence worker: signs the clock-ins that logins queue
        // (`commands::session`), one after another in arrival order, so
        // a login never waits for the FDM. The receiver is taken once:
        // a second start spawns nothing.
        if let Some(mut jobs) = state.session_rules.take_presence_jobs() {
            let state = Arc::clone(&state);
            spawned.push(rt.spawn(async move {
                loop {
                    tokio::select! {
                        () = state.shutdown_token.cancelled() => return,
                        job = jobs.recv() => match job {
                            Some(PresenceJob::ClockIn(insz)) => {
                                commands::social::clock_in_if_off_shift(&state, &insz).await;
                            }
                            None => return,
                        },
                    }
                }
            }));
        }

        // Remote support time limit: ends a session past its 2 h even
        // when nobody opens a screen that asks for its status.
        {
            let state = Arc::clone(&state);
            spawned.push(rt.spawn(async move {
                let period = state.timers.support_reap;
                let mut tick = tokio::time::interval_at(Instant::now() + period, period);
                loop {
                    tokio::select! {
                        () = state.shutdown_token.cancelled() => return,
                        _ = tick.tick() => commands::support::reap_expired(&state),
                    }
                }
            }));
        }

        // The idle lock: waits for the activity clock, locks an
        // unattended session (`commands::session::lock`, reason idle).
        spawned.push(rt.spawn(run_idle_lock(Arc::clone(&state))));

        // The FDM status monitor: polls while a session is open.
        spawned.push(rt.spawn(run_fdm_monitor(Arc::clone(&state))));

        // The owner alerts: one cheap read of the settings per
        // tick while they are off (the default), no probe and no HTTP
        // request. It is spawned in every start so that switching the
        // alerts on in Settings works without a restart.
        spawned.push(rt.spawn(crate::alerts::run_owner_alerts(Arc::clone(&state))));

        // The HubRise poller and outbox; a build without the `hubrise`
        // feature has no background HTTP workers at all.
        #[cfg(feature = "hubrise")]
        {
            use std::sync::Arc;

            // The crate's run_loop returns when the shutdown token is
            // cancelled.
            let mut poller_ctx = commands::hubrise::poller_ctx(&state);
            let sink = Arc::clone(&state.events);
            poller_ctx.on_outcome = Some(Arc::new(move |outcome, event| {
                if let Some(event) = crate::events::event_for_outcome(outcome, event) {
                    sink.emit(event);
                }
            }));
            let poller_cancel = state.shutdown_token.clone();
            spawned.push(rt.spawn(async move {
                pos_hubrise::poller::run_loop(poller_ctx, poller_cancel).await;
            }));

            // Outbox worker: drains POS to HubRise order pushes that
            // were enqueued by checkout. Same pattern as the poller:
            // background loop, short DB locks, async HTTP, graceful
            // shutdown via the shared CancellationToken.
            let outbox_ctx = pos_hubrise::outbox::OutboxCtx {
                http: state.shared_http.clone(),
                db: Arc::clone(&state.poller_db),
                wake: Arc::clone(&state.hubrise_outbox_wake),
                enabled: state.hubrise_switch.clone(),
            };
            let outbox_cancel = state.shutdown_token.clone();
            spawned.push(rt.spawn(async move {
                pos_hubrise::outbox::run_loop(outbox_ctx, outbox_cancel).await;
            }));
        }

        self.tasks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend(spawned);
    }

    /// Stop the background loops: cancel the shutdown token. They finish
    /// on their own; [`Engine::take_background_tasks`] hands out the
    /// handles to wait for them.
    pub fn shutdown(&self) {
        self.shutdown_token.cancel();
    }

    /// The handles of the spawned loops, removed from the engine, so a
    /// test (or a shell that wants a clean exit) can await them.
    pub fn take_background_tasks(&self) -> Vec<JoinHandle<()>> {
        std::mem::take(
            &mut *self
                .tasks
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }

    /// How many automatic Z passes have completed (for tests).
    pub fn auto_z_passes(&self) -> u64 {
        self.ticks.auto_z.load(Ordering::SeqCst)
    }

    /// How many mail outbox passes have completed (for tests).
    pub fn mail_outbox_passes(&self) -> u64 {
        self.ticks.mail_outbox.load(Ordering::SeqCst)
    }

    /// How many Peppol queue passes have completed (for tests).
    pub fn peppol_outbox_passes(&self) -> u64 {
        self.ticks.peppol_outbox.load(Ordering::SeqCst)
    }

    /// How many export passes have completed (for tests).
    pub fn export_passes(&self) -> u64 {
        self.ticks.export_outbox.load(Ordering::SeqCst)
    }
}
