//! The background loops of the engine: the automatic Z, the digital
//! ticket outbox and, with the `hubrise` feature, the HubRise poller and
//! outbox. The shell calls [`Engine::start_background`] once the runtime
//! is up and [`Engine::shutdown`] when it closes; the loops run on the
//! runtime the shell gives and report through the engine's
//! [`crate::events::EventSink`].

use std::sync::atomic::Ordering;

use tokio::task::JoinHandle;

use crate::commands;
use crate::engine::Engine;
use crate::events::EngineEvent;

impl Engine {
    /// Spawn the loops on `rt`: all of them stop when
    /// [`Engine::shutdown`] cancels the shutdown token.
    ///
    /// Also registers the FDM client's sign-message hook so the
    /// warnings and informations the FDM attaches to a signed event
    /// reach the sink (POS description v2.2, Message). That hook is a
    /// process-wide global until phase P1a-T8: when one is already set
    /// in this process, it stays.
    pub fn start_background(&self, rt: &tokio::runtime::Handle) {
        let state = self.state().clone();
        let mut spawned: Vec<JoinHandle<()>> = Vec::new();

        {
            let sink = state.events.clone();
            let registered = pos_fiscal::fdm_client::on_sign_messages(move |messages| {
                sink.emit(EngineEvent::FdmSignMessages(messages.clone()));
            });
            if !registered {
                tracing::debug!("sign-message hook already set in this process; keeping it");
            }
        }

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
