# AI Scan Release Report Template

Use this template for GitHub-Release-style iteration reports. Copy it to `RELEASES/YYYY-MM-DD-version.md` for each release or milestone.

## Release

- Version:
- Date:
- Build/commit:
- Owner:

## Summary

Write 3-5 sentences describing what changed. Avoid marketing claims; describe verifiable behavior.

## Release Discipline

- Link to the exact build, commit, artifact, or local build command used for verification.
- Separate shipped behavior from backend-only work, UI placeholders, planned work, and live-service dependencies.
- Update `docs/status.md` first when a capability status changes.
- Update user docs, website, changelog, and release notes in the same release when user-visible behavior changes.
- Do not publish a release note that implies an installer exists unless the installer artifact has been built and smoke-tested.

## Capability Honesty Check

Answer each item before publishing:

| Question | Answer | Evidence |
| --- | --- | --- |
| Is there a visible user path for every feature called shipped? |  |  |
| Does every shipped network/account/API claim have live or local verification evidence? |  |  |
| Are account, balance, recharge, and Key-management claims backed by live login validation? |  |  |
| Are unavailable features clearly marked as planned, partial, or in integration? |  |  |
| Are secrets redacted from examples, screenshots, logs, and reports? |  |  |

## Shipped

- 

## Needs Interface Adaptation

List visible UI or documented workflows that still need a real backend/service contract.

- 

## Planned / Not Yet Available

List product intent that users should not expect in this release.

- 

## User Impact

- Ordinary users:
- Professional users/operators:
- Developers/integrators:

## Verification

Record manual or automated checks.

| Check | Result | Evidence |
| --- | --- | --- |
|  |  |  |

## Known Limitations

- 

## Upgrade / Rollback Notes

- 

## Documentation Updates

- User manual:
- Troubleshooting:
- Developer/API docs:
- Website:

## Documentation Publishing Red Lines

Do not publish if any of these are true:

- Website, docs, changelog, or release notes claim account, balance, recharge, Key management, repair, installer, chat, or stream support without verified evidence.
- A placeholder page, disabled button, backend-only command, mock value, HTML 200, or `/health` response is used as proof of user availability.
- `docs/status.md` disagrees with the website, user manual, changelog, or release note.
- Raw API Keys, cookies, tokens, Authorization headers, passwords, proxy credentials, or refresh tokens appear in examples or artifacts.
