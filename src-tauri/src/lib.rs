mod commands;
mod core;
mod msutools;
mod platform;
mod scanners;
mod storage;
mod telemetry;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    install_rustls_crypto_provider();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_system_profile,
            commands::run_system_environment_scan,
            commands::run_quick_scan,
            commands::run_quick_scan_streamed,
            commands::get_public_settings,
            commands::login,
            commands::login_2fa,
            commands::logout,
            commands::get_account_status,
            commands::list_api_keys,
            commands::create_api_key,
            commands::delete_api_key,
            commands::export_diagnostic_report,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run ai-scan tauri application");
}

fn install_rustls_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}
