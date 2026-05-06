use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, RootCertStore};
use serde_json::json;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_rustls::TlsConnector;
use url::Url;
use webpki_roots::TLS_SERVER_ROOTS;

use crate::core::models::{
    CheckStatus, Finding, QuickScanRequest, QuickScanResult, ScanCheck, ScanOverallStatus, Severity,
};
use crate::core::redaction;

pub const QUICK_SCAN_STEPS: [(&str, &str); 5] = [
    ("dns", "DNS 解析"),
    ("tcp", "TCP 连接"),
    ("tls", "TLS 证书"),
    ("http", "HTTP 响应"),
    ("models", "模型接口"),
];

pub async fn quick_scan(request: QuickScanRequest) -> anyhow::Result<QuickScanResult> {
    let started_at = Utc::now();
    let timeout_duration = Duration::from_millis(request.timeout_ms.unwrap_or(8_000).clamp(1_000, 30_000));
    let mut checks = Vec::new();
    let mut findings = Vec::new();
    let target = normalize_base_url(request.base_url.as_deref());

    let Some(base_url) = target.clone() else {
        checks.push(skipped_check(
            "target",
            "API 地址",
            "未填写 API 地址。",
        ));
        findings.push(Finding {
            id: "base_url_missing".to_string(),
            title: "API 地址缺失".to_string(),
            severity: Severity::Medium,
            message: "还没有配置要检测的 API 地址。".to_string(),
            next_step: "请填写 msutools 的 OpenAI 兼容 API 地址后重新体检。".to_string(),
            fix_suggestion: Some("请填写 msutools 的 OpenAI 兼容 API 地址后重新体检。".to_string()),
        });
        return Ok(QuickScanResult {
            target,
            started_at: started_at.to_rfc3339(),
            finished_at: Utc::now().to_rfc3339(),
            status: ScanOverallStatus::NotReady,
            scanned_at: Utc::now().to_rfc3339(),
            checks,
            findings,
        });
    };

    let parsed = Url::parse(&base_url)?;
    checks.push(check_dns(&parsed).await);
    checks.push(check_tcp(&parsed, timeout_duration).await);

    if parsed.scheme() == "https" {
        checks.push(check_tls(&parsed, timeout_duration).await);
    } else {
        checks.push(ScanCheck {
            id: "tls".to_string(),
            title: "TLS 证书".to_string(),
            status: CheckStatus::Skipped,
            severity: Severity::Low,
            message: "当前地址不是 HTTPS，已跳过 TLS 检查。".to_string(),
            evidence: json!({ "scheme": parsed.scheme() }),
            duration_ms: 0,
        });
    }

    checks.push(check_http_root(&base_url, timeout_duration).await);
    checks.push(check_models(&base_url, request.api_key.as_deref(), timeout_duration).await);

    findings.extend(findings_from_checks(&checks));

    let overall = overall_status(&checks);

    let finished_at = Utc::now().to_rfc3339();
    Ok(QuickScanResult {
        target: Some(base_url),
        started_at: started_at.to_rfc3339(),
        finished_at: finished_at.clone(),
        status: overall,
        scanned_at: finished_at,
        checks,
        findings,
    })
}

pub fn findings_from_checks(checks: &[ScanCheck]) -> Vec<Finding> {
    let mut findings = Vec::new();

    for check in checks {
        if matches!(check.status, CheckStatus::Fail | CheckStatus::Warn) {
            findings.push(Finding {
                id: format!("{}_{}", check.id, serde_status(&check.status)),
                title: check.title.clone(),
                severity: check.severity.clone(),
                message: check.message.clone(),
                next_step: suggestion_for(&check.id).unwrap_or_else(|| "请打开专业模式查看详细证据。".to_string()),
                fix_suggestion: suggestion_for(&check.id),
            });
        }
    }

    findings
}

pub fn overall_status(checks: &[ScanCheck]) -> ScanOverallStatus {
    if checks.iter().any(|check| matches!(check.status, CheckStatus::Fail)) {
        ScanOverallStatus::Failed
    } else if checks.iter().any(|check| matches!(check.status, CheckStatus::Warn | CheckStatus::Skipped)) {
        ScanOverallStatus::NeedsAttention
    } else {
        ScanOverallStatus::Passed
    }
}

pub fn normalize_base_url(raw: Option<&str>) -> Option<String> {
    let value = raw?.trim().trim_end_matches('/');
    if value.is_empty() {
        return None;
    }

    let with_scheme = if value.starts_with("http://") || value.starts_with("https://") {
        value.to_string()
    } else {
        format!("https://{value}")
    };

    Some(with_scheme.trim_end_matches("/v1").trim_end_matches('/').to_string())
}

pub async fn check_dns(url: &Url) -> ScanCheck {
    let started = Instant::now();
    let host = match url.host_str() {
        Some(host) => host,
        None => return failed_check("dns", "DNS 解析", "API 地址里没有域名。", json!({}), started),
    };
    let port = url.port_or_known_default().unwrap_or(443);

    match (host, port).to_socket_addrs() {
        Ok(addrs) => {
            let addresses: Vec<String> = addrs.map(|addr| addr.to_string()).collect();
            if addresses.is_empty() {
                failed_check("dns", "DNS 解析", "DNS 没有返回可用地址。", json!({ "host": host, "port": port }), started)
            } else {
                pass_check("dns", "DNS 解析", "DNS 解析成功。", json!({ "host": host, "port": port, "addresses": addresses }), started)
            }
        }
        Err(error) => failed_check("dns", "DNS 解析", "DNS 解析失败。", json!({ "host": host, "port": port, "error": error.to_string() }), started),
    }
}

pub async fn check_tcp(url: &Url, limit: Duration) -> ScanCheck {
    let started = Instant::now();
    let Some(addr) = first_socket_addr(url) else {
        return failed_check("tcp", "TCP 连接", "没有可用于连接的服务器地址。", json!({}), started);
    };

    match timeout(limit, TcpStream::connect(addr)).await {
        Ok(Ok(_stream)) => pass_check("tcp", "TCP 连接", "服务器端口可以连通。", json!({ "address": addr.to_string() }), started),
        Ok(Err(error)) => failed_check("tcp", "TCP 连接", "服务器端口连接失败。", json!({ "address": addr.to_string(), "error": error.to_string() }), started),
        Err(_) => failed_check("tcp", "TCP 连接", "服务器端口连接超时。", json!({ "address": addr.to_string(), "timeoutMs": limit.as_millis() }), started),
    }
}

pub async fn check_tls(url: &Url, limit: Duration) -> ScanCheck {
    let started = Instant::now();
    let Some(host) = url.host_str() else {
        return failed_check("tls", "TLS 证书", "API 地址里没有域名。", json!({}), started);
    };
    let Some(addr) = first_socket_addr(url) else {
        return failed_check("tls", "TLS 证书", "没有可用于 TLS 检查的服务器地址。", json!({ "host": host }), started);
    };

    let result = timeout(limit, async {
        let mut root_store = RootCertStore::empty();
        root_store.extend(TLS_SERVER_ROOTS.iter().cloned());
        let config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        let server_name = ServerName::try_from(host.to_string())?;
        let connector = TlsConnector::from(Arc::new(config));
        let stream = TcpStream::connect(addr).await?;
        let _tls_stream = connector.connect(server_name, stream).await?;
        anyhow::Ok(())
    })
    .await;

    match result {
        Ok(Ok(())) => pass_check("tls", "TLS 证书", "HTTPS 握手成功，证书链可用。", json!({ "host": host, "address": addr.to_string() }), started),
        Ok(Err(error)) => failed_check("tls", "TLS 证书", "HTTPS 握手失败，可能是证书、代理或系统时间问题。", json!({ "host": host, "address": addr.to_string(), "error": error.to_string() }), started),
        Err(_) => failed_check("tls", "TLS 证书", "HTTPS 握手超时。", json!({ "host": host, "address": addr.to_string(), "timeoutMs": limit.as_millis() }), started),
    }
}

pub async fn check_http_root(base_url: &str, limit: Duration) -> ScanCheck {
    let started = Instant::now();
    let client = match reqwest::Client::builder().timeout(limit).build() {
        Ok(client) => client,
        Err(error) => return failed_check("http", "HTTP 响应", "无法创建 HTTP 检查客户端。", json!({ "error": error.to_string() }), started),
    };

    match client.get(base_url).send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            let final_url = response.url().to_string();
            if response.status().is_success() || response.status().is_redirection() || status == 401 || status == 403 || status == 404 {
                pass_check("http", "HTTP 响应", "服务已经返回 HTTP 响应。", json!({ "status": status, "url": final_url }), started)
            } else {
                warn_check("http", "HTTP 响应", "服务返回了非预期 HTTP 状态。", json!({ "status": status, "url": final_url }), started)
            }
        }
        Err(error) => failed_check("http", "HTTP 响应", "HTTP 请求失败。", json!({ "url": base_url, "error": error.to_string() }), started),
    }
}

pub async fn check_models(base_url: &str, api_key: Option<&str>, limit: Duration) -> ScanCheck {
    let started = Instant::now();
    let Some(api_key) = api_key.map(str::trim).filter(|value| !value.is_empty()) else {
        return ScanCheck {
            id: "models".to_string(),
            title: "模型接口".to_string(),
            status: CheckStatus::Skipped,
            severity: Severity::Medium,
            message: "没有填写 API Key，已跳过 /v1/models 检查。".to_string(),
            evidence: json!({ "requiresApiKey": true }),
            duration_ms: started.elapsed().as_millis(),
        };
    };

    let url = format!("{}/v1/models", base_url.trim_end_matches('/'));
    let mut headers = HeaderMap::new();
    let bearer = format!("Bearer {}", api_key.trim_start_matches("Bearer ").trim());
    match HeaderValue::from_str(&bearer) {
        Ok(value) => {
            headers.insert(AUTHORIZATION, value);
        }
        Err(error) => {
            return failed_check("models", "模型接口", "API Key 无法作为 Authorization 请求头使用。", json!({ "error": error.to_string() }), started);
        }
    };

    let client = match reqwest::Client::builder()
        .timeout(limit)
        .default_headers(headers)
        .build()
    {
        Ok(client) => client,
        Err(error) => return failed_check("models", "模型接口", "无法创建模型接口检查客户端。", json!({ "error": error.to_string() }), started),
    };

    match client.get(&url).send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            let preview = redaction::redact_secret(&body.chars().take(240).collect::<String>());
            if (200..300).contains(&status) {
                pass_check("models", "模型接口", "/v1/models 返回成功，API Key 和模型列表接口可用。", json!({ "status": status, "bodyPreview": preview }), started)
            } else if status == 401 || status == 403 {
                failed_check("models", "模型接口", "/v1/models 拒绝了当前 API Key。", json!({ "status": status, "bodyPreview": preview }), started)
            } else {
                warn_check("models", "模型接口", "/v1/models 返回了非预期状态。", json!({ "status": status, "bodyPreview": preview }), started)
            }
        }
        Err(error) => failed_check("models", "模型接口", "/v1/models 请求失败。", json!({ "url": url, "error": error.to_string() }), started),
    }
}

fn first_socket_addr(url: &Url) -> Option<SocketAddr> {
    let host = url.host_str()?;
    let port = url.port_or_known_default()?;
    (host, port).to_socket_addrs().ok()?.next()
}

fn pass_check(id: &str, title: &str, message: &str, evidence: serde_json::Value, started: Instant) -> ScanCheck {
    ScanCheck {
        id: id.to_string(),
        title: title.to_string(),
        status: CheckStatus::Pass,
        severity: Severity::Info,
        message: message.to_string(),
        evidence,
        duration_ms: started.elapsed().as_millis(),
    }
}

fn warn_check(id: &str, title: &str, message: &str, evidence: serde_json::Value, started: Instant) -> ScanCheck {
    ScanCheck {
        id: id.to_string(),
        title: title.to_string(),
        status: CheckStatus::Warn,
        severity: Severity::Medium,
        message: message.to_string(),
        evidence,
        duration_ms: started.elapsed().as_millis(),
    }
}

fn failed_check(id: &str, title: &str, message: &str, evidence: serde_json::Value, started: Instant) -> ScanCheck {
    ScanCheck {
        id: id.to_string(),
        title: title.to_string(),
        status: CheckStatus::Fail,
        severity: Severity::High,
        message: message.to_string(),
        evidence,
        duration_ms: started.elapsed().as_millis(),
    }
}

pub fn skipped_check(id: &str, title: &str, message: &str) -> ScanCheck {
    ScanCheck {
        id: id.to_string(),
        title: title.to_string(),
        status: CheckStatus::Skipped,
        severity: Severity::Medium,
        message: message.to_string(),
        evidence: json!({}),
        duration_ms: 0,
    }
}

fn suggestion_for(id: &str) -> Option<String> {
    match id {
        "dns" => Some("请检查域名拼写、DNS、VPN 或系统代理配置。".to_string()),
        "tcp" => Some("请检查防火墙、代理、VPN，以及目标服务端口是否开放。".to_string()),
        "tls" => Some("请检查系统时间、证书信任、HTTPS 代理拦截或源站证书配置。".to_string()),
        "http" => Some("请确认 API 地址是否正确，以及该地址是否用于 OpenAI 兼容接口。".to_string()),
        "models" => Some("请确认 API Key 是否正确、账户是否可用，并确认服务暴露 GET /v1/models。".to_string()),
        _ => None,
    }
}

fn serde_status(status: &CheckStatus) -> &'static str {
    match status {
        CheckStatus::Pass => "pass",
        CheckStatus::Warn => "warn",
        CheckStatus::Fail => "fail",
        CheckStatus::Skipped => "skipped",
    }
}
