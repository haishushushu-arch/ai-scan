# AI Scan Documentation

AI Scan is the msutools desktop client for checking local AI API readiness. It helps ordinary users inspect account, API Key, balance, recharge entry, local environment, network status, client configuration signals, and runtime readiness, while giving professional users inspectable diagnostic evidence.

This documentation is split by audience. Every page must keep completed capability, planned capability, and capability that needs backend/interface adaptation separate.

## Current Capability Status

[Feature status](status.md) is the single source of truth for what users can rely on today.

Status words used across the docs:

- `Available`: users can complete the workflow in the current product, with results backed by real local checks or real API responses.
- `Partially available`: real capability exists, but the workflow still has UI, integration, export, or edge-case gaps.
- `In integration`: commands, page entries, or contracts exist, but ordinary users should not yet rely on the workflow as complete.
- `Planned`: product intent only; do not describe it as usable.

Single-status maintenance rule: do not copy the full status table into user manuals, website pages, release notes, or support scripts. Link to `docs/status.md`, and update that page first when a capability changes.

## Directory

- [Feature Status](status.md): the only complete status page for shipped, partial, integrating, and planned capabilities.
- [User Manual](user/manual.md): ordinary-user workflow for account, API Key, balance, recharge, environment health check, and repair expectations.
- [Troubleshooting](user/troubleshooting.md): problem-to-action guide for common API, Key, network, TLS, proxy, and client-configuration issues.
- [Professional Guide](professional/professional-guide.md): evidence interpretation for operators and advanced users.
- [Developer/API Documentation](developer/api.md): Tauri command contracts, data models, current gaps, and integration requirements.
- [Maintenance Rules](maintenance/documentation-rules.md): rules for keeping docs honest and sustainable.

## Maintenance Rules

1. Do not document a feature as available unless there is a wired UI path, a real command/API behind it, and a verified response shape.
2. Use the status words from `docs/status.md` when status matters.
3. When backend contracts change, update `docs/developer/api.md` before updating user-facing instructions.
4. When a release changes user-visible behavior, update `CHANGELOG.md` and add a release note under `RELEASES/`.
5. Do not paste raw secrets into examples. Mask API Keys, account IDs, access tokens, cookies, and proxy credentials.
