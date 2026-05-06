use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::models::{
    AccountBalance, AccountStatus, AccountUser, ApiKeyList, ApiKeySummary, CreatedApiKey,
    CreateApiKeyRequest, DeleteApiKeyRequest, Login2faRequest, LoginRequest, LoginResult,
    PublicSettings,
};
use crate::core::redaction;
use crate::storage::{self, SessionTokens};

const DEFAULT_BASE: &str = "https://www.msutools.cn";
const API_PREFIX: &str = "/api/v1";

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    code: ApiCode,
    message: Option<String>,
    data: Option<T>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ApiCode {
    Number(i64),
    Text(String),
}

impl ApiCode {
    fn is_success(&self) -> bool {
        match self {
            Self::Number(value) => *value == 0,
            Self::Text(value) => value == "0" || value.eq_ignore_ascii_case("success"),
        }
    }

    fn as_display(&self) -> String {
        match self {
            Self::Number(value) => value.to_string(),
            Self::Text(value) => value.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct AuthResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    user: UserDto,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LoginResponse {
    Authenticated(AuthResponse),
    Requires2fa(TotpLoginDto),
}

#[derive(Debug, Deserialize)]
struct TotpLoginDto {
    requires_2fa: bool,
    temp_token: Option<String>,
    user_email_masked: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserDto {
    id: i64,
    username: Option<String>,
    email: Option<String>,
    balance: Option<f64>,
    concurrency: Option<i64>,
    role: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PaginatedResponse<T> {
    items: Vec<T>,
    total: Option<i64>,
    page: Option<i64>,
    page_size: Option<i64>,
    pages: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ApiKeyDto {
    id: i64,
    key: Option<String>,
    name: Option<String>,
    status: Option<String>,
    created_at: Option<String>,
    last_used_at: Option<String>,
    expires_at: Option<String>,
    quota: Option<f64>,
    quota_used: Option<f64>,
}

#[derive(Debug, Serialize)]
struct CreateApiKeyPayload {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    group_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quota: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_in_days: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rate_limit_5h: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rate_limit_1d: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rate_limit_7d: Option<f64>,
}

pub async fn public_settings() -> anyhow::Result<PublicSettings> {
    get_public("/settings/public").await
}

pub async fn login(request: LoginRequest) -> anyhow::Result<LoginResult> {
    let client = reqwest::Client::builder().build()?;
    let url = api_url("/auth/login");
    let payload = serde_json::json!({
        "email": request.email,
        "password": request.password,
        "turnstile_token": request.turnstile_token,
    });
    let response = client.post(url).json(&payload).send().await?;
    let login: LoginResponse = unwrap_response(response).await?;
    match login {
        LoginResponse::Authenticated(auth) => persist_auth(auth),
        LoginResponse::Requires2fa(challenge) => {
            if !challenge.requires_2fa {
                anyhow::bail!("msutools login returned an unsupported response shape");
            }
            Ok(LoginResult::requires_2fa(
                challenge.temp_token,
                challenge.user_email_masked,
            ))
        }
    }
}

pub async fn login_2fa(request: Login2faRequest) -> anyhow::Result<LoginResult> {
    let client = reqwest::Client::builder().build()?;
    let url = api_url("/auth/login/2fa");
    let payload = serde_json::json!({
        "temp_token": request.temp_token,
        "totp_code": request.totp_code,
    });
    let response = client.post(url).json(&payload).send().await?;
    let auth: AuthResponse = unwrap_response(response).await?;
    persist_auth(auth)
}

fn persist_auth(auth: AuthResponse) -> anyhow::Result<LoginResult> {
    let expires_at_unix_ms = auth
        .expires_in
        .map(|seconds| Utc::now().timestamp_millis() + seconds.saturating_mul(1000));
    let token_expires_at = expires_at_unix_ms
        .and_then(chrono::DateTime::<Utc>::from_timestamp_millis)
        .map(|value| value.to_rfc3339());

    storage::save_session(&SessionTokens {
        access_token: auth.access_token,
        refresh_token: auth.refresh_token,
        expires_at_unix_ms,
    })?;

    Ok(LoginResult::authenticated(
        account_from_user(auth.user),
        token_expires_at,
    ))
}

pub async fn logout() -> anyhow::Result<()> {
    let session = storage::load_session()?;
    if let Some(session) = session {
        if let Some(refresh_token) = session.refresh_token {
            let client = authed_client(&session.access_token)?;
            let _ = client
                .post(api_url("/auth/logout"))
                .json(&serde_json::json!({ "refresh_token": refresh_token }))
                .send()
                .await;
        }
    }
    storage::clear_session()
}

pub async fn account_status() -> anyhow::Result<AccountStatus> {
    let Some(session) = storage::load_session()? else {
        return Ok(AccountStatus::unauthenticated(
            "No msutools login session is stored on this device.",
        ));
    };

    let client = authed_client(&session.access_token)?;
    let response = client.get(api_url("/auth/me")).send().await?;
    if response.status().as_u16() == 401 {
        return Ok(AccountStatus::unauthenticated(
            "The stored msutools session expired. Please log in again.",
        ));
    }
    let user: UserDto = unwrap_response(response).await?;
    Ok(account_from_user(user))
}

pub async fn list_api_keys() -> anyhow::Result<ApiKeyList> {
    let Some(session) = storage::load_session()? else {
        return Ok(ApiKeyList::unconfigured(
            "Log in to msutools before listing API keys.",
        ));
    };

    let client = authed_client(&session.access_token)?;
    let response = client
        .get(api_url("/keys"))
        .query(&[("page", "1"), ("page_size", "50")])
        .send()
        .await?;

    if response.status().as_u16() == 401 {
        return Ok(ApiKeyList::unconfigured(
            "The stored msutools session expired. Please log in again.",
        ));
    }

    let page: PaginatedResponse<ApiKeyDto> = unwrap_response(response).await?;
    let _ = (page.total, page.page, page.page_size, page.pages);
    Ok(ApiKeyList {
        configured: true,
        keys: page.items.into_iter().map(api_key_from_dto).collect(),
        message: "API keys loaded from msutools.".to_string(),
    })
}

pub async fn create_api_key(request: CreateApiKeyRequest) -> anyhow::Result<CreatedApiKey> {
    let session = require_session()?;
    let client = authed_client(&session.access_token)?;
    let payload = CreateApiKeyPayload {
        name: request.name,
        group_id: request.group_id,
        quota: request.quota,
        expires_in_days: request.expires_in_days,
        rate_limit_5h: request.rate_limit_5h,
        rate_limit_1d: request.rate_limit_1d,
        rate_limit_7d: request.rate_limit_7d,
    };
    let response = client.post(api_url("/keys")).json(&payload).send().await?;
    let key: ApiKeyDto = unwrap_response(response).await?;
    let plaintext_key_once = key.key.as_ref().and_then(|value| {
        if value.trim().is_empty() || value.contains('*') {
            None
        } else {
            Some(value.clone())
        }
    });
    Ok(CreatedApiKey {
        key: api_key_from_dto(key),
        plaintext_key_once,
    })
}

pub async fn delete_api_key(request: DeleteApiKeyRequest) -> anyhow::Result<String> {
    let session = require_session()?;
    let client = authed_client(&session.access_token)?;
    let path = format!("/keys/{}", request.id);
    let response = client.delete(api_url(&path)).send().await?;
    let _: Value = unwrap_response(response).await?;
    Ok("API key deleted.".to_string())
}

pub fn base_url() -> &'static str {
    DEFAULT_BASE
}

fn api_url(path: &str) -> String {
    format!("{DEFAULT_BASE}{API_PREFIX}{path}")
}

async fn get_public<T: DeserializeOwned>(path: &str) -> anyhow::Result<T> {
    let client = reqwest::Client::builder().build()?;
    let response = client.get(api_url(path)).send().await?;
    unwrap_response(response).await
}

async fn unwrap_response<T: DeserializeOwned>(response: reqwest::Response) -> anyhow::Result<T> {
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        anyhow::bail!(
            "msutools request failed with HTTP {}: {}",
            status.as_u16(),
            redaction::redact_secret(&text.chars().take(240).collect::<String>())
        );
    }

    let envelope: ApiResponse<T> = serde_json::from_str(&text).map_err(|error| {
        anyhow::anyhow!(
            "msutools response was not a standard JSON envelope: {error}; preview={}",
            redaction::redact_secret(&text.chars().take(240).collect::<String>())
        )
    })?;

    if !envelope.code.is_success() {
        anyhow::bail!(
            "msutools API error {}: {}",
            envelope.code.as_display(),
            envelope.message.unwrap_or_else(|| "Unknown error".to_string())
        );
    }

    envelope
        .data
        .ok_or_else(|| anyhow::anyhow!("msutools API returned an empty data field"))
}

fn authed_client(access_token: &str) -> anyhow::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {access_token}"))?,
    );
    Ok(reqwest::Client::builder().default_headers(headers).build()?)
}

fn require_session() -> anyhow::Result<SessionTokens> {
    storage::load_session()?.ok_or_else(|| anyhow::anyhow!("Log in to msutools first."))
}

fn account_from_user(user: UserDto) -> AccountStatus {
    let display_name = user
        .username
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string);
    let email_masked = user.email.as_deref().map(mask_email);
    let balance = AccountBalance {
        amount: user.balance.map(|value| format!("{value:.4}")),
        unit: Some("USD".to_string()),
        updated_at: None,
    };
    let quota_text = user
        .concurrency
        .map(|value| format!("并发 {value}"))
        .or_else(|| user.role.map(|role| format!("角色 {role}")))
        .or(user.status.map(|status| format!("状态 {status}")));
    AccountStatus::authenticated(
        AccountUser {
            id: Some(user.id.to_string()),
            display_name,
            email_masked,
        },
        balance,
        quota_text,
    )
}

fn api_key_from_dto(key: ApiKeyDto) -> ApiKeySummary {
    let status = key.status.unwrap_or_else(|| "unknown".to_string());
    let masked = key
        .key
        .as_deref()
        .map(mask_key)
        .unwrap_or_else(|| "sk-****".to_string());
    let mut summary = ApiKeySummary::new(
        key.id.to_string(),
        key.name.unwrap_or_else(|| format!("API Key {}", key.id)),
        masked,
        status,
        key.created_at,
        key.last_used_at,
    );
    summary.expires_at = key.expires_at;
    summary.quota_text = key.quota.map(|value| {
        if value <= 0.0 {
            "不限额".to_string()
        } else {
            format!("{value:.4} USD")
        }
    });
    summary.usage_text = key.quota_used.map(|value| format!("已用 {value:.4} USD"));
    summary
}

fn mask_email(email: &str) -> String {
    let Some((name, domain)) = email.split_once('@') else {
        return redaction::redact_secret(email);
    };
    let prefix: String = name.chars().take(2).collect();
    format!("{prefix}***@{domain}")
}

fn mask_key(key: &str) -> String {
    let trimmed = key.trim();
    if trimmed.len() <= 12 {
        return "sk-****".to_string();
    }
    let start: String = trimmed.chars().take(6).collect();
    let end: String = trimmed.chars().rev().take(4).collect::<String>().chars().rev().collect();
    format!("{start}...{end}")
}
