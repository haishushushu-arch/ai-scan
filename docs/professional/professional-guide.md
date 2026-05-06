# Professional Guide

This guide is for operators, support engineers, and advanced users who need evidence rather than a simple pass/fail answer.

## Diagnostic Model

The current Rust backend exposes these evidence sources:

- `SystemProfile`: host OS, version, kernel, architecture, hostname, username, CPU, memory, shell, PATH, selected environment variables, and selected tool versions.
- `QuickScanResult`: normalized target, started/finished timestamps, checks, findings.
- `DiagnosticReport`: combined report with redaction metadata.

The report builder exists in Rust. A user-facing export workflow still needs UI wiring.

## Quick Scan Order

The scanner runs in dependency order:

1. Target normalization.
2. DNS lookup.
3. TCP connect.
4. TLS handshake for HTTPS URLs.
5. HTTP reachability at the base URL.
6. `GET /v1/models` when an API Key is supplied.

Interpret failures in that order. For example, a `/v1/models` failure is not meaningful if TCP cannot connect.

## Check Semantics

| Check | Pass Means | Common Failure Meaning |
| --- | --- | --- |
| `dns` | Host resolved to at least one socket address. | Bad hostname, resolver issue, VPN/proxy issue. |
| `tcp` | A TCP socket opened to the first resolved address. | Firewall, blocked port, dead origin, route issue. |
| `tls` | HTTPS handshake completed with trusted roots. | Bad certificate, wrong SNI, broken origin TLS, interception, wrong system time. |
| `http` | Endpoint responded with success, redirect, 401, 403, or 404. | Route exists but unexpected service behavior, proxy issue, timeout. |
| `models` | `/v1/models` returned 2xx. | Bad Key, missing permission, wrong base URL, account/balance issue, service incompatibility. |

## Redaction

The environment collector treats sensitive variable names as redaction candidates. Diagnostic reports state:

- `sensitiveValuesRedacted: true`
- `includesRawSecrets: false`

Any future raw-log export must keep this guarantee or explicitly label the export as unsafe.

## Current Account and Key Integration Boundary

The current Rust backend contains msutools commands for:

- Public settings.
- Login and logout.
- Stored session based account status.
- Live account identity and mapped balance/quota fields when `/auth/me` returns them.
- API Key list/create/delete through `/keys`.

The following remain not proven as complete user-facing capabilities:

- Visible login flow in the current React surface.
- Visible recharge action.
- End-to-end verified account/billing response compatibility.
- Frontend support for Key creation and deletion.
- API Key disable/enable if the backend contract supports it.
- Client configuration detection and writing.
- Repair plan generation.
- Environment dependency installation.

When triaging a user report, separate product gaps from user-side failures. A missing balance in the current build is expected until the account/billing interface is connected.

## Evidence Checklist

For a support-grade report, collect:

- AI Scan version.
- Operating system and architecture.
- Target base URL.
- Full quick-scan check list with statuses and evidence.
- Whether an API Key was supplied, without exposing the Key.
- Masked account identity only after account integration exists.
- Timestamp and network context, such as VPN/proxy/CDN path.

## Suggested Severity Mapping

- High: DNS/TCP/TLS failure, 401/403 on a known-valid Key, or diagnostics unable to run.
- Medium: HTTP unexpected status, models endpoint warning, missing required runtime.
- Low: optional tool missing, stale client configuration, nonblocking environment warning.
- Info: successful checks and contextual evidence.
