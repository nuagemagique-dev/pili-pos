//! `pos-eft-mock`: the payment terminal mock as a standalone process,
//! for demos and the emulator. Configuration through the environment:
//! `PILI_EFT_MOCK_PORT` (HTTP, 19400), `PILI_EFT_MOCK_OPI_PORT` (OPI
//! terminal, 19410), `PILI_EFT_MOCK_OPI_DEVICE_PORT` (the till's channel 1
//! port the terminal prints to, 4102), `PILI_EFT_MOCK_API_KEY` (optional).

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a standalone tool whose output is the console"
)]

use std::process::ExitCode;

fn main() -> ExitCode {
    let config = match pos_eft_mock::Config::from_env() {
        Ok(c) => c,
        Err(message) => {
            eprintln!("pos-eft-mock: {message}");
            return ExitCode::from(2);
        }
    };
    let device_port = config.opi_device_port;
    let mock = match pos_eft_mock::Mock::start_on(config) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("pos-eft-mock: cannot listen: {e}");
            return ExitCode::from(1);
        }
    };
    println!(
        "pos-eft-mock HTTP on {} (base_url of worldline_terminal_api and ccv_payment_api, payments page)",
        mock.url()
    );
    println!(
        "pos-eft-mock OPI terminal on {} (address of ccv_opi), printing to the till's port {device_port}",
        mock.opi_addr(),
    );
    println!(
        "Next payment: POST {}/mock/next {{\"result\":\"approve|decline|timeout|busy|unknown\",\"brand\":\"Visa\",\"delay_ms\":1500}}",
        mock.url()
    );
    println!("Press Ctrl+C to stop");
    // The server runs on its own thread; keep this one parked until killed.
    loop {
        std::thread::park();
    }
}
