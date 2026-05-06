use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemProfile {
    pub os: String,
    pub os_version: Option<String>,
    pub kernel_version: Option<String>,
    pub architecture: String,
    pub hostname: Option<String>,
    pub username: Option<String>,
    pub cpu_brand: Option<String>,
    pub cpu_cores: usize,
    pub total_memory_bytes: u64,
    pub used_memory_bytes: u64,
    pub shell: Option<String>,
    pub path_entries: Vec<String>,
    pub environment: Vec<EnvironmentVariable>,
    pub tools: Vec<ToolVersion>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentVariable {
    pub name: String,
    pub value: String,
    pub redacted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolVersion {
    pub name: String,
    pub executable: String,
    pub available: bool,
    pub version: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickScanRequest {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub timeout_ms: Option<u64>,
}

impl Default for QuickScanRequest {
    fn default() -> Self {
        Self {
            base_url: Some("https://www.msutools.cn".to_string()),
            api_key: None,
            timeout_ms: Some(8_000),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickScanResult {
    pub target: Option<String>,
    pub started_at: String,
    pub finished_at: String,
    pub status: ScanOverallStatus,
    pub scanned_at: String,
    pub checks: Vec<ScanCheck>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkScanRequest {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub timeout_ms: Option<u64>,
}

impl Default for NetworkScanRequest {
    fn default() -> Self {
        Self {
            base_url: Some("https://www.msutools.cn".to_string()),
            api_key: None,
            timeout_ms: Some(8_000),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkScanResult {
    pub target: Option<String>,
    pub host: Option<String>,
    pub scanned_at: String,
    pub status: ScanOverallStatus,
    pub exit_ip: Option<NetworkIpInfo>,
    pub server_ips: Vec<NetworkServerIp>,
    pub probes: Vec<NetworkHttpProbe>,
    pub checks: Vec<ScanCheck>,
    pub findings: Vec<Finding>,
    pub diagnostic_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkIpInfo {
    pub ip: String,
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub isp: Option<String>,
    pub org: Option<String>,
    pub asn: Option<String>,
    pub timezone: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkServerIp {
    pub ip: String,
    pub address: String,
    pub port: u16,
    pub family: String,
    pub location: Option<NetworkIpInfo>,
    pub status: CheckStatus,
    pub message: String,
    pub duration_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkHttpProbe {
    pub id: String,
    pub title: String,
    pub method: String,
    pub url: String,
    pub status: CheckStatus,
    pub severity: Severity,
    pub status_code: Option<u16>,
    pub reason: Option<String>,
    pub message: String,
    pub detail: String,
    pub suggestion: String,
    pub duration_ms: u128,
    pub response_headers: Vec<NetworkHeader>,
    pub body_preview: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgressEvent {
    pub run_id: String,
    pub phase: ScanProgressPhase,
    pub progress: u8,
    pub completed: usize,
    pub total: usize,
    pub current_step_id: Option<String>,
    pub current_step_title: Option<String>,
    pub message: String,
    pub check: Option<ScanCheck>,
    pub emitted_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanProgressPhase {
    Started,
    StepStarted,
    StepFinished,
    Finished,
    Canceled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanCheck {
    pub id: String,
    pub title: String,
    pub status: CheckStatus,
    pub severity: Severity,
    pub message: String,
    pub evidence: serde_json::Value,
    pub duration_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub id: String,
    pub title: String,
    pub severity: Severity,
    pub message: String,
    pub next_step: String,
    pub fix_suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanOverallStatus {
    Passed,
    NeedsAttention,
    Failed,
    NotReady,
}

impl From<&Severity> for String {
    fn from(value: &Severity) -> Self {
        match value {
            Severity::Info => "info",
            Severity::Warning | Severity::Low | Severity::Medium => "warning",
            Severity::Error | Severity::High => "error",
        }
        .to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountStatus {
    pub login_state: AccountLoginState,
    pub display_name: Option<String>,
    pub email_masked: Option<String>,
    pub balance_text: Option<String>,
    pub quota_text: Option<String>,
    pub authenticated: bool,
    pub state: AccountState,
    pub user: Option<AccountUser>,
    pub balance: Option<AccountBalance>,
    pub message: String,
}

impl AccountStatus {
    pub fn unauthenticated(message: impl Into<String>) -> Self {
        Self {
            login_state: AccountLoginState::LoggedOut,
            display_name: None,
            email_masked: None,
            balance_text: None,
            quota_text: None,
            authenticated: false,
            state: AccountState::Unauthenticated,
            user: None,
            balance: None,
            message: message.into(),
        }
    }

    pub fn authenticated(user: AccountUser, balance: AccountBalance, quota_text: Option<String>) -> Self {
        let display_name = user.display_name.clone();
        let email_masked = user.email_masked.clone();
        let balance_text = balance.amount.as_ref().map(|amount| {
            let unit = balance.unit.clone().unwrap_or_else(|| "USD".to_string());
            format!("{amount} {unit}")
        });
        Self {
            login_state: AccountLoginState::LoggedIn,
            display_name,
            email_masked,
            balance_text,
            quota_text,
            authenticated: true,
            state: AccountState::Authenticated,
            user: Some(user),
            balance: Some(balance),
            message: "Authenticated with msutools.".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountState {
    Authenticated,
    Unauthenticated,
    Unconfigured,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountLoginState {
    LoggedIn,
    LoggedOut,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountUser {
    pub id: Option<String>,
    pub display_name: Option<String>,
    pub email_masked: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountBalance {
    pub amount: Option<String>,
    pub unit: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyList {
    pub configured: bool,
    pub keys: Vec<ApiKeySummary>,
    pub message: String,
}

impl ApiKeyList {
    pub fn unconfigured(message: impl Into<String>) -> Self {
        Self {
            configured: false,
            keys: Vec::new(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeySummary {
    pub id: String,
    pub name: String,
    pub key_masked: String,
    pub masked_key: String,
    pub group_id: Option<i64>,
    pub created_at: Option<String>,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub quota_text: Option<String>,
    pub usage_text: Option<String>,
    pub enabled: Option<bool>,
    pub status: String,
}

impl ApiKeySummary {
    pub fn new(
        id: String,
        name: String,
        key_masked: String,
        status: String,
        created_at: Option<String>,
        last_used_at: Option<String>,
    ) -> Self {
        let enabled = match status.as_str() {
            "active" => Some(true),
            "inactive" | "disabled" | "expired" | "quota_exhausted" => Some(false),
            _ => None,
        };
        Self {
            id,
            name,
            masked_key: key_masked.clone(),
            key_masked,
            group_id: None,
            created_at,
            last_used_at,
            expires_at: None,
            quota_text: None,
            usage_text: None,
            enabled,
            status,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicSettings {
    pub site_name: Option<String>,
    pub site_subtitle: Option<String>,
    pub api_base_url: Option<String>,
    pub doc_url: Option<String>,
    pub contact_info: Option<String>,
    pub turnstile_enabled: Option<bool>,
    pub turnstile_site_key: Option<String>,
    pub payment_enabled: bool,
    pub purchase_subscription_enabled: Option<bool>,
    pub purchase_subscription_url: Option<String>,
    pub balance_low_notify_recharge_url: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub turnstile_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResult {
    pub status: LoginResultStatus,
    pub account: AccountStatus,
    pub token_expires_at: Option<String>,
    pub requires_2fa: bool,
    pub temp_token: Option<String>,
    pub user_email_masked: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoginResultStatus {
    Authenticated,
    #[serde(rename = "requires_2fa")]
    Requires2fa,
}

impl LoginResult {
    pub fn authenticated(account: AccountStatus, token_expires_at: Option<String>) -> Self {
        Self {
            status: LoginResultStatus::Authenticated,
            account,
            token_expires_at,
            requires_2fa: false,
            temp_token: None,
            user_email_masked: None,
        }
    }

    pub fn requires_2fa(temp_token: Option<String>, user_email_masked: Option<String>) -> Self {
        Self {
            status: LoginResultStatus::Requires2fa,
            account: AccountStatus::unauthenticated("Two-factor verification is required."),
            token_expires_at: None,
            requires_2fa: true,
            temp_token,
            user_email_masked,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Login2faRequest {
    pub temp_token: String,
    pub totp_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub group_id: Option<i64>,
    pub quota: Option<f64>,
    pub expires_in_days: Option<i32>,
    pub rate_limit_5h: Option<f64>,
    pub rate_limit_1d: Option<f64>,
    pub rate_limit_7d: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableGroup {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub platform: Option<String>,
    pub rate_multiplier: Option<f64>,
    pub status: Option<String>,
    pub subscription_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedApiKey {
    pub key: ApiKeySummary,
    pub plaintext_key_once: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteApiKeyRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportDiagnosticReportRequest {
    pub include_system_profile: Option<bool>,
    pub quick_scan: Option<QuickScanRequest>,
    pub include_system_environment_scan: Option<bool>,
    pub include_installer_scan: Option<bool>,
    pub include_network_scan: Option<bool>,
    pub network_scan: Option<NetworkScanRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    pub generated_at: String,
    pub app_version: String,
    pub system_profile: Option<SystemProfile>,
    pub quick_scan: Option<QuickScanResult>,
    pub system_environment_scan: Option<SystemEnvironmentScanResult>,
    pub installer_scan: Option<InstallerScanResult>,
    pub network_scan: Option<NetworkScanResult>,
    pub account_status: AccountStatus,
    pub api_keys: ApiKeyList,
    pub redaction: ReportRedactionInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportRedactionInfo {
    pub sensitive_values_redacted: bool,
    pub includes_raw_secrets: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemEnvironmentScanResult {
    pub scanned_at: String,
    pub status: ScanOverallStatus,
    pub checks: Vec<ScanCheck>,
    pub findings: Vec<Finding>,
    pub profile: SystemProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallerScanResult {
    pub scanned_at: String,
    pub status: ScanOverallStatus,
    pub items: Vec<InstallerItem>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallerItem {
    pub id: String,
    pub name: String,
    pub category: InstallerCategory,
    pub status: InstallerStatus,
    pub version: Option<String>,
    pub detail: String,
    pub required: bool,
    pub install_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallerCategory {
    Runtime,
    DeveloperTool,
    Container,
    SystemComponent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallerStatus {
    Installed,
    Missing,
    NeedsAttention,
    Unsupported,
}
