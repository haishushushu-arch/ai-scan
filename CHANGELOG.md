# Changelog

All notable changes to AI Scan should be recorded here.

This project uses operational release notes instead of marketing summaries. Each entry should separate shipped capability, interface-adaptation work, planned work, and known limitations.

## Unreleased

### Fixed

- Fixed a runtime Rustls panic during TLS checks by selecting the `ring` CryptoProvider explicitly at app startup and narrowing direct Rustls/Tokio-Rustls features to `ring`.
- Fixed a release artifact mix-up where a raw `cargo build --release` Tauri binary could load stale `http://localhost:5173` WebView content from another project. The Tauri dev URL now matches the Vite dev server on `127.0.0.1:1420`, stale WebView cache was cleared during verification, and release artifacts are generated only through `npm run tauri:build`.
- Fixed Windows bundling configuration by including the existing `.ico` icon in `tauri.conf.json`.

### Added

- Added real-time Home Health Check progress. The backend now emits `quick-scan-progress` events for start, each scan item start/finish, completion, and failures; the Home page shows a progress bar, current step, pass/warn/fail counts, per-step result, and duration from the real scanner.
- Added real desktop account UI for email/password login, two-factor login challenges, logout, account refresh, and recharge link routing from public settings.
- Added real API Key list/create/delete UI wired to the Tauri commands, including one-time display for newly created plaintext keys when the backend returns them.
- Added Home Health Check inputs for API base URL and optional API Key so users can run the real DNS/TCP/TLS/HTTP/models scan against their target.
- Added Professional Mode diagnostic-report preview using the Rust `export_diagnostic_report` command.
- Added `docs/status.md` as the single source of truth for capability status.
- Added release discipline, capability honesty checks, and documentation publishing red lines to release and agent guidance.
- Established the documentation structure under `docs/` with user manual, troubleshooting, professional guide, developer/API documentation, and maintenance rules.
- Added a static product website under `website/` focused on the msutools desktop client scope.
- Added a release report template and initial development snapshot under `RELEASES/`.
- Expanded the ordinary-user manual with a 3-minute first-run workflow, plain-language result handling, and support-sharing guidance.
- Produced verified Windows release artifacts: the Tauri executable, MSI installer, and NSIS setup executable.

### Documented Current Capability

- Desktop shell and navigation are present.
- System profile collection is implemented in the Tauri backend.
- Quick scan checks API base URL, DNS, TCP, TLS, HTTP reachability, and `/v1/models` when a Key is supplied, with streamed progress events visible in the UI.
- Diagnostic report building exists in Rust.
- The website now states that the current focus is local health checks and API connectivity, with Windows MSI and NSIS artifacts available from the local build output.

### Build Notes

- `npm run tauri:build` completed successfully and generated:
  - `src-tauri/target/release/bundle/msi/ai-scan_0.1.0_x64_en-US.msi`
  - `src-tauri/target/release/bundle/nsis/ai-scan_0.1.0_x64-setup.exe`
- `cargo build --release` must not be used as the release-delivery path for this Tauri app because it can leave frontend asset embedding and WebView cache behavior outside the tested bundle workflow.

### Backend Present, UI/Live Verification Needed

- msutools public settings, login/logout, stored-session account status, and API Key list/create/delete commands are present in the Rust backend and now have visible UI.
- Account identity, balance, quota, recharge links, and API Key workflows still need live test-account verification before docs can call them complete.

### Needs Interface Adaptation

- Recharge entry and payment flow.
- API Key disable/copy workflow, depending on backend capability.

### Planned

- Client configuration discovery and safe writes.
- Repair plans and one-click repair.
- Environment dependency installers.
