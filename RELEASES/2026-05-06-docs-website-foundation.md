# AI Scan Iteration Report: Docs and Website Foundation

## Release

- Version: Unreleased documentation snapshot
- Date: 2026-05-06
- Build/commit: Not available in this workspace because `.git` metadata is absent
- Owner: docs/website worker

## Summary

This iteration establishes a maintainable documentation and static website foundation for AI Scan, and records the first visible account/API Key management wiring. The content is based on current code-visible capabilities and explicitly separates implemented behavior, partially available workflows that still need live test-account coverage, and work that still needs adaptation.

## Shipped

- User manual for ordinary users.
- Troubleshooting guide organized by quick-scan failure order.
- Professional diagnostic guide for support and operators.
- Developer/API documentation for Tauri command contracts and integration gaps.
- Documentation maintenance rules.
- Static product website focused on the msutools desktop client.
- Release report template.
- Visible account UI for email/password login, two-factor challenge, logout, account refresh, and recharge link routing from public settings.
- Visible API Key UI for list, create, delete, and one-time display of newly created plaintext Keys when the backend returns them.
- Home Health Check inputs for API base URL and optional API Key.
- Real-time Home Health Check progress with a streamed Tauri event for each scan phase, plus UI progress bar, current step, pass/attention/fail counts, per-step status, message, and duration.
- Professional Mode diagnostic-report JSON preview backed by the Rust report command.
- Runtime Rustls CryptoProvider fix for TLS checks.
- Windows MSI and NSIS setup artifacts from the verified Tauri bundle workflow.

## Partially Available, Live Verification Needed

- Public settings.
- Login/logout and stored-session account status.
- Account identity, balance, and quota mapping.
- API Key list/create/delete workflow.

## Needs Interface Adaptation

- Recharge entry/payment flow.
- API Key disable/copy workflow, depending on backend capability.
- UI export flow for diagnostic reports if the product exposes report downloads.

## Planned / Not Yet Available

- One-click repair.
- Safe client configuration writes.
- Environment dependency installers.
- Provider-specific adapters beyond OpenAI-compatible `/v1/models` checks.

## Verification

| Check | Result | Evidence |
| --- | --- | --- |
| Frontend build | Passed | `npm run build` completed successfully. |
| Rust build check | Passed | `cargo check` completed successfully under `src-tauri`. |
| Rustls feature check | Passed | `cargo tree -e features -i rustls` now shows `ring`, `std`, and `tls12`, without `aws-lc-rs`. |
| Tauri executable build | Passed | `npm run tauri:build` produced `src-tauri/target/release/ai-scan.exe` with the current AI Scan frontend embedded. |
| Runtime smoke test | Passed | Started the rebuilt `ai-scan.exe` by absolute path, captured the UI, confirmed it shows AI Scan rather than stale WebView content, then stopped it. |
| Real-time scan smoke test | Passed | Started the rebuilt app, clicked the Home Health Check action, and captured the live progress UI showing the progress bar, current step, and completed scan rows. |
| MSI bundle | Passed | Generated `src-tauri/target/release/bundle/msi/ai-scan_0.1.0_x64_en-US.msi`. |
| NSIS bundle | Passed | Generated `src-tauri/target/release/bundle/nsis/ai-scan_0.1.0_x64-setup.exe`. |
| WebView stale-content regression | Fixed | Cleared stale `cn.msutools.ai-scan` WebView cache and aligned Tauri `devUrl` with Vite's `127.0.0.1:1420` dev server. |
| Live public settings probe | Passed | `GET https://www.msutools.cn/api/v1/settings/public` returned `200 OK` with a standard JSON envelope. |
| Live unauthenticated Key probe | Passed | `GET https://www.msutools.cn/api/v1/keys` without auth returned `401 Unauthorized`, confirming the route is protected. |
| Capability honesty | Passed | Docs label account, balance, recharge, and API Key management as partially available/live-verification-needed, not fully available. |
| Website static assets | Passed | Website uses standalone HTML/CSS with no build step. |

## Known Limitations

- The website is a static artifact and is not wired into the app build.
- No screenshots are embedded yet.
- Release version is not tied to a Git commit because this workspace has no `.git` directory.
- No live test-account login was performed in this iteration, so Turnstile, successful login, 2FA success, live balance shape, and live API Key create/delete still need controlled test credentials.
- Raw `cargo build --release` output is not a valid release-delivery path for this Tauri app; use `npm run tauri:build` so the frontend assets and bundle metadata are built together.
- Windows installer artifacts are unsigned development builds and still need code signing before public distribution.

## Documentation Updates

- User manual: `docs/user/manual.md`
- Troubleshooting: `docs/user/troubleshooting.md`
- Developer/API docs: `docs/developer/api.md`
- Website: `website/index.html`
