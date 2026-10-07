//! The background loops of the engine: the automatic Z, the digital
//! ticket outbox, the presence worker, the idle lock and, with the `hubrise` feature, the HubRise poller and
//! outbox. The shell calls [`Engine::start_background`] once the runtime
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

        let session_open = state.session.lock().map(|s| s.is_some()).unwrap_or(false);
        if !session_open {
            tokio::select! {
                _ = state.shutdown_token.cancelled() => return,
                _ = &mut touched => continue,
            }
        }

        let deadline = rules.last_activity() + state.timers.idle_lock;
        if Instant::now() < deadline {
            tokio::select! {
                _ = state.shutdown_token.cancelled() => return,
                _ = &mut touched => {}
                _ = tokio::time::sleep_until(deadline) => {}
            }
            // Re-read the clock: a touch moved the deadline, a sleep
            // that ran out leaves the lock below.
            continue;
        }

        tracing::info!("idle lock: locking the session");
        let locking = commands::session::lock(&state, LockReason::Idle);
        tokio::select! {
            _ = state.shutdown_token.cancelled() => return,
            result = locking => {
                if let Err(e) = result {
                    tracing::warn!(%e, "idle lock failed");
                }
            }
        }
        // Whatever happened, wait for the next touch: a failed lock must
        // not spin on a deadline that already passed.
        tokio::select! {
            _ = state.shutdown_token.cancelled() => return,
            _ = &mut touched => {}
        }
    }
}

impl Engine {
    /// Spawn the loops on `rt`: all of them stop when
    /// [`Engine::shutdown`] cancels the shutdown token.
    pub fn start_background(&self, rt: &tokio::runtime::Handle) {
        let state = self.state().clone();
        let mut spawned: Vec<JoinHandle<()>> = Vec::new();

        // Automatic Z of a business day left open (producers FAQ
        // Q501c): at start-up, then every minute. A failure (FDM
        // unreachable) is retried on the next tick; meanwhile new
        // events wait (`commands::booking`).
        {
            let state = state.clone();
            let ticks = self.ticks.clone();
            spawned.push(rt.spawn(async move {
                let mut tick = tokio::time::interval(state.timers.auto_z);
                loop {
                    tokio::select! {
                        _ = state.shutdown_token.cancelled() => return,
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
            let state = state.clone();
            let ticks = self.ticks.clone();
            spawned.push(rt.spawn(async move {
                let mut tick = tokio::time::interval(state.timers.mail_outbox);
                loop {
                    tokio::select! {
                        _ = state.shutdown_token.cancelled() => return,
                        _ = tick.tick() => {
                            commands::delivery::run_outbox_once(&state).await;
                            ticks.mail_outbox.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                }
            }));
        }

        // Presence worker: signs the clock-ins that logins queue
        // (`commands::session`), one after another in arrival order, so
        // a login never waits for the FDM. The receiver is taken once:
        // a second start spawns nothing.
        if let Some(mut jobs) = state.session_rules.take_presence_jobs() {
            let state = state.clone();
            spawned.push(rt.spawn(async move {
                loop {
                    tokio::select! {
                        _ = state.shutdown_token.cancelled() => return,
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

        // The idle lock: waits for the activity clock, locks an
        // unattended session (`commands::session::lock`, reason idle).
        spawned.push(rt.spawn(run_idle_lock(state.clone())));

        // The HubRise poller and outbox; a build without the `hubrise`
        // feature has no background HTTP workers at all.
        #[cfg(feature = "hubrise")]
        {
            use std::sync::Arc;

            // The crate's run_loop returns when the shutdown token is
            // cancelled.
            let mut poller_ctx = commands::hubrise::poller_ctx(&state);
            let sink = state.events.clone();
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
                db: state.poller_db.clone(),
                wake: state.hubrise_outbox_wake.clone(),
                enabled: state.hubrise_switch.clone(),
            };
            let outbox_cancel = state.shutdown_token.clone();
            spawned.push(rt.spawn(async move {
                pos_hubrise::outbox::run_loop(outbox_ctx, outbox_cancel).await;
            }));
        }

        self.tasks.lock().expect("background tasks").extend(spawned);
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
        std::mem::take(&mut *self.tasks.lock().expect("background tasks"))
    }

    /// How many automatic Z passes have completed (for tests).
    pub fn auto_z_passes(&self) -> u64 {
        self.ticks.auto_z.load(Ordering::SeqCst)
    }

    /// How many mail outbox passes have completed (for tests).
    pub fn mail_outbox_passes(&self) -> u64 {
        self.ticks.mail_outbox.load(Ordering::SeqCst)
    }
}
