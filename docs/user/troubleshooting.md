# Troubleshooting

Use this guide after running a quick scan. Start with the first failing check because later checks often depend on earlier ones.

## Quick Status Table

| Symptom | Likely Area | First Action |
| --- | --- | --- |
| API base URL is missing | Configuration | Enter the msutools OpenAI-compatible API base URL. |
| DNS lookup failed | Network/DNS | Check domain spelling, DNS resolver, VPN, and proxy. |
| TCP connect failed or timed out | Network/firewall | Check firewall, VPN, proxy, and whether the service is reachable on the expected port. |
| TLS handshake failed | Certificate/HTTPS | Check system time, certificate trust, HTTPS interception, and origin certificate. |
| HTTP returned unexpected status | Service route | Confirm the URL is the API service route, not only a website homepage. |
| `/v1/models` returned 401/403 | API Key | Confirm the API Key, prefix, permission, and account balance. |
| `/v1/models` skipped | Missing Key | Provide an API Key before expecting model validation. |
| Account or balance says unconfigured | Login/session or UI integration | Confirm login session first; some visible UI actions still need wiring and live verification. |

## API Base URL

The quick scan normalizes common input:

- `api.example.com` becomes `https://api.example.com`.
- A trailing `/v1` is removed before the scan builds `/v1/models`.

Use the OpenAI-compatible service root as the base URL. Do not paste a chat-completion endpoint as the base URL.

## DNS Problems

If DNS fails:

1. Check spelling of the hostname.
2. Try another network.
3. Disable or change VPN/proxy temporarily.
4. Check whether the system DNS resolver is blocked or hijacked.

Professional users should compare system resolver output with public resolver output before blaming the API service.

## TCP Problems

If TCP fails:

1. Confirm the service is listening on the expected port, usually 443 for HTTPS.
2. Check local firewall and endpoint security software.
3. Check VPN split-tunnel rules.
4. Test from a different network.

TCP success only means the socket opened. It does not prove the API route or Key works.

## TLS Problems

If TLS fails:

1. Verify the system date and time.
2. Check whether antivirus, enterprise proxy, or VPN is intercepting HTTPS.
3. Confirm the hostname certificate is valid for the domain.
4. If the service is behind a CDN or reverse proxy, confirm origin TLS is valid.

## HTTP Reachability Problems

HTTP success includes ordinary responses such as 200, 3xx, 401, 403, and 404 because those prove the server responded. A warning means the endpoint returned an unexpected status or failed before reaching the server.

If HTTP fails after DNS/TCP/TLS pass, check:

- Service route.
- Reverse proxy configuration.
- WAF/CDN rules.
- Whether the path belongs to the website instead of the API.

## API Key Problems

`GET /v1/models` is only requested when an API Key is provided.

If it returns 401 or 403:

1. Check that the Key is copied completely.
2. Do not include extra spaces or quotes.
3. Confirm the Key belongs to the configured base URL.
4. Confirm the account is active and has balance or quota.
5. Rotate the Key if it may have leaked.

Current limitation: Rust commands exist for msutools Key list/create/delete, but the visible desktop UI and live endpoint compatibility still need verification before the workflow should be treated as fully available.

## Account, Balance, and Recharge

If the UI says account, balance, or recharge is waiting for interface adaptation, that is not necessarily a user error. It can mean there is no stored login session, the live msutools response did not match expectations, or the visible desktop action is still pending.

Do not claim balance is zero unless the UI shows a real returned balance value. Pending, logged-out, and interface-error states are different from zero balance.

## Collecting Evidence for Support

Professional evidence should include:

- App version.
- OS and architecture.
- Base URL, with secrets removed.
- Quick scan checks and findings.
- Redacted environment variables.
- Exact time of the test.

Never send raw API Keys, cookies, tokens, or unredacted proxy credentials.
