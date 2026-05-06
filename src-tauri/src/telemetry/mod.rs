use chrono::Utc;

use crate::commands::{get_account_status, list_api_keys};
use crate::core::models::{
    DiagnosticReport, ExportDiagnosticReportRequest, ReportRedactionInfo,
};
use crate::{platform, scanners};

pub async fn build_diagnostic_report(
    request: ExportDiagnosticReportRequest,
) -> anyhow::Result<DiagnosticReport> {
    let system_profile = if request.include_system_profile.unwrap_or(true) {
        Some(platform::system_profile().await?)
    } else {
        None
    };

    let quick_scan = match request.quick_scan {
        Some(scan_request) => Some(scanners::quick_scan(scan_request).await?),
        None => None,
    };

    let system_environment_scan = if request.include_system_environment_scan.unwrap_or(true) {
        Some(platform::system_environment_scan().await?)
    } else {
        None
    };

    let installer_scan = if request.include_installer_scan.unwrap_or(true) {
        Some(platform::installer_scan().await?)
    } else {
        None
    };

    let network_scan = if request.include_network_scan.unwrap_or(true) {
        let scan_request = request.network_scan.unwrap_or_default();
        Some(scanners::network_scan(scan_request).await?)
    } else {
        None
    };

    Ok(DiagnosticReport {
        generated_at: Utc::now().to_rfc3339(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        system_profile,
        quick_scan,
        system_environment_scan,
        installer_scan,
        network_scan,
        account_status: get_account_status().await.map_err(anyhow::Error::msg)?,
        api_keys: list_api_keys().await.map_err(anyhow::Error::msg)?,
        redaction: ReportRedactionInfo {
            sensitive_values_redacted: true,
            includes_raw_secrets: false,
        },
    })
}
