use std::env;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use anyhow::Context;
use chrono::Utc;
use sysinfo::System;
use tokio::process::Command;
use tokio::time::timeout;

use crate::core::models::{EnvironmentVariable, SystemProfile, ToolVersion};
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
