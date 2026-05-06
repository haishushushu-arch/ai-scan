use std::env;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use anyhow::Context;
use chrono::Utc;
use serde_json::json;
use sysinfo::System;
use tokio::process::Command;
use tokio::time::timeout;

use crate::core::models::{
    CheckStatus, EnvironmentVariable, Finding, ScanCheck, ScanOverallStatus, Severity,
    SystemEnvironmentScanResult, SystemProfile, ToolVersion,
};
use crate::core::redaction;

pub async fn system_profile() -> anyhow::Result<SystemProfile> {
    let mut system = System::new_all();
    system.refresh_all();

    let tools = detect_tools().await;
    let environment = collect_environment();
    let path_entries = env::var_os("PATH")
        .map(|path| env::split_paths(&path).map(path_to_string).collect())
        .unwrap_or_default();

    Ok(SystemProfile {
        os: System::name().unwrap_or_else(|| env::consts::OS.to_string()),
        os_version: System::os_version(),
        kernel_version: System::kernel_version(),
        architecture: env::consts::ARCH.to_string(),
        hostname: hostname::get().ok().map(|value| value.to_string_lossy().to_string()),
        username: env::var("USERNAME")
            .or_else(|_| env::var("USER"))
            .ok()
            .filter(|value| !value.is_empty()),
        cpu_brand: system.cpus().first().map(|cpu| cpu.brand().to_string()),
        cpu_cores: system.cpus().len(),
        total_memory_bytes: system.total_memory().saturating_mul(1024),
        used_memory_bytes: system.used_memory().saturating_mul(1024),
        shell: env::var("SHELL")
            .or_else(|_| env::var("ComSpec"))
            .ok()
            .filter(|value| !value.is_empty()),
        path_entries,
        environment,
        tools,
        generated_at: Utc::now().to_rfc3339(),
    })
}

pub async fn system_environment_scan() -> anyhow::Result<SystemEnvironmentScanResult> {
    let profile = system_profile().await?;
    let mut checks = Vec::new();

    checks.push(check_os_profile(&profile));
    checks.push(check_memory(&profile));
    checks.push(check_shell(&profile));
    checks.push(check_path(&profile));
    checks.push(check_proxy_environment(&profile));
    checks.push(check_ai_environment(&profile));
    checks.push(check_tooling(&profile));

    let findings = system_findings_from_checks(&checks);
    let status = system_overall_status(&checks);
    Ok(SystemEnvironmentScanResult {
        scanned_at: Utc::now().to_rfc3339(),
        status,
        checks,
        findings,
        profile,
    })
}

fn collect_environment() -> Vec<EnvironmentVariable> {
    let important = [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "NO_PROXY",
        "ALL_PROXY",
        "OPENAI_API_KEY",
        "OPENAI_BASE_URL",
        "OPENAI_API_BASE",
        "PATH",
        "NODE_OPTIONS",
        "NPM_CONFIG_REGISTRY",
        "GIT_SSL_NO_VERIFY",
        "DOCKER_HOST",
    ];

    important
        .iter()
        .filter_map(|name| {
            env::var(name).ok().map(|value| {
                let redacted = redaction::is_sensitive_name(name);
                EnvironmentVariable {
                    name: (*name).to_string(),
                    value: redaction::redact_named_value(name, &value),
                    redacted,
                }
            })
        })
        .collect()
}

async fn detect_tools() -> Vec<ToolVersion> {
    let candidates = [
        ("node", "node", &["--version"][..]),
        ("npm", "npm", &["--version"][..]),
        ("git", "git", &["--version"][..]),
        ("curl", "curl", &["--version"][..]),
        ("docker", "docker", &["--version"][..]),
    ];

    let mut output = Vec::with_capacity(candidates.len());
    for (name, executable, args) in candidates {
        output.push(detect_tool(name, executable, args).await);
    }
    output
}

async fn detect_tool(name: &str, executable: &str, args: &[&str]) -> ToolVersion {
    match run_version_command(executable, args).await {
        Ok(version) => ToolVersion {
            name: name.to_string(),
            executable: executable.to_string(),
            available: true,
            version: Some(first_line(&version)),
            error: None,
        },
        Err(error) => ToolVersion {
            name: name.to_string(),
            executable: executable.to_string(),
            available: false,
            version: None,
            error: Some(error.to_string()),
        },
    }
}

async fn run_version_command(executable: &str, args: &[&str]) -> anyhow::Result<String> {
    let child = Command::new(executable)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to start {executable}"))?;

    let output = timeout(Duration::from_secs(4), child.wait_with_output())
        .await
        .with_context(|| format!("{executable} version check timed out"))?
        .with_context(|| format!("failed to read {executable} output"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(if stdout.is_empty() { stderr } else { stdout })
    } else {
        anyhow::bail!(
            "{} exited with {}: {}",
            executable,
            output.status,
            if stderr.is_empty() { stdout } else { stderr }
        )
    }
}

fn first_line(value: &str) -> String {
    value.lines().next().unwrap_or(value).trim().to_string()
}

fn path_to_string(path: PathBuf) -> String {
    path.to_string_lossy().to_string()
}

fn check_os_profile(profile: &SystemProfile) -> ScanCheck {
    let missing = [
        ("osVersion", profile.os_version.as_deref()),
        ("hostname", profile.hostname.as_deref()),
        ("username", profile.username.as_deref()),
    ]
    .into_iter()
    .filter_map(|(name, value)| value.filter(|text| !text.trim().is_empty()).map(|_| ()).is_none().then_some(name))
    .collect::<Vec<_>>();

    if missing.is_empty() {
        pass_check(
            "system_profile",
            "系统信息",
            "已读取操作系统、架构、主机名和当前用户。",
            json!({
                "os": profile.os,
                "osVersion": profile.os_version,
                "architecture": profile.architecture,
                "hostname": profile.hostname,
                "username": profile.username,
            }),
        )
    } else {
        warn_check(
            "system_profile",
            "系统信息",
            "系统信息已读取，但部分字段不可用。",
            json!({
                "os": profile.os,
                "architecture": profile.architecture,
                "missing": missing,
            }),
        )
    }
}

fn check_memory(profile: &SystemProfile) -> ScanCheck {
    let total_gib = profile.total_memory_bytes as f64 / 1024_f64.powi(3);
    let used_gib = profile.used_memory_bytes as f64 / 1024_f64.powi(3);
    let usage = if profile.total_memory_bytes == 0 {
        0.0
    } else {
        profile.used_memory_bytes as f64 / profile.total_memory_bytes as f64
    };

    if profile.total_memory_bytes == 0 {
        warn_check(
            "memory",
            "内存信息",
            "未能读取系统内存容量。",
            json!({ "totalBytes": profile.total_memory_bytes, "usedBytes": profile.used_memory_bytes }),
        )
    } else if usage >= 0.92 {
        warn_check(
            "memory",
            "内存信息",
            "当前内存占用偏高，运行多个 AI 客户端时可能卡顿。",
            json!({ "totalGiB": total_gib, "usedGiB": used_gib, "usageRatio": usage }),
        )
    } else {
        pass_check(
            "memory",
            "内存信息",
            "系统内存信息正常。",
            json!({ "totalGiB": total_gib, "usedGiB": used_gib, "usageRatio": usage }),
        )
    }
}

fn check_shell(profile: &SystemProfile) -> ScanCheck {
    match profile.shell.as_deref().filter(|value| !value.trim().is_empty()) {
        Some(shell) => pass_check(
            "shell",
            "默认 Shell",
            "已识别当前 Shell。",
            json!({ "shell": shell }),
        ),
        None => warn_check(
            "shell",
            "默认 Shell",
            "未识别当前 Shell，部分客户端配置可能需要手动确认。",
            json!({}),
        ),
    }
}

fn check_path(profile: &SystemProfile) -> ScanCheck {
    let empty_entries = profile.path_entries.iter().filter(|entry| entry.trim().is_empty()).count();
    let duplicate_count = duplicate_count(&profile.path_entries);
    let existing_count = profile
        .path_entries
        .iter()
        .filter(|entry| !entry.trim().is_empty() && std::path::Path::new(entry).exists())
        .count();

    if profile.path_entries.is_empty() {
        failed_check(
            "path",
            "PATH 路径",
            "PATH 为空，命令行客户端通常无法正常找到运行环境。",
            json!({ "entries": 0 }),
        )
    } else if empty_entries > 0 || duplicate_count > 0 {
        warn_check(
            "path",
            "PATH 路径",
            "PATH 中存在空项或重复项，建议清理后再配置 AI 客户端。",
            json!({
                "entries": profile.path_entries.len(),
                "existingEntries": existing_count,
                "emptyEntries": empty_entries,
                "duplicateEntries": duplicate_count,
            }),
        )
    } else {
        pass_check(
            "path",
            "PATH 路径",
            "PATH 已读取，未发现明显空项或重复项。",
            json!({
                "entries": profile.path_entries.len(),
                "existingEntries": existing_count,
            }),
        )
    }
}

fn check_proxy_environment(profile: &SystemProfile) -> ScanCheck {
    let proxy_names = ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "NO_PROXY"];
    let proxies = profile
        .environment
        .iter()
        .filter(|item| proxy_names.iter().any(|name| item.name.eq_ignore_ascii_case(name)))
        .collect::<Vec<_>>();

    if proxies.is_empty() {
        warn_check(
            "proxy_environment",
            "代理环境变量",
            "未发现常用代理环境变量。如果当前网络必须走代理，请在客户端内确认代理配置。",
            json!({ "detected": [] }),
        )
    } else {
        pass_check(
            "proxy_environment",
            "代理环境变量",
            "已读取常用代理环境变量。",
            json!({
                "detected": proxies.iter().map(|item| json!({
                    "name": item.name,
                    "value": item.value,
                    "redacted": item.redacted,
                })).collect::<Vec<_>>()
            }),
        )
    }
}

fn check_ai_environment(profile: &SystemProfile) -> ScanCheck {
    let names = ["OPENAI_BASE_URL", "OPENAI_API_BASE", "OPENAI_API_KEY"];
    let values = profile
        .environment
        .iter()
        .filter(|item| names.iter().any(|name| item.name.eq_ignore_ascii_case(name)))
        .collect::<Vec<_>>();

    let has_base_url = values
        .iter()
        .any(|item| item.name.eq_ignore_ascii_case("OPENAI_BASE_URL") || item.name.eq_ignore_ascii_case("OPENAI_API_BASE"));

    if values.is_empty() {
        warn_check(
            "ai_environment",
            "AI 环境变量",
            "未发现 OpenAI 兼容环境变量。命令行客户端可能需要单独配置 API 地址和 Key。",
            json!({ "detected": [] }),
        )
    } else if !has_base_url {
        warn_check(
            "ai_environment",
            "AI 环境变量",
            "已发现 API Key，但未发现 OpenAI 兼容 Base URL。",
            json!({
                "detected": values.iter().map(|item| json!({
                    "name": item.name,
                    "value": item.value,
                    "redacted": item.redacted,
                })).collect::<Vec<_>>()
            }),
        )
    } else {
        pass_check(
            "ai_environment",
            "AI 环境变量",
            "已发现 OpenAI 兼容环境变量。",
            json!({
                "detected": values.iter().map(|item| json!({
                    "name": item.name,
                    "value": item.value,
                    "redacted": item.redacted,
                })).collect::<Vec<_>>()
            }),
        )
    }
}

fn check_tooling(profile: &SystemProfile) -> ScanCheck {
    let required = ["node", "npm", "git", "curl"];
    let missing = required
        .iter()
        .filter(|name| {
            !profile
                .tools
                .iter()
                .any(|tool| tool.name.eq_ignore_ascii_case(name) && tool.available)
        })
        .copied()
        .collect::<Vec<_>>();

    if missing.is_empty() {
        pass_check(
            "tooling",
            "常用命令",
            "Node.js、npm、Git 和 curl 均可用。",
            json!({ "tools": profile.tools }),
        )
    } else {
        warn_check(
            "tooling",
            "常用命令",
            "部分常用命令不可用，相关 AI 客户端或安装器可能受影响。",
            json!({ "missing": missing, "tools": profile.tools }),
        )
    }
}

fn duplicate_count(values: &[String]) -> usize {
    let mut seen = std::collections::HashSet::new();
    let mut duplicates = 0;
    for value in values {
        let normalized = value.trim().to_lowercase();
        if normalized.is_empty() {
            continue;
        }
        if !seen.insert(normalized) {
            duplicates += 1;
        }
    }
    duplicates
}

fn system_findings_from_checks(checks: &[ScanCheck]) -> Vec<Finding> {
    checks
        .iter()
        .filter(|check| matches!(check.status, CheckStatus::Fail | CheckStatus::Warn))
        .map(|check| Finding {
            id: format!("{}_{}", check.id, status_suffix(&check.status)),
            title: check.title.clone(),
            severity: check.severity.clone(),
            message: check.message.clone(),
            next_step: system_suggestion_for(&check.id),
            fix_suggestion: Some(system_suggestion_for(&check.id)),
        })
        .collect()
}

fn system_overall_status(checks: &[ScanCheck]) -> ScanOverallStatus {
    if checks.iter().any(|check| matches!(check.status, CheckStatus::Fail)) {
        ScanOverallStatus::Failed
    } else if checks.iter().any(|check| matches!(check.status, CheckStatus::Warn | CheckStatus::Skipped)) {
        ScanOverallStatus::NeedsAttention
    } else {
        ScanOverallStatus::Passed
    }
}

fn system_suggestion_for(id: &str) -> String {
    match id {
        "system_profile" => "系统信息不完整时，优先确认当前用户权限和系统 API 是否可访问。".to_string(),
        "memory" => "关闭占用较高的程序，或稍后在负载较低时重新检测。".to_string(),
        "shell" => "确认 PowerShell、cmd、bash 或 zsh 是否可用，并检查终端启动配置。".to_string(),
        "path" => "清理 PATH 中的空项、重复项和失效目录，再重新打开客户端。".to_string(),
        "proxy_environment" => "如果需要代理，请确认 HTTP_PROXY、HTTPS_PROXY 或客户端内代理设置。".to_string(),
        "ai_environment" => "为命令行客户端配置 OPENAI_BASE_URL 和对应 API Key。".to_string(),
        "tooling" => "安装或修复 Node.js、npm、Git、curl 后重新检测。".to_string(),
        _ => "请打开专业模式查看详细证据。".to_string(),
    }
}

fn status_suffix(status: &CheckStatus) -> &'static str {
    match status {
        CheckStatus::Pass => "pass",
        CheckStatus::Warn => "warn",
        CheckStatus::Fail => "fail",
        CheckStatus::Skipped => "skipped",
    }
}

fn pass_check(id: &str, title: &str, message: &str, evidence: serde_json::Value) -> ScanCheck {
    ScanCheck {
        id: id.to_string(),
        title: title.to_string(),
        status: CheckStatus::Pass,
        severity: Severity::Info,
        message: message.to_string(),
        evidence,
        duration_ms: 0,
    }
}

fn warn_check(id: &str, title: &str, message: &str, evidence: serde_json::Value) -> ScanCheck {
    ScanCheck {
        id: id.to_string(),
        title: title.to_string(),
        status: CheckStatus::Warn,
        severity: Severity::Medium,
        message: message.to_string(),
        evidence,
        duration_ms: 0,
    }
}

fn failed_check(id: &str, title: &str, message: &str, evidence: serde_json::Value) -> ScanCheck {
    ScanCheck {
        id: id.to_string(),
        title: title.to_string(),
        status: CheckStatus::Fail,
        severity: Severity::High,
        message: message.to_string(),
        evidence,
        duration_ms: 0,
    }
}
