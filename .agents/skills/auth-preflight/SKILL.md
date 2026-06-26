---
name: auth-preflight
description: Verify access and proactively request browser/MFA auth before using auth-backed tools, MCPs, CLIs, or services, avoiding repeated failed calls and auth loops.
---

Use this skill before operating an auth-backed tool, MCP server, CLI, service API, logs system, deployment tool, SLO tool, mail tool, tenant lookup tool, or similar integration when the work depends on access being valid.

## Goal

Establish the exact required authentication up front, then make the intended tool call once with valid access. Do not discover auth problems by repeatedly failing productive calls.

## Resolve the target

1. Identify the target tool, MCP, service, CLI command, or API from the user's request and current task.
2. Identify the exact audience, environment, group, account, token type, and MFA requirement from tool schema, repo docs, runbooks, CLI help, or known local instructions.
3. Prefer the narrowest access scope that satisfies the target tool. Do not request unrelated audiences, groups, environments, or broad permissions unless the tool's documented auth path requires them.
4. If the target is ambiguous and different auth paths would materially differ, ask one concise question before authenticating.

## Preflight

1. If a status/cache check exists, run it before the first productive call.
2. If no status check exists, use the cheapest safe read-only probe that proves access to the intended target. For mutating workflows, never use the mutation itself as the auth probe.
3. If auth is missing, expired, invalid, or known to require MFA, refresh or generate the exact token before making the productive call.
4. Treat browser/MFA prompts as expected confirmation for the requested work. Do not ask separate permission just to open or complete the MFA flow.
5. If token generation times out or returns an unclear result, check the cache/status once before deciding it failed.

## Execute without looping

1. Run the intended tool call after auth preflight passes.
2. On one auth-shaped failure, refresh the exact same required token once, then retry the original call once.
3. Never loop on auth. If the retry still fails, stop with the exact blocker.
4. Distinguish auth from non-auth failures:
   - missing, expired, invalid, or MFA-required token: refresh once;
   - permission, group, policy, or account denial after valid auth: report the access blocker;
   - malformed query, missing index, missing macro, bad request, rate limit, service outage, or tool bug: debug as a tool/query/service issue, not by refreshing auth again.

## Known Atlassian SLAuth defaults

Follow tool-specific instructions when they are available. For SLAuth-backed tools, use `check_slauth_token_status` when available, then `generate_slauth_token` only when the token is missing, expired, or MFA is required. Common token parameters:

- Splunk: audience `splunk.paas-inf.net`, groups `["atlassian-all"]`, `mfa=true`, environment `production` unless the task explicitly says staging.
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
