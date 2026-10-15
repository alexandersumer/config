# Authentication readiness

[Back to README](../README.md)

Use `workctl auth preflight` before a work session, or `workctl auth exec` to check credentials before launching a command.

`workctl auth preflight` force-renews every declared credential target, even when
its cached credential is still valid, and then verifies the provider's normal cache.
Run it before leaving and complete the browser/MFA prompts. Bitbucket uses browser
OAuth login for the selected account because its CLI has no force-refresh command;
TWG uses forced OAuth refresh and SLAuth uses forced issuance. Each target is
attempted once. Failed renewal is reported even when an older token still works.
Use `--reuse-valid` to renew only missing or short-lived credentials, or `--check`
to check access without explicitly renewing.
`workctl auth exec` does that automatically before starting another command:

```sh
workctl auth preflight
workctl auth preflight --profile code --check --json
workctl auth preflight --profile logs --non-interactive
workctl auth exec --profile work -- codex exec 'continue the staging acceptance run'
workctl auth exec --profile code -- bb pr list
```

## Choose a profile

The built-in `work` profile contains Bitbucket OAuth, TWG OAuth, and production
Splunk SLAuth (`splunk.paas-inf.net`, `atlassian-all`, MFA). `code` contains
Bitbucket and TWG; `logs` contains Splunk only. Select the narrowest profile for the
work. These are explicit requirements, not guesses about other running chats.
Splunk issuance also warms the underlying Atlas login, but does not prove access
to every Atlas service or refresh service-signed runtime credentials.

## Credential ownership and noninteractive use

Provider CLIs retain ownership of credential storage, account selection, and
renewal. workctl never copies credentials, changes home/config locations, starts a
background renewal daemon, or stores tokens in its own files. `--check` makes no
explicit refresh/login calls; provider CLIs may still perform their normal
transparent renewal during status checks. `--non-interactive` permits silent
TWG OAuth refresh (and Bitbucket silent refresh with `--reuse-valid`), but never
starts Bitbucket browser login or SLAuth
issuance that could require browser/MFA. Missing TWG refresh authorization reports
an actionable `twg login` requirement: the installed TWG login requires a
controlling terminal for account/site selection. TWG's separate optional Bitbucket
token is reported by `twg doctor`, but does not invalidate a working TWG OAuth
credential; workctl verifies the separate `bb` CLI credential for Bitbucket use.
It does not repair TWG's optional Bitbucket API token or claim `twg bb` is ready.

## Expiry and access guarantees

With `--reuse-valid`, the default refresh threshold is 300 seconds. `--min-valid-for SECONDS` applies to
known expiry, not to providers with unknown expiry. Bitbucket's supported OAuth
status formats report a conservative lifetime; an authenticated, read-only workspace
list also detects revoked tokens before handoff. Named Bitbucket accounts must be
active for this proof, since the API command has no account override; workctl does
not silently switch accounts. Unknown formats fail closed.
TWG doctor proves API connectivity but exposes no credential expiry, so its result
explicitly reports unknown lifetime. SLAuth parses the provider's cached JWT for
requested audience, expiry and not-before claims; Atlas owns token issuance and
validation. SLAuth TTL is capped by policy at one hour or the remaining underlying
login lifetime. Asking for two hours therefore blocks, even after successful
issuance. No preflight can guarantee future access after revocation or future MFA.
Earlier credentials are aged again at handoff in case a later MFA flow took time.
For long commands, use the provider's supported renewal mechanism inside that
command; `auth exec` checks readiness before launch, not throughout execution.

## Deadlines, failures, and concurrency

Status/silent refresh commands have a 30-second deadline (`--timeout`); browser/MFA
commands have a 180-second deadline (`--login-timeout`). Each target gets at most
one refresh sequence. In `--reuse-valid` mode, Bitbucket tries silent renewal
before a browser fallback.
Permission, malformed-output, installation and connectivity failures do not cause
login loops. In `--reuse-valid` mode, inconclusive issuance checks the cache once without
reissuing. Forced renewal requires a successful provider response and a cache check.
Independent targets continue after a failure. Cancellation returns
130, timeout/access failures return 1, and invalid CLI usage returns 2. Auth
subprocesses run in supervised process groups; descendants are cleaned even after
a successful leader exit. Captures use bounded in-memory pipes, not temporary
credential files. Only a narrowly validated Okta activation URL can be forwarded
from provider output; provider diagnostics and credential output are never printed.
Complete the browser opened by Bitbucket if it needs a new login.

Concurrent workctl preflights coordinate through an owned, private regular lock in
the OS temporary directory, wait for a bounded interval, and recheck caches after
acquiring it. The lock does not coordinate unrelated tools or invocations using a
different temporary directory. Unsafe symlink/FIFO/public lock files are refused.
The persistent lock contains no credentials and is not deleted on release.

## JSON and command execution

`auth preflight --json` emits one schema-versioned result with per-target status,
remaining lifetime where known, expiry/check timestamps, actions, and honest
unknown-expiry notes. No provider output is embedded. `auth exec` sends its report
to stderr and replaces itself with the supplied argv, preserving stdin, terminal,
environment, stdout, signals and exact child exit status. It never invokes a shell
unless you explicitly supply one. Failed preflight prevents the child from running.
`--json` is rejected for `auth exec`, keeping child stdout intact. One preflight is
performed per invocation; existing sessions must invoke this wrapper or the command
from their launcher to receive automatic preflight.

## Custom profiles

Custom profiles contain only credential descriptors, never tokens or commands:

```json
{
  "schema_version": 1,
  "profiles": {
    "staging": [
      {"provider": "bitbucket", "account": "default"},
      {"provider": "twg"},
      {"provider": "slauth", "audience": "micros-log-insight", "environment": "prod", "groups": [], "mfa": false}
    ]
  }
}
```

Use `workctl auth preflight --config /path/to/auth.json --profile staging` or pass
the same options to `auth exec`. Config is explicit; workctl does not search home
credential stores. Files must be regular and at most 64 KiB. Schema versions,
unknown fields, empty profiles, duplicate credential descriptors, invalid audiences,
groups and environments are rejected before calling any provider.

## Verification

```sh
# Deterministic CLI/OS contracts with external provider simulators (CI, no secrets):
cargo test --locked --test auth_cli
# Actual installed providers, network, shared caches, and browser/MFA (explicit):
cargo test --locked --test auth_live -- --ignored --nocapture
# Narrow service-specific live proof; this does not prove Splunk MFA:
WORKCTL_AUTH_LIVE_CONFIG=/path/to/auth.json WORKCTL_AUTH_LIVE_PROFILE=staging \
  cargo test --locked --test auth_live real_slauth -- --ignored --nocapture
```

The required Linux/macOS CI lane runs the auth CLI suite alongside the existing
Git contracts. It covers cache reuse, refresh/recheck, stale caches, unknown expiry,
TTL limits, account preservation, exact descriptors, noninteractive/check-only
behavior, partial failure, malformed responses, secret suppression, live activation
prompts, output bounds, unsafe files, concurrency, deadlines, cancellation,
descendant cleanup, and command argv/stdin/exit behavior. These simulator-backed
adapter tests are distinct from real account acceptance. The explicit live lane
fails if a required provider, credential, network or browser/MFA step is unavailable;
it verifies real preflight, cached reuse, wrapped output/exit, and the impossible
SLAuth lifetime case. Ordinary CI skips this account-dependent lane explicitly.
