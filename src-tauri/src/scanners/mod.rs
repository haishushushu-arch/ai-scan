use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, RootCertStore};
use serde::Deserialize;
use serde_json::json;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_rustls::TlsConnector;
use url::Url;
use webpki_roots::TLS_SERVER_ROOTS;

use crate::core::models::{
    CheckStatus, Finding, NetworkHeader, NetworkHttpProbe, NetworkIpInfo, NetworkScanRequest,
    NetworkScanResult, NetworkServerIp, QuickScanRequest, QuickScanResult, ScanCheck,
    ScanOverallStatus, Severity,
};
use crate::core::redaction;

pub const QUICK_SCAN_STEPS: [(&str, &str); 5] = [
    ("dns", "DNS 解析"),
    ("tcp", "TCP 连接"),
    ("tls", "TLS 证书"),
    ("http", "HTTP 响应"),
    ("models", "模型接口"),
];

const NETWORK_USER_AGENT: &str = "AI-SCAN/0.1.0 msutools diagnostics";

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

pub async fn network_scan(request: NetworkScanRequest) -> anyhow::Result<NetworkScanResult> {
    let scanned_at = Utc::now().to_rfc3339();
    let timeout_duration =
        Duration::from_millis(request.timeout_ms.unwrap_or(8_000).clamp(1_000, 30_000));
    let target = normalize_base_url(request.base_url.as_deref());
    let client = reqwest::Client::builder()
        .timeout(timeout_duration)
        .user_agent(NETWORK_USER_AGENT)
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;

    let mut checks = Vec::new();
    let mut findings = Vec::new();

    let exit_ip = check_exit_ip(&client, timeout_duration).await;
    checks.push(exit_ip.check.clone());
    if let Some(finding) = exit_ip.finding {
        findings.push(finding);
    }

    let Some(base_url) = target.clone() else {
        let check = skipped_check("network_target", "服务器地址", "未填写要检测的 API 地址。");
        checks.push(check.clone());
        findings.push(Finding {
            id: "network_base_url_missing".to_string(),
            title: "API 地址缺失".to_string(),
            severity: Severity::Medium,
            message: "网络检测没有目标服务器地址。".to_string(),
            next_step: "请填写 msutools API 地址后重新检测。".to_string(),
            fix_suggestion: Some("请填写 https://www.msutools.cn 或你的专属 API 地址。".to_string()),
        });
        let result = NetworkScanResult {
            target,
            host: None,
            scanned_at,
            status: ScanOverallStatus::NotReady,
            exit_ip: exit_ip.info,
            server_ips: Vec::new(),
            probes: Vec::new(),
            checks,
            findings,
            diagnostic_text: String::new(),
        };
        return Ok(with_network_diagnostic_text(result));
    };

    let parsed = Url::parse(&base_url)?;
    let host = parsed.host_str().map(ToString::to_string);
    let port = parsed.port_or_known_default().unwrap_or(443);

    let dns_result = resolve_server_ips(&parsed, port, timeout_duration).await;
    checks.push(dns_result.check.clone());
    if let Some(finding) = dns_result.finding {
        findings.push(finding);
    }

    let mut server_ips = Vec::new();
    for address in dns_result.addresses.iter().take(8) {
        server_ips.push(check_server_ip(&client, *address, timeout_duration).await);
    }
    for server in &server_ips {
        if matches!(server.status, CheckStatus::Fail) {
            findings.push(network_finding(
                &format!("server_ip_{}_failed", sanitize_id(&server.ip)),
                "服务器端口连接失败",
                Severity::High,
                &format!("{} {}", server.address, server.message),
                "请切换网络、关闭异常代理/VPN 后重新检测；如果仍失败，请把服务器 IP 和错误截图给客服。",
            ));
        }
    }

    let mut probes = Vec::new();
    probes.push(probe_http_endpoint(
        &client,
        "site",
        "网站首页",
        "GET",
        &base_url,
        None,
        timeout_duration,
    )
    .await);

    let health_url = format!("{}/health", base_url.trim_end_matches('/'));
    probes.push(probe_http_endpoint(
        &client,
        "health",
        "健康接口",
        "GET",
        &health_url,
        None,
        timeout_duration,
    )
    .await);

    let models_url = format!("{}/v1/models", base_url.trim_end_matches('/'));
    probes.push(probe_http_endpoint(
        &client,
        "models",
        "模型接口",
        "GET",
        &models_url,
        request.api_key.as_deref(),
        timeout_duration,
    )
    .await);

    for probe in &probes {
        checks.push(probe_to_check(probe));
        if !matches!(probe.status, CheckStatus::Pass) {
            findings.push(probe_to_finding(probe));
        }
    }

    let status = network_overall_status(&checks, &server_ips, &probes);
    let result = NetworkScanResult {
        target: Some(base_url),
        host,
        scanned_at,
        status,
        exit_ip: exit_ip.info,
        server_ips,
        probes,
        checks,
        findings,
        diagnostic_text: String::new(),
    };

    Ok(with_network_diagnostic_text(result))
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

struct ExitIpScan {
    info: Option<NetworkIpInfo>,
    check: ScanCheck,
    finding: Option<Finding>,
}

struct DnsScan {
    addresses: Vec<SocketAddr>,
    check: ScanCheck,
    finding: Option<Finding>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IpApiResponse {
    query: Option<String>,
    status: Option<String>,
    country: Option<String>,
    region_name: Option<String>,
    city: Option<String>,
    isp: Option<String>,
    org: Option<String>,
    #[serde(rename = "as")]
    asn: Option<String>,
    timezone: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IpifyResponse {
    ip: String,
}

async fn check_exit_ip(client: &reqwest::Client, limit: Duration) -> ExitIpScan {
    let started = Instant::now();
    let detail_url = "http://ip-api.com/json/?fields=status,message,country,regionName,city,isp,org,as,query,timezone";
    let detail = timeout(limit, client.get(detail_url).send()).await;

    match detail {
        Ok(Ok(response)) => {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            match serde_json::from_str::<IpApiResponse>(&text) {
                Ok(payload) if payload.status.as_deref() == Some("success") => {
                    let info = NetworkIpInfo {
                        ip: payload.query.unwrap_or_else(|| "unknown".to_string()),
                        country: payload.country,
                        region: payload.region_name,
                        city: payload.city,
                        isp: payload.isp,
                        org: payload.org,
                        asn: payload.asn,
                        timezone: payload.timezone,
                        source: "ip-api.com".to_string(),
                    };
                    let check = ScanCheck {
                        id: "exit_ip".to_string(),
                        title: "本地出口 IP".to_string(),
                        status: CheckStatus::Pass,
                        severity: Severity::Info,
                        message: format!("出口 IP 为 {}。", location_label(&info)),
                        evidence: json!({ "ip": info.ip, "source": info.source }),
                        duration_ms: started.elapsed().as_millis(),
                    };
                    ExitIpScan {
                        info: Some(info),
                        check,
                        finding: None,
                    }
                }
                Ok(payload) => {
                    fallback_exit_ip(client, limit, started, payload.message.unwrap_or_else(|| format!("ip-api 返回状态 {status}"))).await
                }
                Err(error) => {
                    fallback_exit_ip(client, limit, started, format!("出口 IP 归属地响应无法解析：{error}")).await
                }
            }
        }
        Ok(Err(error)) => {
            fallback_exit_ip(client, limit, started, format!("出口 IP 归属地查询失败：{error}")).await
        }
        Err(_) => fallback_exit_ip(client, limit, started, "出口 IP 归属地查询超时。".to_string()).await,
    }
}

async fn fallback_exit_ip(
    client: &reqwest::Client,
    limit: Duration,
    started: Instant,
    reason: String,
) -> ExitIpScan {
    let fallback = timeout(limit, client.get("https://api.ipify.org?format=json").send()).await;
    match fallback {
        Ok(Ok(response)) => {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            match serde_json::from_str::<IpifyResponse>(&text) {
                Ok(payload) => {
                    let info = NetworkIpInfo {
                        ip: payload.ip,
                        country: None,
                        region: None,
                        city: None,
                        isp: None,
                        org: None,
                        asn: None,
                        timezone: None,
                        source: "api.ipify.org".to_string(),
                    };
                    let check = ScanCheck {
                        id: "exit_ip".to_string(),
                        title: "本地出口 IP".to_string(),
                        status: CheckStatus::Warn,
                        severity: Severity::Medium,
                        message: "已获取出口 IP，但归属地查询不可用。".to_string(),
                        evidence: json!({ "ip": info.ip, "source": info.source, "locationError": reason }),
                        duration_ms: started.elapsed().as_millis(),
                    };
                    ExitIpScan {
                        info: Some(info),
                        check,
                        finding: Some(Finding {
                            id: "exit_ip_location_unavailable".to_string(),
                            title: "出口 IP 归属地查询不可用".to_string(),
                            severity: Severity::Medium,
                            message: reason,
                            next_step: "请把出口 IP 截图给客服，或检查本机代理/VPN 是否阻断了 IP 查询服务。".to_string(),
                            fix_suggestion: Some("如果正在使用代理/VPN，请临时关闭后重新检测；也可以把出口 IP 提供给客服排查。".to_string()),
                        }),
                    }
                }
                Err(error) => failed_exit_ip(started, format!("出口 IP 备用响应无法解析：{error}; HTTP {status}")),
            }
        }
        Ok(Err(error)) => failed_exit_ip(started, format!("出口 IP 查询失败：{error}")),
        Err(_) => failed_exit_ip(started, "出口 IP 查询超时。".to_string()),
    }
}

fn failed_exit_ip(started: Instant, message: String) -> ExitIpScan {
    let check = ScanCheck {
        id: "exit_ip".to_string(),
        title: "本地出口 IP".to_string(),
        status: CheckStatus::Fail,
        severity: Severity::High,
        message: "无法获取本地出口 IP。".to_string(),
        evidence: json!({ "error": message }),
        duration_ms: started.elapsed().as_millis(),
    };
    ExitIpScan {
        info: None,
        check,
        finding: Some(Finding {
            id: "exit_ip_failed".to_string(),
            title: "无法获取出口 IP".to_string(),
            severity: Severity::High,
            message,
            next_step: "请检查系统网络、DNS、代理或 VPN，确认浏览器是否能访问公网。".to_string(),
            fix_suggestion: Some("先关闭异常代理/VPN，确认系统时间正确，再重新检测。".to_string()),
        }),
    }
}

async fn resolve_server_ips(url: &Url, port: u16, limit: Duration) -> DnsScan {
    let started = Instant::now();
    let Some(host) = url.host_str() else {
        let message = "API 地址里没有域名。".to_string();
        return DnsScan {
            addresses: Vec::new(),
            check: failed_check("server_dns", "服务器 IP", &message, json!({}), started),
            finding: Some(network_finding(
                "server_dns_missing_host",
                "服务器地址没有域名",
                Severity::High,
                &message,
                "请确认 API 地址格式正确，例如 https://www.msutools.cn。",
            )),
        };
    };

    let host = host.to_string();
    let result = timeout(limit, tokio::task::spawn_blocking(move || {
        (host.as_str(), port).to_socket_addrs()
    }))
    .await;

    match result {
        Ok(Ok(Ok(addrs))) => {
            let addresses = unique_socket_addrs(addrs.collect());
            if addresses.is_empty() {
                let message = "DNS 没有返回服务器 IP。".to_string();
                DnsScan {
                    addresses,
                    check: failed_check(
                        "server_dns",
                        "服务器 IP",
                        &message,
                        json!({ "host": url.host_str(), "port": port }),
                        started,
                    ),
                    finding: Some(network_finding(
                        "server_dns_empty",
                        "服务器 DNS 无结果",
                        Severity::High,
                        &message,
                        "请检查域名是否拼写错误，或让客服确认域名解析是否正常。",
                    )),
                }
            } else {
                let evidence: Vec<String> = addresses.iter().map(ToString::to_string).collect();
                DnsScan {
                    addresses,
                    check: pass_check(
                        "server_dns",
                        "服务器 IP",
                        "已解析到服务器 IP。",
                        json!({ "host": url.host_str(), "port": port, "addresses": evidence }),
                        started,
                    ),
                    finding: None,
                }
            }
        }
        Ok(Ok(Err(error))) => {
            let message = format!("服务器 DNS 解析失败：{error}");
            DnsScan {
                addresses: Vec::new(),
                check: failed_check("server_dns", "服务器 IP", &message, json!({ "host": url.host_str(), "port": port }), started),
                finding: Some(network_finding(
                    "server_dns_failed",
                    "服务器 DNS 解析失败",
                    Severity::High,
                    &message,
                    "请检查本机 DNS、代理/VPN，或把错误截图给客服确认域名解析。",
                )),
            }
        }
        Ok(Err(error)) => {
            let message = format!("服务器 DNS 解析任务失败：{error}");
            DnsScan {
                addresses: Vec::new(),
                check: failed_check("server_dns", "服务器 IP", &message, json!({ "host": url.host_str(), "port": port }), started),
                finding: Some(network_finding(
                    "server_dns_task_failed",
                    "服务器 DNS 解析失败",
                    Severity::High,
                    &message,
                    "请重新检测；如果持续出现，请把诊断详情发给客服。",
                )),
            }
        }
        Err(_) => {
            let message = "服务器 DNS 解析超时。".to_string();
            DnsScan {
                addresses: Vec::new(),
                check: failed_check("server_dns", "服务器 IP", &message, json!({ "host": url.host_str(), "port": port, "timeoutMs": limit.as_millis() }), started),
                finding: Some(network_finding(
                    "server_dns_timeout",
                    "服务器 DNS 解析超时",
                    Severity::High,
                    &message,
                    "请切换 DNS、关闭异常代理/VPN 后重新检测。",
                )),
            }
        }
    }
}

async fn check_server_ip(
    client: &reqwest::Client,
    address: SocketAddr,
    limit: Duration,
) -> NetworkServerIp {
    let started = Instant::now();
    let connect = timeout(limit, TcpStream::connect(address)).await;
    let (status, message) = match connect {
        Ok(Ok(_)) => (CheckStatus::Pass, "端口可以连通。".to_string()),
        Ok(Err(error)) => (CheckStatus::Fail, format!("端口连接失败：{error}")),
        Err(_) => (CheckStatus::Fail, "端口连接超时。".to_string()),
    };
    let location = lookup_ip_location(client, address.ip(), limit).await.ok();
    NetworkServerIp {
        ip: address.ip().to_string(),
        address: address.to_string(),
        port: address.port(),
        family: if address.is_ipv4() { "IPv4" } else { "IPv6" }.to_string(),
        location,
        status,
        message,
        duration_ms: started.elapsed().as_millis(),
    }
}

async fn lookup_ip_location(
    client: &reqwest::Client,
    ip: IpAddr,
    limit: Duration,
) -> anyhow::Result<NetworkIpInfo> {
    let url = format!(
        "http://ip-api.com/json/{ip}?fields=status,message,country,regionName,city,isp,org,as,query,timezone"
    );
    let response = timeout(limit, client.get(url).send()).await??;
    let payload = response.json::<IpApiResponse>().await?;
    if payload.status.as_deref() != Some("success") {
        anyhow::bail!(payload.message.unwrap_or_else(|| "IP location lookup failed".to_string()));
    }
    Ok(NetworkIpInfo {
        ip: payload.query.unwrap_or_else(|| ip.to_string()),
        country: payload.country,
        region: payload.region_name,
        city: payload.city,
        isp: payload.isp,
        org: payload.org,
        asn: payload.asn,
        timezone: payload.timezone,
        source: "ip-api.com".to_string(),
    })
}

async fn probe_http_endpoint(
    client: &reqwest::Client,
    id: &str,
    title: &str,
    method: &str,
    url: &str,
    api_key: Option<&str>,
    limit: Duration,
) -> NetworkHttpProbe {
    let started = Instant::now();
    let has_api_key = api_key.map(str::trim).filter(|value| !value.is_empty()).is_some();
    let mut request = client.get(url);
    if let Some(api_key) = api_key.map(str::trim).filter(|value| !value.is_empty()) {
        request = request.bearer_auth(api_key.trim_start_matches("Bearer ").trim());
    }

    let result = timeout(limit, request.send()).await;
    match result {
        Ok(Ok(response)) => {
            let status_code = response.status().as_u16();
            let final_url = response.url().to_string();
            let headers = collect_response_headers(response.headers());
            let body = response.text().await.unwrap_or_default();
            let preview = sanitize_body_preview(&body);
            let classification = classify_http_status(id, status_code, has_api_key);
            NetworkHttpProbe {
                id: id.to_string(),
                title: title.to_string(),
                method: method.to_string(),
                url: final_url,
                status: classification.0,
                severity: classification.1,
                status_code: Some(status_code),
                reason: Some(status_reason(status_code).to_string()),
                message: classification.2,
                detail: classification.3,
                suggestion: classification.4,
                duration_ms: started.elapsed().as_millis(),
                response_headers: headers,
                body_preview: if preview.is_empty() { None } else { Some(preview) },
                error: None,
            }
        }
        Ok(Err(error)) => {
            let message = format!("请求失败：{error}");
            NetworkHttpProbe {
                id: id.to_string(),
                title: title.to_string(),
                method: method.to_string(),
                url: url.to_string(),
                status: CheckStatus::Fail,
                severity: Severity::High,
                status_code: None,
                reason: None,
                message: "无法访问目标地址。".to_string(),
                detail: message.clone(),
                suggestion: "请检查系统代理、VPN、防火墙、DNS，或把错误详情发给客服。".to_string(),
                duration_ms: started.elapsed().as_millis(),
                response_headers: Vec::new(),
                body_preview: None,
                error: Some(message),
            }
        }
        Err(_) => NetworkHttpProbe {
            id: id.to_string(),
            title: title.to_string(),
            method: method.to_string(),
            url: url.to_string(),
            status: CheckStatus::Fail,
            severity: Severity::High,
            status_code: None,
            reason: None,
            message: "请求超时。".to_string(),
            detail: format!("请求超过 {} ms 未完成。", limit.as_millis()),
            suggestion: "请检查本机网络质量、代理/VPN 或服务端可用性，然后重新检测。".to_string(),
            duration_ms: started.elapsed().as_millis(),
            response_headers: Vec::new(),
            body_preview: None,
            error: Some("timeout".to_string()),
        },
    }
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

fn unique_socket_addrs(addresses: Vec<SocketAddr>) -> Vec<SocketAddr> {
    let mut by_key = BTreeMap::new();
    for address in addresses {
        by_key.entry(address.to_string()).or_insert(address);
    }
    by_key.into_values().collect()
}

fn collect_response_headers(headers: &reqwest::header::HeaderMap) -> Vec<NetworkHeader> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            let value = value.to_str().ok()?;
            let lower = name.as_str().to_ascii_lowercase();
            let useful = matches!(
                lower.as_str(),
                "server"
                    | "cf-ray"
                    | "cf-cache-status"
                    | "x-request-id"
                    | "x-ratelimit-limit"
                    | "x-ratelimit-remaining"
                    | "x-ratelimit-reset"
                    | "retry-after"
                    | "content-type"
                    | "date"
                    | "via"
                    | "www-authenticate"
            );
            if useful {
                Some(NetworkHeader {
                    name: name.as_str().to_string(),
                    value: redaction::redact_secret(value),
                })
            } else {
                None
            }
        })
        .take(16)
        .collect()
}

fn sanitize_body_preview(body: &str) -> String {
    let preview = body.chars().take(500).collect::<String>();
    let mut value = redaction::redact_secret(&preview);

    if value.len() <= 6 && preview.len() > 6 {
        value = preview;
    }

    for marker in ["sk-", "Bearer ", "refresh_token", "api_key", "access_token"] {
        if value.to_ascii_lowercase().contains(&marker.to_ascii_lowercase()) {
            return redaction::redact_secret(&value);
        }
    }

    value
}

fn classify_http_status(
    probe_id: &str,
    code: u16,
    has_api_key: bool,
) -> (CheckStatus, Severity, String, String, String) {
    match code {
        200..=299 => (
            CheckStatus::Pass,
            Severity::Info,
            "请求成功。".to_string(),
            format!("服务器返回 HTTP {code}。"),
            "网络链路可用。".to_string(),
        ),
        300..=399 => (
            CheckStatus::Pass,
            Severity::Info,
            "服务器返回跳转响应。".to_string(),
            format!("服务器返回 HTTP {code}，客户端已跟随有限次数跳转。"),
            "如果最终地址不是预期域名，请检查 API 地址是否填写正确。".to_string(),
        ),
        401 if probe_id == "models" && !has_api_key => (
            CheckStatus::Warn,
            Severity::Medium,
            "模型接口需要 API Key。".to_string(),
            "HTTP 401 表示 /v1/models 已到达服务端，但当前请求没有提供有效 API Key。".to_string(),
            "请填写 API Key 后重新检测；如果填入后仍是 401，请重新生成 Key 或联系客服确认账号状态。".to_string(),
        ),
        401 => {
            let suggestion = if has_api_key {
                "请检查 API Key 是否正确、是否过期、是否带有 Bearer 前缀重复问题；也可以重新登录后创建新 Key。"
            } else {
                "这是未提供 API Key 时的常见响应。请填写 API Key 后重新检测模型接口。"
            };
            (
                if probe_id == "models" { CheckStatus::Fail } else { CheckStatus::Warn },
                Severity::High,
                "服务器拒绝认证。".to_string(),
                "HTTP 401 表示目标服务需要有效身份凭据。".to_string(),
                suggestion.to_string(),
            )
        }
        403 => (
            CheckStatus::Fail,
            Severity::High,
            "服务器禁止访问。".to_string(),
            "HTTP 403 可能是账号权限、IP 风控、Cloudflare/WAF 拦截或服务端访问策略导致。".to_string(),
            "请确认账号和 Key 权限；如果浏览器也无法访问，请截图此详情给客服核查 IP 或风控状态。".to_string(),
        ),
        404 if probe_id == "site" || probe_id == "health" => (
            CheckStatus::Warn,
            Severity::Medium,
            "目标路径不存在。".to_string(),
            "HTTP 404 表示域名可访问，但当前路径没有对应页面；这不一定代表 OpenAI 兼容 API 不可用。".to_string(),
            "请重点查看 /v1/models 的检测结果；如果 API 地址包含错误路径，请改回站点根地址后重新检测。".to_string(),
        ),
        404 => (
            CheckStatus::Fail,
            Severity::High,
            "目标路径不存在。".to_string(),
            "HTTP 404 表示域名可访问，但当前 API 路径没有对应服务。".to_string(),
            "请检查 API 地址是否多写或少写路径，OpenAI 兼容接口通常应能访问 /v1/models。".to_string(),
        ),
        408 => (
            CheckStatus::Fail,
            Severity::High,
            "请求超时。".to_string(),
            "HTTP 408 表示服务器等待请求超时。".to_string(),
            "请切换网络或关闭异常代理/VPN 后重新检测。".to_string(),
        ),
        429 => (
            CheckStatus::Fail,
            Severity::High,
            "请求过于频繁或额度受限。".to_string(),
            "HTTP 429 表示触发限流、余额/额度不足或上游暂时拒绝更多请求。".to_string(),
            "请稍后重试，检查账户余额和限额；如果持续出现，请把 Retry-After、请求 ID 或 cf-ray 截图给客服。".to_string(),
        ),
        500 => (
            CheckStatus::Fail,
            Severity::High,
            "服务端内部错误。".to_string(),
            "HTTP 500 表示服务端处理请求时发生错误。".to_string(),
            "请截图诊断详情给客服，尤其是请求时间、URL、响应头和 body 预览。".to_string(),
        ),
        502 => (
            CheckStatus::Fail,
            Severity::High,
            "网关错误。".to_string(),
            "HTTP 502 通常表示网关无法从上游拿到有效响应。".to_string(),
            "请稍后重试；若持续出现，请把诊断详情发给客服确认服务端上游状态。".to_string(),
        ),
        503 => (
            CheckStatus::Fail,
            Severity::High,
            "服务暂不可用。".to_string(),
            "HTTP 503 表示服务暂时不可用，可能在维护、过载或上游不可用。".to_string(),
            "请稍后重试，并把诊断详情发给客服确认服务状态。".to_string(),
        ),
        520 => (
            CheckStatus::Fail,
            Severity::High,
            "Cloudflare 未知源站错误。".to_string(),
            "HTTP 520 表示 Cloudflare 从源站收到异常响应。".to_string(),
            "请截图 cf-ray、时间和目标地址给客服，通常需要服务端排查源站。".to_string(),
        ),
        521 => (
            CheckStatus::Fail,
            Severity::High,
            "Cloudflare 无法连接源站。".to_string(),
            "HTTP 521 表示 Cloudflare 到源站连接失败，用户本地通常无法自行修复。".to_string(),
            "请截图 cf-ray、时间和目标地址给客服；可先切换网络确认不是本地代理阻断。".to_string(),
        ),
        522 => (
            CheckStatus::Fail,
            Severity::High,
            "Cloudflare 连接源站超时。".to_string(),
            "HTTP 522 表示 Cloudflare 到源站连接超时。".to_string(),
            "请截图 cf-ray 和检测时间给客服，通常需要服务端确认源站连通性。".to_string(),
        ),
        523 => (
            CheckStatus::Fail,
            Severity::High,
            "Cloudflare 无法到达源站。".to_string(),
            "HTTP 523 表示源站路由不可达或 DNS 指向异常。".to_string(),
            "请把诊断详情发给客服确认域名解析和源站网络。".to_string(),
        ),
        524 => (
            CheckStatus::Fail,
            Severity::High,
            "Cloudflare 等待源站超时。".to_string(),
            "HTTP 524 表示源站已连接但响应超时。".to_string(),
            "请稍后重试；若持续出现，请截图给客服确认服务端负载。".to_string(),
        ),
        525 => (
            CheckStatus::Fail,
            Severity::High,
            "Cloudflare 与源站 TLS 握手失败。".to_string(),
            "HTTP 525 表示源站证书或 TLS 配置异常。".to_string(),
            "请截图 cf-ray 和目标地址给客服，通常需要服务端修复证书。".to_string(),
        ),
        526 => (
            CheckStatus::Fail,
            Severity::High,
            "Cloudflare 认为源站证书无效。".to_string(),
            "HTTP 526 表示源站证书不被 Cloudflare 信任。".to_string(),
            "请截图诊断详情给客服，通常需要服务端修复证书链。".to_string(),
        ),
        _ if code >= 500 => (
            CheckStatus::Fail,
            Severity::High,
            "服务器返回错误状态。".to_string(),
            format!("HTTP {code} 表示服务端或网关异常。"),
            "请截图状态码、响应头和 body 预览给客服排查。".to_string(),
        ),
        _ if code >= 400 => (
            CheckStatus::Warn,
            Severity::Medium,
            "服务器返回客户端错误状态。".to_string(),
            format!("HTTP {code} 表示请求被目标服务拒绝或路径不符合预期。"),
            "请检查 API 地址和 API Key；如不确定，请截图诊断详情给客服。".to_string(),
        ),
        _ => (
            CheckStatus::Warn,
            Severity::Medium,
            "服务器返回非标准状态。".to_string(),
            format!("服务器返回 HTTP {code}。"),
            "请截图诊断详情给客服确认。".to_string(),
        ),
    }
}

fn status_reason(code: u16) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        408 => "Request Timeout",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        520 => "Cloudflare Unknown Error",
        521 => "Cloudflare Web Server Is Down",
        522 => "Cloudflare Connection Timed Out",
        523 => "Cloudflare Origin Is Unreachable",
        524 => "Cloudflare A Timeout Occurred",
        525 => "Cloudflare SSL Handshake Failed",
        526 => "Cloudflare Invalid SSL Certificate",
        _ => "HTTP Status",
    }
}

fn probe_to_check(probe: &NetworkHttpProbe) -> ScanCheck {
    ScanCheck {
        id: format!("network_{}", probe.id),
        title: probe.title.clone(),
        status: probe.status.clone(),
        severity: probe.severity.clone(),
        message: match probe.status_code {
            Some(code) => format!("{} HTTP {}。", probe.message, code),
            None => probe.message.clone(),
        },
        evidence: json!({
            "method": probe.method,
            "url": probe.url,
            "statusCode": probe.status_code,
            "reason": probe.reason,
            "detail": probe.detail,
            "suggestion": probe.suggestion,
            "headers": probe.response_headers,
            "bodyPreview": probe.body_preview,
            "error": probe.error,
        }),
        duration_ms: probe.duration_ms,
    }
}

fn probe_to_finding(probe: &NetworkHttpProbe) -> Finding {
    Finding {
        id: format!("network_{}_{}", probe.id, serde_status(&probe.status)),
        title: probe.title.clone(),
        severity: probe.severity.clone(),
        message: probe.detail.clone(),
        next_step: probe.suggestion.clone(),
        fix_suggestion: Some(probe.suggestion.clone()),
    }
}

fn network_finding(
    id: &str,
    title: &str,
    severity: Severity,
    message: &str,
    next_step: &str,
) -> Finding {
    Finding {
        id: id.to_string(),
        title: title.to_string(),
        severity,
        message: message.to_string(),
        next_step: next_step.to_string(),
        fix_suggestion: Some(next_step.to_string()),
    }
}

fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn network_overall_status(
    checks: &[ScanCheck],
    server_ips: &[NetworkServerIp],
    probes: &[NetworkHttpProbe],
) -> ScanOverallStatus {
    if checks.iter().any(|check| matches!(check.status, CheckStatus::Fail))
        || server_ips.iter().any(|item| matches!(item.status, CheckStatus::Fail))
        || probes.iter().any(|probe| matches!(probe.status, CheckStatus::Fail))
    {
        ScanOverallStatus::Failed
    } else if checks
        .iter()
        .any(|check| matches!(check.status, CheckStatus::Warn | CheckStatus::Skipped))
        || probes
            .iter()
            .any(|probe| matches!(probe.status, CheckStatus::Warn | CheckStatus::Skipped))
    {
        ScanOverallStatus::NeedsAttention
    } else {
        ScanOverallStatus::Passed
    }
}

fn with_network_diagnostic_text(mut result: NetworkScanResult) -> NetworkScanResult {
    result.diagnostic_text = build_network_diagnostic_text(&result);
    result
}

fn build_network_diagnostic_text(result: &NetworkScanResult) -> String {
    let mut lines = Vec::new();
    lines.push("AI-SCAN 网络检测报告".to_string());
    lines.push(format!("检测时间: {}", result.scanned_at));
    lines.push(format!("目标地址: {}", result.target.as_deref().unwrap_or("未填写")));
    lines.push(format!("目标域名: {}", result.host.as_deref().unwrap_or("未知")));
    lines.push(format!("总体状态: {}", overall_status_text(&result.status)));
    lines.push(String::new());

    lines.push("本地出口 IP:".to_string());
    if let Some(ip) = &result.exit_ip {
        lines.push(format!("  IP: {}", ip.ip));
        lines.push(format!("  归属地: {}", location_label(ip)));
        lines.push(format!("  运营商: {}", ip.isp.as_deref().unwrap_or("未知")));
        lines.push(format!("  ASN: {}", ip.asn.as_deref().unwrap_or("未知")));
    } else {
        lines.push("  未获取".to_string());
    }
    lines.push(String::new());

    lines.push("服务器 IP:".to_string());
    if result.server_ips.is_empty() {
        lines.push("  未解析到服务器 IP".to_string());
    } else {
        for item in &result.server_ips {
            let location = item
                .location
                .as_ref()
                .map(location_label)
                .unwrap_or_else(|| "归属地未知".to_string());
            lines.push(format!(
                "  {} [{}] {} - {} ({})",
                item.address,
                item.family,
                location,
                check_status_text(&item.status),
                item.message
            ));
        }
    }
    lines.push(String::new());

    lines.push("HTTP 探测:".to_string());
    for probe in &result.probes {
        lines.push(format!(
            "  [{}] {} {} -> {} {}",
            check_status_text(&probe.status),
            probe.method,
            probe.url,
            probe
                .status_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "无状态码".to_string()),
            probe.reason.as_deref().unwrap_or("")
        ));
        lines.push(format!("    说明: {}", probe.detail));
        lines.push(format!("    建议: {}", probe.suggestion));
        if !probe.response_headers.is_empty() {
            let headers = probe
                .response_headers
                .iter()
                .map(|item| format!("{}={}", item.name, item.value))
                .collect::<Vec<_>>()
                .join("; ");
            lines.push(format!("    响应头: {headers}"));
        }
        if let Some(body) = &probe.body_preview {
            lines.push(format!("    Body预览: {}", body.replace('\n', "\\n")));
        }
        if let Some(error) = &probe.error {
            lines.push(format!("    错误: {error}"));
        }
    }
    lines.push(String::new());

    lines.push("问题与建议:".to_string());
    if result.findings.is_empty() {
        lines.push("  未发现需要处理的问题。".to_string());
    } else {
        for finding in &result.findings {
            lines.push(format!("  - {}: {}", finding.title, finding.message));
            lines.push(format!("    下一步: {}", finding.next_step));
        }
    }

    lines.join("\n")
}

fn location_label(info: &NetworkIpInfo) -> String {
    let mut parts = Vec::new();
    if let Some(country) = &info.country {
        parts.push(country.as_str());
    }
    if let Some(region) = &info.region {
        parts.push(region.as_str());
    }
    if let Some(city) = &info.city {
        parts.push(city.as_str());
    }
    if parts.is_empty() {
        info.ip.clone()
    } else {
        format!("{} ({})", info.ip, parts.join(" / "))
    }
}

fn check_status_text(status: &CheckStatus) -> &'static str {
    match status {
        CheckStatus::Pass => "通过",
        CheckStatus::Warn => "需关注",
        CheckStatus::Fail => "未通过",
        CheckStatus::Skipped => "已跳过",
    }
}

fn overall_status_text(status: &ScanOverallStatus) -> &'static str {
    match status {
        ScanOverallStatus::Passed => "通过",
        ScanOverallStatus::NeedsAttention => "需关注",
        ScanOverallStatus::Failed => "未通过",
        ScanOverallStatus::NotReady => "未就绪",
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
