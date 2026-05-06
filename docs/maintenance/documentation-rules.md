# Documentation Maintenance Rules

AI Scan documentation must remain operational, verifiable, and honest. This project is not helped by marketing-style claims that get ahead of implementation.

## Source of Truth Order

1. Working code and command contracts.
2. Release notes for shipped behavior.
3. Interface specifications for pending integration.
4. Product plans.

If these disagree, update the docs to match working code first, then record the integration gap.

## Required Labels

Use these labels consistently:

- `Implemented`: wired in UI or command and testable in the current build.
- `Needs interface adaptation`: UI or model exists, but the real msutools service contract is not connected.
- `Planned`: product intent exists, but no usable implementation is present.
- `Deprecated`: feature was removed or should no longer be used.

## Do Not Claim

Do not claim these are complete user-facing workflows until code, UI, live verification, and release notes prove them:

- Live msutools login from the visible app.
- Live balance display in a verified logged-in flow.
- Recharge from the desktop app.
- API Key creation/deletion from the visible app.
- Automatic client config writing.
- One-click repairs.
- Dependency installation.

## Update Checklist

For every user-visible release:

1. Update the relevant user manual section.
2. Update troubleshooting if new errors or checks were added.
3. Update developer/API docs if commands or payloads changed.
4. Update `CHANGELOG.md`.
5. Add or update a release report in `RELEASES/`.
6. Check the website status labels.

## Writing Style

- Prefer concrete behavior over value claims.
- Say what the user can verify.
- Separate ordinary-user instructions from professional diagnostics.
- Include failure modes and next actions.
- Mask secrets in examples.
- Avoid unsupported promises such as "fully automated" or "one click" unless that path exists.
