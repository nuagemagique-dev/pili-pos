#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// The Tauri-bound modules live in `lib.rs`; the fiscal core is the
// `pos-engine` crate.
use pili_pos::events::TauriEventSink;
use pili_pos::{cli, ipc, paths};
use pos_engine::engine::{Engine, EngineConfig, EnginePaths, EngineTimers, config_source_from};
use pos_engine::install;
use pos_engine::{HttpTls, SoftwareIdentity};

use std::sync::Arc;

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
            let problems = install::check_config(
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
            match install::init_db(&db_path, seed.as_deref()) {
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

    // The engine's events reach the webview through this sink; the
    // handle is set in `setup`, before the background loops start.
    let sink = Arc::new(TauriEventSink::new());

    let engine = Engine::open(EngineConfig {
        paths: EnginePaths::under(paths::data_dir()),
        config: config_source_from(
            std::env::var("PILI_POS_CONFIG").ok().as_deref(),
            std::path::Path::new(paths::PRODUCTION_CONFIG),
        ),
        events: sink.clone(),
        timers: EngineTimers::default(),
        // The version string the FDM and the tickets carry.
        identity: SoftwareIdentity::new(concat!("pili-pos/", env!("CARGO_PKG_VERSION")))
            .expect("software identity"),
        http_tls: HttpTls::Platform,
        // The pos.json value: the Linux kiosk has no override.
        device_id_override: None,
        support_program: paths::support_program(),
    })
    .expect("Failed to initialize app state");

    // Handles the health listener and the window need, taken BEFORE the
    // engine is moved into Tauri's managed state.
    let health_db = engine.poller_db.clone();
    let health_cancel = engine.shutdown_token.clone();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(engine)
        .setup(move |app| {
            use tauri::Manager;

            sink.set_handle(app.handle().clone());

            // Kiosk (deploy/pili-pos.service sets PILI_KIOSK=1): no
            // title bar, so no close or minimise button on the
            // touchscreen. tao draws its own header bar, so Cage's
            // server-side decoration flag cannot remove it. Development
            // keeps a normal window.
            if std::env::var_os("PILI_KIOSK").is_some()
                && let Some(window) = app.get_webview_window("main")
            {
                if let Err(e) = window.set_decorations(false) {
                    tracing::warn!(%e, "kiosk: could not remove the title bar");
                }
                if let Err(e) = window.set_fullscreen(true) {
                    tracing::warn!(%e, "kiosk: could not go fullscreen");
                }
            }

            // The automatic Z, the mail outbox and the HubRise workers
            // run in the engine (`pos_engine::background`), on Tauri's
            // runtime; their events come back through the sink above.
            app.state::<Engine>()
                .start_background(tauri::async_runtime::handle().inner());

            // Healthcheck: tiny HTTP server on localhost:9999 so
            // tsb-dashboard or a cron can verify the POS is alive.
            // Returns 200 + JSON with uptime info. Gracefully shuts
            // down with the same CancellationToken as the other workers.
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

            Ok(())
        })
        .on_window_event(move |window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                use tauri::Manager;
                window.app_handle().state::<Engine>().shutdown();
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
        ipc::menu::get_categories,
        ipc::menu::get_menu_items,
        // Orders
        ipc::order::create_order,
        ipc::order::add_order_line,
        ipc::order::remove_order_line,
        ipc::order::update_order_line_quantity,
        ipc::order::get_order,
        ipc::order::list_order_lines,
        ipc::order::list_open_orders,
        // Auth
        ipc::auth::login,
        ipc::auth::login_with_new_pin,
        ipc::auth::list_users,
        ipc::auth::create_user,
        ipc::auth::set_pin,
        ipc::auth::get_current_user,
        ipc::auth::logout,
        ipc::auth::deactivate_user,
        // Checkout
        ipc::checkout::checkout,
        ipc::unpaid::close_unpaid,
        ipc::eft::eft_pay,
        ipc::eft::eft_abort,
        ipc::eft::eft_check,
        ipc::eft::list_eft_payments,
        ipc::eft::check_payment_terminal,
        ipc::eft::list_unsettled_card_payments,
        ipc::checkout::check_fdm_status,
        ipc::fdm_monitor::fdm_health,
        ipc::fdm_monitor::fdm_check_now,
        // Ticket delivery (paper / email)
        ipc::delivery::check_mail_status,
        ipc::delivery::list_ticket_deliveries,
        ipc::delivery::print_missing_original,
        // Nightly export
        ipc::export::export_status,
        ipc::export::export_run_now,
        // Print
        ipc::print::check_printer,
        ipc::print::check_kitchen_printer,
        ipc::print::print_webshop_kitchen,
        ipc::print::print_webshop_customer,
        // Session
        ipc::session::set_active_order,
        ipc::session::leave_order,
        ipc::session::lock_session,
        ipc::session::user_activity,
        // Settings
        ipc::settings::get_settings,
        ipc::settings::set_training_mode,
        // Tables
        ipc::tables::list_tables,
        ipc::tables::get_table_plan,
        ipc::tables::set_table_plan,
        ipc::order::list_cart_lines,
        ipc::social::list_open_shifts,
        ipc::tables::list_open_takeaways,
        ipc::tables::sign_table_order,
        ipc::tables::sign_pre_bill,
        // Reports
        ipc::report::generate_x_report,
        ipc::report::generate_user_x_report,
        ipc::report::generate_z_report,
        ipc::report::list_z_reports,
        ipc::report::needs_z_report,
        ipc::report::backup_database,
        // Copy
        ipc::copy::copy_ticket,
        ipc::copy::list_fiscal_events,
        ipc::copy::list_copyable_tickets,
        ipc::support::remote_support_status,
        ipc::support::remote_support_running,
        ipc::support::start_remote_support,
        ipc::support::stop_remote_support,
        // Refund
        ipc::refund::list_refundable_events,
        ipc::refund::refund_checkout,
        // Social (Event S - ONSS clock in/out)
        ipc::social::work_in,
        ipc::social::work_out,
        ipc::social::clock_in_employee,
        ipc::social::clock_out_employee,
        ipc::social::end_shift,
        // Financial (Event F - cash in/out, drawer)
        ipc::financial::money_in_out,
        ipc::financial::open_drawer_fiscal,
        // HubRise
        ipc::hubrise::hubrise_get_connection_status,
        ipc::hubrise::hubrise_start_oauth_flow,
        ipc::hubrise::hubrise_complete_oauth,
        ipc::hubrise::hubrise_disconnect,
        ipc::hubrise::hubrise_list_webshop_orders,
        ipc::hubrise::hubrise_patch_order_status,
        ipc::hubrise::hubrise_set_confirmed_time,
        ipc::hubrise::hubrise_status,
        ipc::hubrise::hubrise_sync_catalog,
        ipc::hubrise::hubrise_adoption_preview,
        ipc::hubrise::hubrise_adopt_catalog,
        ipc::hubrise::hubrise_list_channels,
        ipc::hubrise::hubrise_set_channel,
        ipc::modifiers::get_item_modifiers,
        ipc::modifiers::add_order_line_with_modifiers,
        ipc::webshop::webshop_accept_order,
        ipc::webshop::webshop_handover_order,
        ipc::webshop::webshop_reject_order,
        ipc::webshop::webshop_refund_order,
        ipc::webshop::webshop_complete_order,
        ipc::webshop::webshop_close_cancelled_order,
        ipc::integrations::get_integrations,
        ipc::integrations::set_hubrise_enabled,
        // Peppol B2B invoicing (dev build only - not in the v1.0.0 cert scope)
        ipc::invoice::sign_invoice_fiscal,
        ipc::invoice::issue_b2b_invoice,
    ]);

    #[cfg(not(feature = "peppol"))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        // Menu
        ipc::menu::get_categories,
        ipc::menu::get_menu_items,
        // Orders
        ipc::order::create_order,
        ipc::order::add_order_line,
        ipc::order::remove_order_line,
        ipc::order::update_order_line_quantity,
        ipc::order::get_order,
        ipc::order::list_order_lines,
        ipc::order::list_open_orders,
        // Auth
        ipc::auth::login,
        ipc::auth::login_with_new_pin,
        ipc::auth::list_users,
        ipc::auth::create_user,
        ipc::auth::set_pin,
        ipc::auth::get_current_user,
        ipc::auth::logout,
        ipc::auth::deactivate_user,
        // Checkout
        ipc::checkout::checkout,
        ipc::unpaid::close_unpaid,
        ipc::eft::eft_pay,
        ipc::eft::eft_abort,
        ipc::eft::eft_check,
        ipc::eft::list_eft_payments,
        ipc::eft::check_payment_terminal,
        ipc::eft::list_unsettled_card_payments,
        ipc::checkout::check_fdm_status,
        ipc::fdm_monitor::fdm_health,
        ipc::fdm_monitor::fdm_check_now,
        // Ticket delivery (paper / email)
        ipc::delivery::check_mail_status,
        ipc::delivery::list_ticket_deliveries,
        ipc::delivery::print_missing_original,
        // Nightly export
        ipc::export::export_status,
        ipc::export::export_run_now,
        // Print
        ipc::print::check_printer,
        ipc::print::check_kitchen_printer,
        // Session
        ipc::session::set_active_order,
        ipc::session::leave_order,
        ipc::session::lock_session,
        ipc::session::user_activity,
        // Settings
        ipc::settings::get_settings,
        ipc::settings::set_training_mode,
        // Tables
        ipc::tables::list_tables,
        ipc::tables::get_table_plan,
        ipc::tables::set_table_plan,
        ipc::order::list_cart_lines,
        ipc::social::list_open_shifts,
        ipc::tables::list_open_takeaways,
        ipc::tables::sign_table_order,
        ipc::tables::sign_pre_bill,
        // Reports
        ipc::report::generate_x_report,
        ipc::report::generate_user_x_report,
        ipc::report::generate_z_report,
        ipc::report::list_z_reports,
        ipc::report::needs_z_report,
        ipc::report::backup_database,
        // Copy
        ipc::copy::copy_ticket,
        ipc::copy::list_fiscal_events,
        ipc::copy::list_copyable_tickets,
        ipc::support::remote_support_status,
        ipc::support::remote_support_running,
        ipc::support::start_remote_support,
        ipc::support::stop_remote_support,
        // Refund
        ipc::refund::list_refundable_events,
        ipc::refund::refund_checkout,
        // Social (Event S - ONSS clock in/out)
        ipc::social::work_in,
        ipc::social::work_out,
        ipc::social::clock_in_employee,
        ipc::social::clock_out_employee,
        ipc::social::end_shift,
        // Financial (Event F - cash in/out, drawer)
        ipc::financial::money_in_out,
        ipc::financial::open_drawer_fiscal,
        // No invoice (I event) commands: invoicing is not part of the
        // v1.0.0 certification scope.
        // HubRise (switched per installation, ipc::integrations)
        ipc::print::print_webshop_kitchen,
        ipc::print::print_webshop_customer,
        ipc::hubrise::hubrise_get_connection_status,
        ipc::hubrise::hubrise_start_oauth_flow,
        ipc::hubrise::hubrise_complete_oauth,
        ipc::hubrise::hubrise_disconnect,
        ipc::hubrise::hubrise_list_webshop_orders,
        ipc::hubrise::hubrise_patch_order_status,
        ipc::hubrise::hubrise_set_confirmed_time,
        ipc::hubrise::hubrise_status,
        ipc::hubrise::hubrise_sync_catalog,
        ipc::hubrise::hubrise_adoption_preview,
        ipc::hubrise::hubrise_adopt_catalog,
        ipc::hubrise::hubrise_list_channels,
        ipc::hubrise::hubrise_set_channel,
        ipc::modifiers::get_item_modifiers,
        ipc::modifiers::add_order_line_with_modifiers,
        ipc::webshop::webshop_accept_order,
        ipc::webshop::webshop_handover_order,
        ipc::webshop::webshop_reject_order,
        ipc::webshop::webshop_refund_order,
        ipc::webshop::webshop_complete_order,
        ipc::webshop::webshop_close_cancelled_order,
        ipc::integrations::get_integrations,
        ipc::integrations::set_hubrise_enabled,
    ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
