---
name: auth-preflight
description: Check or prepare the exact credentials needed for authenticated work, using workctl for supported targets and provider-specific auth elsewhere.
---

Resolve the credential needed for the requested action, check or obtain access,
then continue the task. Auth preparation is the deliverable only for auth-only
requests.

## Resolve scope and intent

Use the selected tool's auth requirements first, then the user's explicit target,
local documentation, and the intended action. Identify the relevant service,
audience, account, environment, groups, token type, and MFA requirement. Request
only the scope needed. A queried dataset's environment does not necessarily
identify the auth environment.

Ask only when unresolved alternatives require materially different credentials.
Browser sign-in and MFA are expected parts of authorized renewal or productive
work; initiate the supported flow when needed and surface its prompt promptly.

For status-only requests, check access without explicitly renewing credentials or
initiating sign-in/MFA. Providers may refresh transparently during a check. For
explicit renewal, renew the selected credential without running unrelated queries.

## Prefer workctl when it covers the credential

Use the [workctl skill](../workctl/SKILL.md) to choose the narrowest matching
profile or an exact custom descriptor. Confirm unfamiliar options with installed
command help. Always specify a profile: the default `work` profile covers multiple
services.

| Intent | Command |
| --- | --- |
| Check status | `workctl auth preflight --profile PROFILE --check --json` |
| Prepare for work | `workctl auth preflight --profile PROFILE --reuse-valid --json` |
| Explicitly renew | `workctl auth preflight --profile PROFILE --json` |

Workctl owns checks, renewal, bounded retries, and verification. Read the exit
status and structured outcome. Choose profiles from the clients the task actually needs: `bitbucket` for native
`bb`, `twg` for TWG OAuth, and `twg-bitbucket` for `twg bb`/`twg bitbucket`.
Use `code` for work requiring all three; its aggregate failure does not make a
healthy individual client unusable. For Nebulae/local runtime work, add `--include-profile runtime`; use a
custom descriptor for an exact Kerberos principal or additional SLAuth audience.
Resolve runtime signing keys, service/user-context tokens, source-access identity,
browser and registry auth separately; the default profile does not prove them.
Manually managed tokens are checked in every mode, never automatically rotated.

After success, continue the requested action without duplicating provider checks. Readiness at handoff does not guarantee
future access or permission for every operation.

On failure, address the reported target and recovery action before rerunning.
Do not automatically repeat preflight or fall through to direct provider renewal.
A credential for a different client, identity, or audience is not equivalent.

Report the exact failing target and observed status. `invalid-or-expired` does not
prove expiration; an earlier chat's failure is not evidence of current failure.
For Bitbucket work, a failed TWG token does not invalidate native `bb` OAuth.
When native `bb` supports the required operation, verify and use it before a
browser fallback. Discover supported commands with installed help; do not copy
credentials between clients. Renew a failed native credential through workctl's
supported flow and retry safely before treating browser access as necessary.
Use the browser when the user requests it, the CLI lacks the required capability,
or the relevant CLI path remains blocked after bounded recovery.

## Provider-specific fallback

Use this path when workctl is unavailable or does not cover the exact credential.
Follow the selected provider's documented auth flow.

- Prefer a status/cache check or the cheapest safe read-only access probe. If no
  cheaper proof exists, the intended read-only call may serve as the probe. Never
  use a mutation as an auth probe.
- Renew missing, expired, or invalid credentials when productive work requires
  access or renewal was requested. For status-only requests, report the state.
- After an inconclusive renewal, check status once before further issuance. If
  human sign-in is still required, surface the supported prompt and continue
  independent work while waiting. Do not loop on unchanged auth.

## Handle failures without repeating unsafe work

A later auth failure during productive work permits one renewal of the same
credential and one safe retry. Use workctl explicit-renewal mode for supported
credentials; use the provider-specific path otherwise. For a mutation, retry only
when the tool guarantees no side effect occurred or a status/idempotency check
proves retry safety. Otherwise report the uncertain operation state and stop
before repeating it. Status-only requests never trigger explicit renewal.

Distinguish invalid credentials from permission/policy denial, malformed requests,
rate limits, outages, and tool failures. Renewing auth does not repair those
failures. If the safe retry fails, stop that credential path and report the exact
access gap; continue independent work or use a verified equivalent client.

A credential-store write blocked by the sandbox is a filesystem restriction.
For authorized renewal or productive work, request escalation for the exact
failed auth command once, then continue after success. Do not redirect HOME or
config directories, copy credential stores, or inspect secrets to bypass it.
Offer a manual command only when escalation is unavailable or declined.

## Protect credentials and report the outcome

Never expose tokens, cookies, auth headers, credential files, or secret-bearing
output. Do not commit auth artifacts.

For auth-only requests, report the target, outcome, and exact blocker. For
productive work, report the task result and mention auth when it affects the
outcome or requires user action.
