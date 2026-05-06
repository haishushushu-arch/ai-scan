# Developer and Interface Documentation

This document defines the current desktop/backend contract and the live verification work needed before account, billing, recharge, and API Key features can be described as fully complete.

## Runtime Architecture

- Frontend: React app under `src/`.
- Desktop backend: Tauri/Rust under `src-tauri/`.
- Documentation and website ownership: `docs/`, `website/`, `CHANGELOG.md`, `RELEASE_TEMPLATE.md`, `RELEASES/`.

This document is informational only. Do not modify source files from this documentation task.

## Tauri Commands

### `get_system_profile`

Status: Implemented.

Returns a `SystemProfile` with:

- OS, OS version, kernel version.
- Architecture.
- Hostname and username when available.
- CPU brand, CPU core count, memory.
- Shell.
- PATH entries.
- Selected environment variables with sensitive values redacted.
- Tool versions for node, npm, git, curl, and docker.
- Generation timestamp.

### `run_quick_scan`

Status: Implemented.

Input:

```json
{
  "baseUrl": "https://api.example.com",
  "apiKey": "sk-***",
  "timeoutMs": 8000
}
```

Behavior:

- Adds `https://` when no scheme is provided.
- Removes trailing `/v1`.
- Clamps timeout between 1,000 ms and 30,000 ms.
- Runs DNS, TCP, TLS, HTTP, and optional `/v1/models` checks.
- Redacts response body preview before storing evidence.

Output includes normalized target, timestamps, checks, and findings.

### `run_quick_scan_streamed`

Status: Implemented and used by the Home Health Check UI.

Input is the same as `run_quick_scan`. The command returns the same `QuickScanResult`, but also emits Tauri events while it runs:

- Event name: `quick-scan-progress`
- Phases: `started`, `step_started`, `step_finished`, `finished`, `failed`
- Progress payload: run ID, phase, integer progress percent, completed count, total count, current step ID/title, user-facing message, optional `ScanCheck`, and timestamp.

The current step order is DNS, TCP, TLS, HTTP, and `/v1/models`. UI progress must be derived from these events rather than from a timer or fake animation.

### `get_public_settings`

Status: Implemented; live response verification needed.

Calls `GET https://www.msutools.cn/api/v1/settings/public` and expects a standard envelope with public site/payment fields.

### `login`

Status: Implemented with visible UI; live account verification needed.

Input:

```json
{
  "email": "user@example.com",
  "password": "password",
  "turnstileToken": null
}
```

Calls `/auth/login`, stores access/refresh tokens locally, and maps the returned user into `AccountStatus`.

If the server returns a two-factor challenge, the command returns `requires2fa: true` and does not store a partial session. The frontend then calls `login_2fa`.

### `login_2fa`

Status: Implemented with visible UI; live account verification needed.

Input:

```json
{
  "tempToken": "temporary-server-token",
  "totpCode": "123456"
}
```

Calls `/auth/login/2fa`, stores access/refresh tokens only after successful verification, and maps the returned user into `AccountStatus`.

### `logout`

Status: Implemented with visible UI; live account verification needed.

Loads the stored session, attempts `/auth/logout` when a refresh token exists, and clears local session storage.

### `get_account_status`

Status: Implemented with visible UI; live account verification needed.

Current behavior: returns logged-out when no local session exists; otherwise calls `/auth/me` and maps user, masked email, balance, and quota/status text.

Required verification:

- Login response envelope and token lifetime.
- `/auth/me` field compatibility.
- Expired-token behavior and refresh strategy.
- Whether recharge URL comes from public settings or another billing endpoint.
- Turnstile desktop challenge behavior when public settings require it.

### `list_api_keys`

Status: Implemented with visible UI for list/create/delete; live account verification needed.

Current behavior: returns unconfigured when no session exists; otherwise calls `/keys?page=1&page_size=50` and maps items into masked `ApiKeySummary`.

Related commands:

- `create_api_key` posts to `/keys` and returns `plaintextKeyOnce` only when the backend returns a non-masked full key.
- `delete_api_key` deletes `/keys/{id}`.

Required verification:

- Endpoint response envelope and pagination shape.
- Whether `status` values match frontend expectations.
- Permission model for create/delete/disable.
- Editing, disabling, copying, grouping, IP allow/deny lists, and rate-limit fields.

### `export_diagnostic_report`

Status: Implemented in Rust model/report builder and visible in Professional Mode as a JSON preview.

Input:

```json
{
  "includeSystemProfile": true,
  "quickScan": {
    "baseUrl": "https://api.example.com",
    "apiKey": "sk-***",
    "timeoutMs": 8000
  }
}
```

Output includes app version, optional system profile, optional quick scan, account status, API Key list, and redaction metadata.

Current UI behavior:

- Runs the report command from Professional Mode.
- Uses the current Home Health Check API base URL and optional API Key.
- Shows JSON preview in the app.
- Does not yet save to a file or upload to support.

## Integration Requirements Before Marking Features Complete

Account, balance, recharge, and API Key features must not be marked complete until all of the following are true:

1. A real endpoint or command contract is documented.
2. The frontend calls that contract.
3. Errors are represented without fake fallback data.
4. Secrets are masked in UI and reports.
5. A manual test case exists for success, unauthenticated, permission denied, and service unavailable.
6. User documentation is updated with exact behavior.

## Repair Command Requirements

Before implementing one-click repair, backend commands should return a repair plan:

```json
{
  "findingId": "tls_fail",
  "risk": "requires_confirmation",
  "actions": [
    {
      "type": "edit_file",
      "path": "C:/Users/example/.config/tool/config.json",
      "backupPath": "C:/Users/example/.config/tool/config.json.bak",
      "preview": "diff or structured change"
    }
  ]
}
```

The frontend should require confirmation for writes and must show backup and revert instructions.

## Compatibility Notes

The scanner is OpenAI-compatible API oriented. `/v1/models` is used as the first authenticated check because it is low-risk and broadly supported. Providers that do not expose this endpoint need adapter logic before they can be treated as supported.
