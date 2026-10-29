# Authentication readiness

[workctl guide](workctl.md) · [README](../README.md)

Use `workctl auth preflight` before a work session, or `workctl auth exec` to check credentials before launching a command.

`workctl auth preflight` renews every renewable credential target, even when
its cached credential is still valid, and then verifies the provider's normal cache.
Run it before leaving and complete the browser/MFA prompts. Bitbucket uses browser
OAuth login for the selected account because its CLI has no force-refresh command;
TWG uses forced OAuth refresh and SLAuth uses forced issuance. TWG Bitbucket API tokens
are manually managed: every mode checks them without rotating or collecting secrets.
Kerberos uses one silent `kinit -R` attempt; it never initiates password entry.
Each target is attempted once. Failed renewal is reported even when an older token still works.
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

The built-in `work` profile contains eight independent credential targets:

| Profile | Targets | SLAuth environment, groups, MFA |
| --- | --- | --- |
| `code` | Bitbucket CLI OAuth, TWG OAuth, and TWG Bitbucket token | Not SLAuth |
| `bitbucket` / `twg` / `twg-bitbucket` | One corresponding credential | Not SLAuth |
| `runtime` | Default-cache Kerberos TGT and principal | Not SLAuth |
| `logs` | `splunk.paas-inf.net` | `prod`, `atlassian-all`, MFA |
| `risk` | `risk-mitigation-service` | `prod`, no groups, MFA |
| `checks` | `pollinator` | `prod`, no groups, MFA |
| `config` | `mlp-config` | `prod`, no groups, no MFA requirement |
| `metrics` | `phobos` | `prod`, no groups, no MFA requirement |
| `services` | `risk` + `checks` + `config` + `metrics` | Exact descriptors above |
| `work` (default) | `code` + `logs` + `services` | Exact descriptors above |

Select the narrowest profile for the work. These are explicit requirements, not
guesses about other running chats. For example, use `--profile risk` before a risk
metadata query and `--profile metrics` before collecting Tome objectives. Custom
profiles can combine these descriptors with other audiences or environments.
Use repeatable `--include-profile` to require more than one profile in a single
preflight or wrapped command. Overlapping targets are checked once, in first-seen
order; every requested profile is validated before any provider call. With
`--config`, all profile names resolve in that explicit file.

```sh
workctl auth preflight --profile work --include-profile runtime --reuse-valid
workctl auth exec --profile code --include-profile runtime --reuse-valid -- COMMAND
```

`runtime` checks the current default Kerberos cache, not every available identity.
On macOS it uses Heimdal `klist -t`; on Linux it uses MIT `klist -s`. A separate
`klist` read identifies the principal. Unsupported formats and wrong requested
principals fail closed. Expiry is unknown, so `--min-valid-for` cannot establish a
Kerberos lifetime guarantee. Use a custom `kerberos` descriptor with `principal`
when the runtime requires a particular identity. Named-principal renewal verifies the cache identity before `kinit -R`, including
when the existing TGT is expired. Cache readiness does not prove
Nebulae endpoint access, ASAP signing, or tenant/user-context authorization.

Splunk issuance also warms the underlying Atlas login, but does not populate the
other audience-specific caches or refresh service-signed runtime credentials.

### Coverage evidence and boundaries

The October 21–22, 2026 session review found Risk Mitigation calls for PR gates,
Pollinator calls for release qualification, MLP Config reads for authorization
diagnosis, and Phobos reads for reliability scorecards. The live acceptance session
(`01a15cab-b96f-7742-8caa-12c8dee0973f`) needed explicit MFA for Risk Mitigation and
Pollinator after earlier cached calls. The reliability measurement session
(`01a160ae-28af-7fb3-99af-50b44f95425f`) stopped when the Phobos cache was absent
and renewed that exact audience. These observations determine the added defaults;
they are historical discovery, not proof of current access.

Readiness covers the declared provider credentials. SLAuth checks verify the
requested cache's audience and lifetime; they do not probe each service's resource
permissions. The productive service call must still handle authorization failures.
In particular:

- TWG OAuth and TWG's Bitbucket API token are separate targets. `code` and `work`
  require both, so a valid native `bb` credential cannot hide a broken `twg bb`
  token. `twg doctor` must report both `status: valid` and `ok: true` for the token.
  Missing/expired tokens block with `twg setup bitbucket --force`; scope denial,
  connectivity errors, missing fields and contradictory statuses fail closed.
  Token setup requires a controlling terminal and secret entry, so workctl never
  launches it through its pipe-based supervisor. Enter the token directly in the
  provider's terminal prompt, never chat. Narrow `bitbucket` or `twg` profiles
  remain available for tasks using only that client.
- Micros, Sliver, Spinnaker, browser sessions, and package-registry credentials
  retain their provider-specific checks. Registry reads succeeded in the reviewed
  sessions; an unpublished package was not an auth failure. These are not covered
  merely because another Atlas audience succeeds.
- Kerberos is covered by the explicit `runtime` profile. Local ASAP signing keys,
  service-to-service principals, and tenant/user-context tokens are runtime-specific. A local `User context
  missing` failure and a deployed principal rejection in the reviewed sessions
  were not repairable by renewing workstation OAuth or Splunk. Resolve these in
  the selected runtime; do not issue broad bot tokens, change access policy, or
  refresh unrelated audiences as a generic preflight fix.

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
controlling terminal for account/site selection. TWG Bitbucket is checked separately
in `code`/`work`, including forced-renewal mode. A healthy manual token reports
`valid`, never `refreshed`; token repair never renews unrelated credentials. Other
selected targets still follow the requested renewal mode.

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
Kerberos cache checks and TWG Bitbucket also report unknown expiry explicitly.
Known expiry is preserved throughout verification. Bitbucket's relative lifetime
is measured from the start of its status check, so time spent checking API access
does not extend it. Credentials are aged again at handoff in case a later MFA flow
took time.
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

Within one preflight, Bitbucket, TWG, Atlas, and Kerberos run in at most four provider
workers. Every target's probe, renewal, and verification stays ordered; targets
using the same provider run sequentially, including all SLAuth cache reads and
issuance. Bitbucket browser login and Atlas issuance share one interaction gate,
held from before process launch through descendant cleanup. Silent checks and
refreshes in other providers can continue during browser/MFA waits. Provider
adapters invoke their own CLIs; they do not use another worker to renew access.
Atlas cache concurrency is not assumed, even across different audiences.
Command deadlines start after the interaction gate is acquired, so queued time
does not consume the login budget. Queued work remains cancelable. All workers
finish before the final expiry check, lock release, or `auth exec` launch. Ordinary
provider failures do not cancel independent work; uncertain process cleanup stops
further work and prevents command execution.

Concurrent workctl preflights coordinate through an owned, private regular lock in
the OS temporary directory, wait for a bounded interval, and recheck caches after
acquiring it. `--coordination-timeout SECONDS` sets this wait independently
(1–3600 seconds); when omitted it retains the `--timeout` value. For example,
`workctl auth preflight --reuse-valid --coordination-timeout 600` can wait for a
long-running MFA preflight without lengthening individual status-command deadlines.
The lock does not coordinate unrelated tools or invocations using a
different temporary directory. Unsafe symlink/FIFO/public lock files are refused.
The persistent lock contains no credentials and is not deleted on release.

## Human-readable output

Human output identifies the selected profile and renewal mode on stderr before
checking credentials. Each target is announced before its provider
check; concurrent announcements can arrive out of profile order. Progress lines and
authentication prompts are written intact, and prompts identify the credential
target. Final reports retain profile order. The final report lists aligned statuses,
verified accounts, readable scope details, remaining lifetime or explicitly unknown expiry, and labeled
reasons and recovery actions. It ends with the ready count and elapsed time.
Failed checks say access is not ready rather than implying a verified credential
with unknown expiry. Color follows the Git commands: terminal-only, with
`NO_COLOR`, `CLICOLOR=0`, and `TERM=dumb` respected. Redirected output contains no
terminal escape sequences. JSON omits human progress; provider authentication
prompts still go to stderr when interaction is necessary.

## JSON and command execution

`auth preflight --json` reports `included_profiles` and emits one schema-versioned result with per-target status,
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
      {"provider": "twg_bitbucket"},
      {"provider": "kerberos", "principal": "user@EXAMPLE.COM"},
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
# Check every built-in credential without initiating renewal or MFA:
cargo test --locked --test auth_live real_work_cache -- --ignored --nocapture
```

Read-only live proof for the new adapters is also available:

```sh
cargo test --locked --test auth_live real_twg_bitbucket_reports -- --ignored --nocapture
cargo test --locked --test auth_live real_kerberos_cache -- --ignored --nocapture
```

The TWG test accepts an explicitly blocked missing/expired token as correct
detection; it is not proof of usable Bitbucket access. The Kerberos test requires
a real valid default TGT and proves exact-identity acceptance and wrong-identity
refusal. Neither test renews credentials.

The required Linux/macOS CI lane runs the auth CLI suite alongside the existing
Git contracts. It covers cache reuse, refresh/recheck, stale caches, unknown expiry,
TTL limits, account preservation, exact descriptors, noninteractive/check-only
behavior, partial failure, malformed responses, secret suppression, live activation
prompts, output bounds, unsafe files, provider overlap and serialization, exclusive
interaction, ordered results, separate coordination deadlines, cancellation,
descendant cleanup, and command argv/stdin/exit behavior. These simulator-backed
adapter tests bound independent fixture groups to four while exercising concurrent
providers and invocations inside each group; this avoids treating host startup
contention as token-expiry behavior. The simulator resolves its Python interpreter
once before starting auth deadlines, avoiding repeated developer-tool launcher
work on macOS. These tests are distinct from real account acceptance. The explicit live lane
fails if a required provider, credential, network or browser/MFA step is unavailable;
it verifies real preflight, cached reuse, wrapped output/exit, and the impossible
SLAuth lifetime case. Ordinary CI skips this account-dependent lane explicitly.
