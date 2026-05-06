# AI Scan

AI Scan is a cross-platform desktop diagnostics client for msutools AI API users. It is built with Tauri 2, React, TypeScript, and Rust.

The current focus is first-run support: scan the user's local environment and API connectivity, show real-time per-step feedback, and keep account/API Key capabilities honest until they are verified against live msutools accounts.

## Current Scope

- Home health check for API base URL, DNS, TCP, TLS, HTTP, and optional `/v1/models`.
- Real-time scan progress through Tauri events, with per-step pass/warn/fail/skipped status and duration.
- System profile collection with secret redaction.
- msutools account login/logout/status, 2FA handoff, recharge link routing, and API Key list/create/delete UI wired to real commands.
- Professional diagnostic report preview with redacted data.
- User docs, developer docs, static website, changelog, and release notes.

Feature status is maintained in [docs/status.md](docs/status.md).

## Development

Install dependencies:

```bash
npm install
```

Run the desktop app in development:

```bash
npm run tauri:dev
```

Build the frontend:

```bash
npm run build
```

Build desktop release artifacts:

```bash
npm run tauri:build
```

Do not use raw `cargo build --release` as the release-delivery path for this Tauri app. Use `npm run tauri:build` so the frontend assets and bundle metadata are built together.

## Documentation

- [User manual](docs/user/manual.md)
- [Troubleshooting](docs/user/troubleshooting.md)
- [Professional guide](docs/professional/professional-guide.md)
- [Developer/API documentation](docs/developer/api.md)
- [Documentation rules](docs/maintenance/documentation-rules.md)
- [Changelog](CHANGELOG.md)
- [Release reports](RELEASES/)

## Release Artifacts

Local build outputs are intentionally ignored by Git:

- `dist/`
- `src-tauri/target/`
- `.probe/`
- `.upstream/`

Windows MSI/NSIS packages are produced by `npm run tauri:build` and should be attached to formal releases after code signing and smoke testing.

GitHub Actions also builds desktop bundles automatically through `.github/workflows/release.yml`:

- Publishing a GitHub Release uploads Windows x64, macOS Apple Silicon, and Linux x64 Tauri bundles to that release.
- Pushing a tag such as `v0.1.0` or running the workflow manually builds bundles and stores them as workflow artifacts.
- Current public builds are unsigned. Add platform signing secrets before treating release assets as production installers.
