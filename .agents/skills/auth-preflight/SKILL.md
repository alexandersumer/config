---
name: auth-preflight
description: Verify access and proactively request exact service-specific auth before using auth-backed tools, MCPs, CLIs, or services, including browser/MFA flows without retry loops.
---

Use this skill before operating an auth-backed tool, MCP server, CLI, service API, logs system, deployment tool, SLO tool, mail tool, tenant lookup tool, or similar integration when the work depends on valid access.

## Goal

Resolve the concrete auth target, prove or obtain access when a safe proof path exists, then make the intended tool call. Skill activation is not the deliverable: do not stop after saying auth will be checked.

Browser or MFA is only a confirmation mechanism. The real target is the narrow service-specific credential: tool, audience, account, group, token type, auth environment, and MFA mode required for the intended action.

If the user only asks to check auth status, run only the status/cache check or safe read-only proof and report the result; do not generate a token or trigger MFA unless the user also asked to refresh/request auth or a subsequent requested productive action depends on it. If the user asks to refresh or request auth, the auth preflight itself is the intended action. Do not run unrelated service queries after refreshing access.

## Resolve the target

Use this order, stopping as soon as the target is clear:

1. Selected tool, MCP, CLI command, API, or productive action. If the tool is already chosen, its documented auth path wins over generic wording in the user request.
2. Explicit service named by the user, issue, runbook, error, or current task.
3. Repo docs, tool schema, CLI help, local instructions, or known service defaults.
4. Domain inference from the intended action. The user does not need to name the auth backend when the action or tool choice implies it.

For log-search work, infer the logs backend from the intended query path. In Atlassian/Ops Sherpa contexts, generic “logs”, “staging logs”, “query logs”, Splunk syntax, or a Splunk-backed logs tool means Splunk auth unless local docs or tool schema point to a different logs service.

Ask one concise question only when multiple plausible targets remain after using the current task, selected tool, and local instructions, and those targets require materially different credentials. Do not ask the user to approve browser/MFA or to restate an auth backend that is already implied.

## Preflight algorithm

1. Build the exact credential descriptor: service/tool, audience, account, group, token type, auth environment, MFA requirement, and the read/write scope needed.
2. Prefer the narrowest credential that satisfies the intended action. Do not request every known audience, broad groups, or unrelated environments as a shortcut.
3. If a status/cache command exists, run it before the first productive call.
4. If access is missing, expired, invalid, or MFA-required, immediately refresh or generate that exact credential for refresh/request tasks and for productive work that depends on access. For check-only tasks, report the missing/expired/MFA-required state without generating unless the user asked to refresh. Browser/MFA prompts are expected confirmation when refresh or productive work requires them.
5. If token generation times out or returns an unclear result, check the cache/status once before declaring failure.
6. If no status check exists, use the cheapest safe read-only probe that proves access to the intended target. If the intended call is read-only and no cheaper probe exists, it may be the first call; handle any auth-shaped failure with the one-retry rule below. For mutating workflows, never use the mutation itself as the auth probe; find a separate safe probe or report the blocker.

## Execute without looping

1. Run the intended tool call after preflight passes. If no status/cache check or cheaper safe probe exists and the intended call is read-only, that call may serve as the first access proof.
2. On one auth-shaped failure from a read-only or known-idempotent call, refresh or generate the exact same credential once, then retry the original call once.
3. On one auth-shaped failure from a mutating or non-idempotent call, retry only if the tool guarantees no side effect occurred or a status/idempotency/dedupe check proves retry safety. Otherwise refresh the credential if appropriate, report the uncertain operation state, and stop before repeating the mutation.
4. Never loop on auth. If the safe retry still fails, stop with the exact blocker.
5. Distinguish auth failures from other failures:
   - missing, expired, invalid, or MFA-required token: refresh once for productive work or explicit refresh/request tasks; for check-only tasks, report the status without refreshing;
   - permission, group, policy, or account denial after valid auth: report the access blocker;
   - malformed query, missing index, missing macro, bad request, rate limit, service outage, or tool bug: debug the query/tool/service issue, not auth.

## Known Atlassian SLAuth defaults

Follow tool-specific instructions when available; they override these defaults. For SLAuth-backed tools, use `check_slauth_token_status` when available, then `generate_slauth_token` only when the token is missing, expired, invalid, or MFA-required.

Common token descriptors:

- Splunk logs: audience `splunk.paas-inf.net`, groups `["atlassian-all"]`, `mfa=true`. Use the SLAuth environment required by the token tool or local instructions, normally production for `splunk.paas-inf.net`. Do not map the queried log dataset environment onto the auth environment unless the token tool explicitly says to; for example, “staging logs” usually means Splunk auth plus a staging filter in the query.
- Micros Log Insight load balancer logs: audience `micros-log-insight`, groups omitted, MFA omitted or false.
- Post Office: audience `post-office`, groups omitted, MFA omitted or false.
- Pollinator: audience `pollinator`, groups omitted, MFA omitted or false.
- Snoopr/TCS: audience `snoopr`, groups omitted, MFA omitted or false.
- Mailtracker: audience `mailtracker`, groups omitted, MFA omitted or false.

## Safety

Never print tokens, secrets, cookies, credential files, auth headers, or full command output that contains credential material. Redact credential-like values from summaries. Do not commit generated tokens or local auth artifacts.

## Final

Report briefly:

- Target checked: `<tool or service>`
- Auth: `<already valid | refreshed | MFA requested | blocked>`
- Productive call: `<succeeded | failed with non-auth issue | not run because blocker>`
- Blocker: `<none or exact next requirement>`
