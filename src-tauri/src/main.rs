#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Modules live in `lib.rs` so integration tests under `tests/` can
// reach them via `use pili_pos::commands;`. The binary just re-imports
// them through the crate name - same code, same output.
use pili_pos::{cli, commands, paths, state};

#[cfg(feature = "hubrise")]
use std::sync::Arc;

use state::AppState;
use tauri::Emitter;

// Every shipped build has the HubRise integration (switched on or off
// per installation); the handler lists below assume it.
#[cfg(not(feature = "hubrise"))]
compile_error!("pili-pos is built with feature `hubrise` (`cert` and `peppol` include it)");

fn main() {
    // Offline commands (image build, install scripts) run before any
    // logging or UI setup and exit.
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli::parse(&args) {
        Ok(cli::Command::RunUi) => {}
        Ok(cli::Command::Version) => {
            println!("pili-pos {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        Ok(cli::Command::CheckConfig) => {
            let problems = cli::check_config(
                std::env::var("PILI_POS_CONFIG").ok().as_deref(),
                std::path::Path::new(paths::PRODUCTION_CONFIG),
            );
            if problems.is_empty() {
                println!("configuration OK");
                return;
            }
            for problem in &problems {
                eprintln!("{problem}");
            }
            std::process::exit(1);
        }
        Ok(cli::Command::InitDb { seed }) => {
            let db_path = paths::db_path();
            match cli::init_db(&db_path, seed.as_deref()) {
                Ok(()) => {
                    println!("database ready: {}", db_path.display());
                    return;
                }
                Err(e) => {
                    eprintln!("pili-pos --init-db: {e:#}");
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    }

    // Load .env file (if present) BEFORE anything reads env vars.
    // In production there's no .env - env vars are set by systemd.
    dotenvy::dotenv().ok();

    // Structured logging - daily-rotated log file + stderr for dev.
    // Must be initialized BEFORE anything that calls tracing macros.
    let logs_dir = paths::logs_dir();
    std::fs::create_dir_all(&logs_dir).ok();
    let file_appender = tracing_appender::rolling::daily(&logs_dir, "pos.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    tracing::info!("pili-pos starting");

    let app_state = AppState::new().expect("Failed to initialize app state");

    // Clone the handles we need for background tasks BEFORE moving
    // app_state into Tauri's managed state. The poller / outbox / health
    // handles only exist when the `hubrise` feature is enabled.
    #[cfg(feature = "hubrise")]
    let poller_db = app_state.poller_db.clone();
    #[cfg(feature = "hubrise")]
    let shared_http = app_state.shared_http.clone();
    #[cfg(feature = "hubrise")]
    let mut poller_ctx = commands::hubrise::poller_ctx(&app_state);
    #[cfg(feature = "hubrise")]
    let outbox_wake = app_state.hubrise_outbox_wake.clone();
    #[cfg(feature = "hubrise")]
    let hubrise_switch = app_state.hubrise_switch.clone();
    let shutdown_for_setup = app_state.shutdown_token.clone();
    let shutdown_for_window = app_state.shutdown_token.clone();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .setup(move |_app| {
            // Warnings and informations the FDM attaches to a signed
            // event go to the screen, whichever command signed
            // (POS description v2.2, Message).
            {
                let handle = _app.handle().clone();
                pos_fiscal::fdm_client::on_sign_messages(move |messages| {
                    let _ = handle.emit("fdm-sign-messages", messages);
                });
            }

            // Kiosk (deploy/pili-pos.service sets PILI_KIOSK=1): no
            // title bar, so no close or minimise button on the
            // touchscreen. tao draws its own header bar, so Cage's
            // server-side decoration flag cannot remove it. Development
            // keeps a normal window.
            if std::env::var_os("PILI_KIOSK").is_some() {
                use tauri::Manager;
                if let Some(window) = _app.get_webview_window("main") {
                    if let Err(e) = window.set_decorations(false) {
                        tracing::warn!(%e, "kiosk: could not remove the title bar");
                    }
                    if let Err(e) = window.set_fullscreen(true) {
                        tracing::warn!(%e, "kiosk: could not go fullscreen");
                    }
                }
            }

            // Automatic Z of a business day left open (producers FAQ
            // Q501c): at start-up, then every minute. A failure (FDM
            // unreachable) is retried on the next tick; meanwhile new
            // events wait (`commands::booking`).
            {
                use tauri::Manager;
                let handle = _app.handle().clone();
                let cancel = shutdown_for_setup.clone();
                tauri::async_runtime::spawn(async move {
                    let mut tick = tokio::time::interval(std::time::Duration::from_secs(60));
                    loop {
                        tokio::select! {
                            _ = cancel.cancelled() => return,
                            _ = tick.tick() => {
                                let state = handle.state::<AppState>();
                                if let Err(e) = commands::report::close_overdue_day(state.inner()).await {
                                    tracing::warn!(%e, "automatic Z not made yet");
                                }
                            }
                        }
                    }
                });
            }

            // Digital ticket outbox: retries queued emails and clears
            // addresses after 72 h. Runs in every build; it does
            // nothing when no email client is configured.
            {
                use tauri::Manager;
                let handle = _app.handle().clone();
                let cancel = shutdown_for_setup.clone();
                tauri::async_runtime::spawn(async move {
                    let mut tick =
                        tokio::time::interval(commands::delivery::OUTBOX_INTERVAL);
                    loop {
                        tokio::select! {
                            _ = cancel.cancelled() => return,
                            _ = tick.tick() => {
                                let state = handle.state::<AppState>();
                                commands::delivery::run_outbox_once(state.inner()).await;
                            }
                        }
                    }
                });
            }

            // The HubRise poller, outbox, and healthcheck all live in
            // this block; the certification build (no `hubrise` feature)
            // ships a kiosk with no background HTTP workers at all.
            #[cfg(feature = "hubrise")]
            {
                let handle = _app.handle().clone();
                // Spawn the HubRise poller background task. The crate's
                // run_loop returns when `shutdown_token` is cancelled.
                poller_ctx.on_outcome = Some(Arc::new(move |outcome, event| {
                    use pos_hubrise::dispatcher::DispatchOutcome;
                    let topic = match outcome {
                        DispatchOutcome::OrderCreated => "webshop-order-received",
                        DispatchOutcome::OrderUpdated => "webshop-order-updated",
                        DispatchOutcome::CatalogUpdated => "catalog-updated",
                        DispatchOutcome::CatalogRejected => "catalog-rejected",
                        DispatchOutcome::CatalogAwaitingAdoption => "catalog-awaiting-adoption",
                        DispatchOutcome::Ignored => return,
                    };
                    let _ = handle.emit(topic, event);
                }));
                let poller_cancel = shutdown_for_setup.clone();
                tauri::async_runtime::spawn(async move {
                    pos_hubrise::poller::run_loop(poller_ctx, poller_cancel).await;
                });

                // Clone poller_db for the healthcheck BEFORE moving it
                // into the outbox context.
                let health_db = poller_db.clone();

                // Outbox worker: drains POS → HubRise order pushes that
                // were enqueued by checkout. Same pattern as the poller:
                // background loop, short DB locks, async HTTP, graceful
                // shutdown via the shared CancellationToken.
                let outbox_ctx = pos_hubrise::outbox::OutboxCtx {
                    http: shared_http,
                    db: poller_db,
                    wake: outbox_wake,
                    enabled: hubrise_switch,
                };
                let outbox_cancel = shutdown_for_setup.clone();
                tauri::async_runtime::spawn(async move {
                    pos_hubrise::outbox::run_loop(outbox_ctx, outbox_cancel).await;
                });

                // Healthcheck: tiny HTTP server on localhost:9999 so
                // tsb-dashboard or a cron can verify the POS is alive.
                // Returns 200 + JSON with uptime info. Gracefully shuts
                // down with the same CancellationToken as the other workers.
                let health_cancel = shutdown_for_setup.clone();
                let start_time = std::time::Instant::now();
                tauri::async_runtime::spawn(async move {
                    use tokio::io::AsyncWriteExt;
                    let listener = match tokio::net::TcpListener::bind("127.0.0.1:9999").await {
                        Ok(l) => l,
                        Err(e) => {
                            tracing::warn!(%e, "healthcheck: failed to bind :9999");
                            return;
                        }
                    };
                    tracing::info!("healthcheck listening on http://127.0.0.1:9999/health");
                    loop {
                        tokio::select! {
                            _ = health_cancel.cancelled() => return,
                            accept = listener.accept() => {
                                if let Ok((mut stream, _)) = accept {
                                    let uptime_secs = start_time.elapsed().as_secs();
                                    let outbox_pending: i64 = {
                                        let db = health_db.lock().expect("health db");
                                        db.query_row(
                                            "SELECT COUNT(*) FROM hubrise_outbox WHERE sent_at IS NULL AND parked_at IS NULL",
                                            [], |r| r.get(0),
                                        ).unwrap_or(0)
                                    };
                                    let body = format!(
                                        r#"{{"status":"ok","uptime_secs":{uptime_secs},"outbox_pending":{outbox_pending}}}"#
                                    );
                                    let resp = format!(
                                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                        body.len(), body
                                    );
                                    let _ = stream.write_all(resp.as_bytes()).await;
                                    let _ = stream.shutdown().await;
                                }
                            }
                        }
                    }
                });
            }

            Ok(())
        })
        .on_window_event(move |_window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                shutdown_for_window.cancel();
            }
        });

    // The invoke_handler list differs between the dev build (with the
    // invoicing commands, feature `peppol`) and the certification build
    // (without). tauri::generate_handler! doesn't accept `#[cfg]` on
    // individual commands, so we branch the whole call. HubRise is in
    // both: whether it runs is the installation's switch.
    #[cfg(feature = "peppol")]
    let builder = builder.invoke_handler(tauri::generate_handler![
        // Menu
        commands::menu::get_categories,
        commands::menu::get_menu_items,
        // Orders
        commands::order::create_order,
        commands::order::add_order_line,
        commands::order::remove_order_line,
        commands::order::update_order_line_quantity,
        commands::order::get_order,
        commands::order::list_order_lines,
        commands::order::list_open_orders,
        // Auth
        commands::auth::login,
        commands::auth::login_with_new_pin,
        commands::auth::list_users,
        commands::auth::create_user,
        commands::auth::set_pin,
        commands::auth::get_current_user,
        commands::auth::logout,
        commands::auth::deactivate_user,
        // Checkout
        commands::checkout::checkout,
        commands::unpaid::close_unpaid,
        commands::checkout::check_fdm_status,
        // Ticket delivery (paper / email)
        commands::delivery::check_mail_status,
        commands::delivery::list_ticket_deliveries,
        commands::delivery::print_missing_original,
        // Print
        commands::print::check_printer,
        commands::print::check_kitchen_printer,
        commands::print::print_webshop_kitchen,
        commands::print::print_webshop_customer,
        // Settings
        commands::settings::get_settings,
        commands::settings::set_training_mode,
        // Tables
        commands::tables::list_tables,
        commands::order::list_cart_lines,
        commands::social::list_open_shifts,
        commands::tables::list_open_takeaways,
        commands::tables::sign_table_order,
        commands::tables::sign_pre_bill,
        // Reports
        commands::report::generate_x_report,
        commands::report::generate_user_x_report,
        commands::report::generate_z_report,
        commands::report::list_z_reports,
        commands::report::needs_z_report,
        commands::report::backup_database,
        // Copy
        commands::copy::copy_ticket,
        commands::copy::list_fiscal_events,
        commands::copy::list_copyable_tickets,
        commands::support::remote_support_status,
        commands::support::remote_support_running,
        commands::support::start_remote_support,
        commands::support::stop_remote_support,
        // Refund
        commands::refund::list_refundable_events,
        commands::refund::refund_checkout,
        // Social (Event S - ONSS clock in/out)
        commands::social::work_in,
        commands::social::work_out,
        // Financial (Event F - cash in/out, drawer)
        commands::financial::money_in_out,
        commands::financial::open_drawer_fiscal,
        // HubRise
        commands::hubrise::hubrise_get_connection_status,
        commands::hubrise::hubrise_start_oauth_flow,
        commands::hubrise::hubrise_complete_oauth,
        commands::hubrise::hubrise_disconnect,
        commands::hubrise::hubrise_list_webshop_orders,
        commands::hubrise::hubrise_patch_order_status,
        commands::hubrise::hubrise_set_confirmed_time,
        commands::hubrise::hubrise_status,
        commands::hubrise::hubrise_sync_catalog,
        commands::hubrise::hubrise_adoption_preview,
        commands::hubrise::hubrise_adopt_catalog,
        commands::hubrise::hubrise_list_channels,
        commands::hubrise::hubrise_set_channel,
        commands::modifiers::get_item_modifiers,
        commands::modifiers::add_order_line_with_modifiers,
        commands::webshop::webshop_accept_order,
        commands::webshop::webshop_handover_order,
        commands::webshop::webshop_reject_order,
        commands::webshop::webshop_refund_order,
        commands::webshop::webshop_complete_order,
        commands::webshop::webshop_close_cancelled_order,
        commands::integrations::get_integrations,
        commands::integrations::set_hubrise_enabled,
        // Peppol B2B invoicing (dev build only - not in the v1.0.0 cert scope)
        commands::invoice::sign_invoice_fiscal,
        commands::invoice::issue_b2b_invoice,
    ]);

    #[cfg(not(feature = "peppol"))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        // Menu
        commands::menu::get_categories,
        commands::menu::get_menu_items,
        // Orders
        commands::order::create_order,
        commands::order::add_order_line,
        commands::order::remove_order_line,
        commands::order::update_order_line_quantity,
        commands::order::get_order,
        commands::order::list_order_lines,
        commands::order::list_open_orders,
        // Auth
        commands::auth::login,
        commands::auth::login_with_new_pin,
        commands::auth::list_users,
        commands::auth::create_user,
        commands::auth::set_pin,
        commands::auth::get_current_user,
        commands::auth::logout,
        commands::auth::deactivate_user,
        // Checkout
        commands::checkout::checkout,
        commands::unpaid::close_unpaid,
        commands::checkout::check_fdm_status,
        // Ticket delivery (paper / email)
        commands::delivery::check_mail_status,
        commands::delivery::list_ticket_deliveries,
        commands::delivery::print_missing_original,
        // Print
        commands::print::check_printer,
        commands::print::check_kitchen_printer,
        // Settings
        commands::settings::get_settings,
        commands::settings::set_training_mode,
        // Tables
        commands::tables::list_tables,
        commands::order::list_cart_lines,
        commands::social::list_open_shifts,
        commands::tables::list_open_takeaways,
        commands::tables::sign_table_order,
        commands::tables::sign_pre_bill,
        // Reports
        commands::report::generate_x_report,
        commands::report::generate_user_x_report,
        commands::report::generate_z_report,
        commands::report::list_z_reports,
        commands::report::needs_z_report,
        commands::report::backup_database,
        // Copy
        commands::copy::copy_ticket,
        commands::copy::list_fiscal_events,
        commands::copy::list_copyable_tickets,
        commands::support::remote_support_status,
        commands::support::remote_support_running,
        commands::support::start_remote_support,
        commands::support::stop_remote_support,
        // Refund
        commands::refund::list_refundable_events,
        commands::refund::refund_checkout,
        // Social (Event S - ONSS clock in/out)
        commands::social::work_in,
        commands::social::work_out,
        // Financial (Event F - cash in/out, drawer)
        commands::financial::money_in_out,
        commands::financial::open_drawer_fiscal,
        // No invoice (I event) commands: invoicing is not part of the
        // v1.0.0 certification scope.
        // HubRise (switched per installation, commands::integrations)
        commands::print::print_webshop_kitchen,
        commands::print::print_webshop_customer,
        commands::hubrise::hubrise_get_connection_status,
        commands::hubrise::hubrise_start_oauth_flow,
        commands::hubrise::hubrise_complete_oauth,
        commands::hubrise::hubrise_disconnect,
        commands::hubrise::hubrise_list_webshop_orders,
        commands::hubrise::hubrise_patch_order_status,
        commands::hubrise::hubrise_set_confirmed_time,
        commands::hubrise::hubrise_status,
        commands::hubrise::hubrise_sync_catalog,
        commands::hubrise::hubrise_adoption_preview,
        commands::hubrise::hubrise_adopt_catalog,
        commands::hubrise::hubrise_list_channels,
        commands::hubrise::hubrise_set_channel,
        commands::modifiers::get_item_modifiers,
        commands::modifiers::add_order_line_with_modifiers,
        commands::webshop::webshop_accept_order,
        commands::webshop::webshop_handover_order,
        commands::webshop::webshop_reject_order,
        commands::webshop::webshop_refund_order,
        commands::webshop::webshop_complete_order,
        commands::webshop::webshop_close_cancelled_order,
        commands::integrations::get_integrations,
        commands::integrations::set_hubrise_enabled,
    ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
