use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::Utc;
use tauri::Emitter;
use url::Url;

use crate::core::models::{
    AccountStatus, ApiKeyList, AvailableGroup, CreatedApiKey, CreateApiKeyRequest, DeleteApiKeyRequest,
    DiagnosticReport, ExportDiagnosticReportRequest, InstallerScanResult, Login2faRequest,
    LoginRequest, LoginResult, NetworkScanRequest, NetworkScanResult, PublicSettings,
    QuickScanRequest, QuickScanResult, ScanCheck, ScanOverallStatus, ScanProgressEvent,
    ScanProgressPhase, SystemEnvironmentScanResult, SystemProfile,
};
use crate::{msutools, platform, scanners, telemetry};

static QUICK_SCAN_CANCELLED: AtomicBool = AtomicBool::new(false);

#[tauri::command]
pub async fn get_system_profile() -> Result<SystemProfile, String> {
    platform::system_profile().await.map_err(to_command_error)
}

#[tauri::command]
pub async fn run_system_environment_scan() -> Result<SystemEnvironmentScanResult, String> {
    platform::system_environment_scan()
        .await
        .map_err(to_command_error)
}

#[tauri::command]
pub async fn run_installer_scan() -> Result<InstallerScanResult, String> {
    platform::installer_scan().await.map_err(to_command_error)
}

#[tauri::command]
pub async fn run_network_scan(request: Option<NetworkScanRequest>) -> Result<NetworkScanResult, String> {
    let request = request.unwrap_or_else(|| NetworkScanRequest {
        base_url: Some(msutools::base_url().to_string()),
        ..NetworkScanRequest::default()
    });
    scanners::network_scan(request).await.map_err(to_command_error)
}

#[tauri::command]
pub async fn run_quick_scan(request: Option<QuickScanRequest>) -> Result<QuickScanResult, String> {
    let request = request.unwrap_or_else(|| QuickScanRequest {
        base_url: Some(msutools::base_url().to_string()),
        ..QuickScanRequest::default()
    });
    scanners::quick_scan(request).await.map_err(to_command_error)
}

#[tauri::command]
pub async fn run_quick_scan_streamed(
    app: tauri::AppHandle,
    request: Option<QuickScanRequest>,
) -> Result<QuickScanResult, String> {
    QUICK_SCAN_CANCELLED.store(false, Ordering::SeqCst);
    let request = request.unwrap_or_else(|| QuickScanRequest {
        base_url: Some(msutools::base_url().to_string()),
        ..QuickScanRequest::default()
    });
    streamed_quick_scan(app, request)
        .await
        .map_err(to_command_error)
}

#[tauri::command]
pub async fn stop_quick_scan() -> Result<(), String> {
    QUICK_SCAN_CANCELLED.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub async fn get_public_settings() -> Result<PublicSettings, String> {
    msutools::public_settings().await.map_err(to_command_error)
}

#[tauri::command]
pub async fn login(request: LoginRequest) -> Result<LoginResult, String> {
    msutools::login(request).await.map_err(to_command_error)
}

#[tauri::command]
pub async fn login_2fa(request: Login2faRequest) -> Result<LoginResult, String> {
    msutools::login_2fa(request).await.map_err(to_command_error)
}

#[tauri::command]
pub async fn logout() -> Result<(), String> {
    msutools::logout().await.map_err(to_command_error)
}

#[tauri::command]
pub async fn get_account_status() -> Result<AccountStatus, String> {
    msutools::account_status().await.map_err(to_command_error)
}

#[tauri::command]
pub async fn list_api_keys() -> Result<ApiKeyList, String> {
    msutools::list_api_keys().await.map_err(to_command_error)
}

#[tauri::command]
pub async fn list_available_groups() -> Result<Vec<AvailableGroup>, String> {
    msutools::available_groups().await.map_err(to_command_error)
}

#[tauri::command]
pub async fn create_api_key(request: CreateApiKeyRequest) -> Result<CreatedApiKey, String> {
    msutools::create_api_key(request).await.map_err(to_command_error)
}

#[tauri::command]
pub async fn delete_api_key(request: DeleteApiKeyRequest) -> Result<String, String> {
    msutools::delete_api_key(request).await.map_err(to_command_error)
}

#[tauri::command]
pub async fn export_diagnostic_report(
    request: ExportDiagnosticReportRequest,
) -> Result<DiagnosticReport, String> {
    telemetry::build_diagnostic_report(request)
        .await
        .map_err(to_command_error)
}

fn to_command_error(error: anyhow::Error) -> String {
    error.to_string()
}

async fn streamed_quick_scan(
    app: tauri::AppHandle,
    request: QuickScanRequest,
) -> anyhow::Result<QuickScanResult> {
    let run_id = format!("scan-{}", Utc::now().timestamp_millis());
    let started_at = Utc::now();
    let total = scanners::QUICK_SCAN_STEPS.len();
    let timeout_duration =
        Duration::from_millis(request.timeout_ms.unwrap_or(8_000).clamp(1_000, 30_000));
    let target = scanners::normalize_base_url(request.base_url.as_deref());
    let mut checks: Vec<ScanCheck> = Vec::new();

    emit_scan_progress(
        &app,
        &run_id,
        ScanProgressPhase::Started,
        0,
        total,
        None,
        "开始体检。".to_string(),
    );

    let Some(base_url) = target.clone() else {
        let check = scanners::skipped_check(
            "target",
            "API 地址",
            "未填写 API 地址。",
        );
        checks.push(check.clone());
        emit_scan_progress(
            &app,
            &run_id,
            ScanProgressPhase::StepFinished,
            1,
            total,
            Some(check),
            "API 地址为空，无法继续体检。".to_string(),
        );
        let finished_at = Utc::now().to_rfc3339();
        let result = QuickScanResult {
            target,
            started_at: started_at.to_rfc3339(),
            finished_at: finished_at.clone(),
            status: ScanOverallStatus::NotReady,
            scanned_at: finished_at,
            checks,
            findings: scanners::findings_from_checks(&[]),
        };
        emit_finished(&app, &run_id, &result, total);
        return Ok(result);
    };

    let parsed = match Url::parse(&base_url) {
        Ok(parsed) => parsed,
        Err(error) => {
            emit_scan_progress(
                &app,
                &run_id,
                ScanProgressPhase::Failed,
                checks.len(),
                total,
                None,
                format!("API 地址格式错误：{error}"),
            );
            return Err(error.into());
        }
    };

    if let Some(result) = maybe_cancel_scan(&app, &run_id, &target, &started_at, &checks, total) {
        return Ok(result);
    }

    emit_step_started(&app, &run_id, checks.len(), total, "dns", "DNS 解析");
    let dns = scanners::check_dns(&parsed).await;
    checks.push(dns.clone());
    emit_step_finished(&app, &run_id, checks.len(), total, dns);

    if let Some(result) = maybe_cancel_scan(&app, &run_id, &target, &started_at, &checks, total) {
        return Ok(result);
    }

    emit_step_started(&app, &run_id, checks.len(), total, "tcp", "TCP 连接");
    let tcp = scanners::check_tcp(&parsed, timeout_duration).await;
    checks.push(tcp.clone());
    emit_step_finished(&app, &run_id, checks.len(), total, tcp);

    if let Some(result) = maybe_cancel_scan(&app, &run_id, &target, &started_at, &checks, total) {
        return Ok(result);
    }

    emit_step_started(&app, &run_id, checks.len(), total, "tls", "TLS 证书");
    let tls = if parsed.scheme() == "https" {
        scanners::check_tls(&parsed, timeout_duration).await
    } else {
        scanners::skipped_check(
            "tls",
            "TLS 证书",
            "当前地址不是 HTTPS，已跳过 TLS 检查。",
        )
    };
    checks.push(tls.clone());
    emit_step_finished(&app, &run_id, checks.len(), total, tls);

    if let Some(result) = maybe_cancel_scan(&app, &run_id, &target, &started_at, &checks, total) {
        return Ok(result);
    }

    emit_step_started(&app, &run_id, checks.len(), total, "http", "HTTP 响应");
    let http = scanners::check_http_root(&base_url, timeout_duration).await;
    checks.push(http.clone());
    emit_step_finished(&app, &run_id, checks.len(), total, http);

    if let Some(result) = maybe_cancel_scan(&app, &run_id, &target, &started_at, &checks, total) {
        return Ok(result);
    }

    emit_step_started(&app, &run_id, checks.len(), total, "models", "模型接口");
    let models =
        scanners::check_models(&base_url, request.api_key.as_deref(), timeout_duration).await;
    checks.push(models.clone());
    emit_step_finished(&app, &run_id, checks.len(), total, models);

    let findings = scanners::findings_from_checks(&checks);
    let status = scanners::overall_status(&checks);
    let finished_at = Utc::now().to_rfc3339();
    let result = QuickScanResult {
        target: Some(base_url),
        started_at: started_at.to_rfc3339(),
        finished_at: finished_at.clone(),
        status,
        scanned_at: finished_at,
        checks,
        findings,
    };
    emit_finished(&app, &run_id, &result, total);
    Ok(result)
}

fn maybe_cancel_scan(
    app: &tauri::AppHandle,
    run_id: &str,
    target: &Option<String>,
    started_at: &chrono::DateTime<Utc>,
    checks: &[ScanCheck],
    total: usize,
) -> Option<QuickScanResult> {
    if !QUICK_SCAN_CANCELLED.load(Ordering::SeqCst) {
        return None;
    }

    let mut partial_checks = checks.to_vec();
    partial_checks.push(scanners::skipped_check(
        "canceled",
        "扫描控制",
        "用户已停止本次扫描。",
    ));
    let mut findings = scanners::findings_from_checks(&partial_checks);
    findings.push(crate::core::models::Finding {
        id: "scan_canceled".to_string(),
        title: "扫描已停止".to_string(),
        severity: crate::core::models::Severity::Info,
        message: "本次扫描由用户主动停止，结果只包含已经完成的检查项。".to_string(),
        next_step: "如需完整诊断，请重新点击开始扫描。".to_string(),
        fix_suggestion: Some("重新运行全盘扫描可以获得完整诊断结果。".to_string()),
    });
    let finished_at = Utc::now().to_rfc3339();
    let result = QuickScanResult {
        target: target.clone(),
        started_at: started_at.to_rfc3339(),
        finished_at: finished_at.clone(),
        status: ScanOverallStatus::NeedsAttention,
        scanned_at: finished_at,
        checks: partial_checks,
        findings,
    };

    emit_scan_progress(
        app,
        run_id,
        ScanProgressPhase::Canceled,
        checks.len(),
        total,
        None,
        "扫描已停止。".to_string(),
    );
    QUICK_SCAN_CANCELLED.store(false, Ordering::SeqCst);
    Some(result)
}

fn emit_step_started(
    app: &tauri::AppHandle,
    run_id: &str,
    completed: usize,
    total: usize,
    id: &str,
    title: &str,
) {
    emit_scan_progress(
        app,
        run_id,
        ScanProgressPhase::StepStarted,
        completed,
        total,
        Some(ScanCheck {
            id: id.to_string(),
            title: title.to_string(),
            status: crate::core::models::CheckStatus::Skipped,
            severity: crate::core::models::Severity::Info,
            message: "正在检查。".to_string(),
            evidence: serde_json::json!({}),
            duration_ms: 0,
        }),
        format!("正在检查 {title}。"),
    );
}

fn emit_step_finished(
    app: &tauri::AppHandle,
    run_id: &str,
    completed: usize,
    total: usize,
    check: ScanCheck,
) {
    let message = format!("{}：{}", check.title, check.message);
    emit_scan_progress(
        app,
        run_id,
        ScanProgressPhase::StepFinished,
        completed,
        total,
        Some(check),
        message,
    );
}

fn emit_finished(
    app: &tauri::AppHandle,
    run_id: &str,
    result: &QuickScanResult,
    total: usize,
) {
    emit_scan_progress(
        app,
        run_id,
        ScanProgressPhase::Finished,
        total,
        total,
        None,
        format!("体检完成：{}。", overall_status_text(&result.status)),
    );
}

fn emit_scan_progress(
    app: &tauri::AppHandle,
    run_id: &str,
    phase: ScanProgressPhase,
    completed: usize,
    total: usize,
    check: Option<ScanCheck>,
    message: String,
) {
    let progress = if total == 0 {
        0
    } else {
        ((completed.min(total) * 100) / total) as u8
    };
    let current_step_id = check.as_ref().map(|item| item.id.clone());
    let current_step_title = check.as_ref().map(|item| item.title.clone());
    let event = ScanProgressEvent {
        run_id: run_id.to_string(),
        phase,
        progress,
        completed: completed.min(total),
        total,
        current_step_id,
        current_step_title,
        message,
        check,
        emitted_at: Utc::now().to_rfc3339(),
    };
    let _ = app.emit("quick-scan-progress", event);
}

fn overall_status_text(status: &ScanOverallStatus) -> &'static str {
    match status {
        ScanOverallStatus::Passed => "可使用",
        ScanOverallStatus::NeedsAttention => "需要关注",
        ScanOverallStatus::Failed => "需要处理",
        ScanOverallStatus::NotReady => "未就绪",
    }
}
