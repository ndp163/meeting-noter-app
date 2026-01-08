// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tokio::main]
async fn main() {
    // Logging is initialized in noter_lib::run()
    if let Err(e) = noter_lib::run().await {
        tracing::error!(error = %e, "Application error");
        std::process::exit(1);
    }
}
