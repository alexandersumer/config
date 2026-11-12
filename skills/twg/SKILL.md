---
name: twg
description: >
  Use TWG whenever Atlassian or company context would help:
  Jira workitems and issues; Confluence pages, whiteboards, databases, PRDs; Bitbucket PRs;
  project or goal status and launch readiness; owners, SMEs,
  approvers, or escalation; personal, org, or leadership work rollups and out-of-office
  catch-ups; dependency maps; code search, repository, or PR discovery; incidents,
  on-call, or reliability; sharing HTML, CSV, PDF, or presentation files
  with coworkers;
  and deep internal research across connected sources, docs, work, and people.
---

# twg

Load listed companions only (`twg skills list`); otherwise use `twg help`.

## TWG Terms

- **Anchor**: the key, URL, person, project, repository, or window that a
  request starts from. Anchor read-only answers on it.
- **Hydrate**: get the full record of a search or query result.

## Fast path for common simple commands

| Need                       | Command                                                             |
| -------------------------- | ------------------------------------------------------------------- |
| Find sources about a topic | `twg rovo search "<topic>" --limit 10`                              |
| Read a Confluence page     | `twg confluence content get <id-or-url> --detail full --format md`  |
| Read a Jira workitem       | `twg jira workitem get <KEY>`                                       |
| Check what code does       | `twg search-code search "<q>"`                                      |
| Read a code file           | `twg search-code file <path> --workspace <workspace> --repo <repo>` |

Use `--site <site>` when supplied or when the anchor URL identifies a site.
For a multi-site request, run each query against its intended site.
Do not assume the saved default matches every anchor.
If the request identifies no site, use the configured default and state the scope.

Read at most 3 Confluence IDs per call.
Use `--search-mode lexical` to search code for an exact string or identifier.
Some answers depend on how a system behaves, such as a default, limit, validation rule, error or config value.
For these, check the code with `search-code` before you rely on docs, then read the file that sets it.

Read search results before you cite them.
For questions these commands cover, do not run `twg help-map` or `twg help describe "<command>"`.
Run independent reads in one shell call, separated by `;`.

For other tasks or unclear syntax, follow **Plan First**.

## Plan First

1. Run `twg help-map`; `*` marks writes. Do not pipe it through `head`.
2. Describe the selected commands in one call (up to 10):
   `twg help describe "<a>" "<b>"`. Reuse those contracts.
3. After contract review, plan internally between `### Plan` and `### End plan`.
   Give one step per call: exact command paths, inputs, flags, dependencies, and unknowns linked to their producer steps.
   Append the plan after all commands in the first execution shell call, with `#` before each line.
   Print only on user request or for clarification or approval.
4. Batch independent reads, separated by `;` (`&&` for dependencies).
   Pipe JSON to `jq` only with `set -o pipefail`; check error fields.

If a command is unclear, run `twg help <terms>`. For more guidance, run
`twg help discover-skills "<intent>"`, then follow `next.loadReference` or
`next.loadSkill`.

## Overview

Load the narrowest companion skill:

- Product work: `../twg-jira/SKILL.md`, `../twg-confluence/SKILL.md`, or
  `../twg-space-creation/SKILL.md`.
- Status: `../twg-status-rollups/SKILL.md`. Load it before engineering.
- Relationships: `../twg-context-discovery/SKILL.md`.
- People: `references/USER-IDENTIFIERS.md`.
- Research: `../twg-agentic-search/SKILL.md`.
- Ownership: `../twg-responsibility-routing/SKILL.md`.
- Code and PRs: `../twg-engineering-work/SKILL.md`.
- Stale work: `../twg-jira-resolve-merged-work/SKILL.md`.
- Incidents: `../twg-operational-health/SKILL.md`.
- Review context: `../twg-code-review/SKILL.md` for missing company context in an existing review.
  Keep the chosen review workflow; do not start a second review or require a report.
- Artifacts: `../twg-artifacts/SKILL.md` for prior Artifact IDs, or to share or update one.


## Invocation And Output

Run `twg <command>`. On shell `command not found`, use `$HOME/.local/bin/twg`
(macOS/Linux) / `$env:LOCALAPPDATA\Programs\twg\bin\twg.exe` (PowerShell), then
tell user to add that directory to PATH. Do not treat auth or command errors as
PATH failures.

Agents: use host-set `TWG_AGENT_DEFAULTS=1` or `TWG_AGENT_DEFAULTS=1 twg <command>`. If assignments are blocked, pass `--mode agent` with explicit output flags. Respect host permissions. `--mode user` overrides the opt-in.

- Answer from inline or already-read compact output if it supports the
  requested facts.
- Quote extracted facts with their sources, not whole payloads.
- For gaps, follow `references/OUTPUT.md`. Keep unavailable evidence unknown.
- On a restricted host, use only `twg` and permitted output files, not
  arbitrary host files, credentials, or other agent runs.
- Use the prompt's timezone and window. Follow the typed route of the
  narrowest companion skill.

## Auth/Setup Guard

For a requested productive TWG task, OAuth maintenance is an in-scope prerequisite:

1. Start with the intended read. If TWG says OAuth refresh is due, expired, or could not be
   persisted, run `twg auth refresh` once, then retry the original read once.
2. If the refresh is blocked from writing `~/.config/twg`, classify that as a sandbox/filesystem
   restriction, not a Confluence/Jira ACL or OAuth denial. Request an approved out-of-sandbox retry
   of the exact `twg auth refresh` command. When reusable command approval is supported, scope it
   to `twg auth refresh`.
3. After an approved refresh succeeds, retry the original TWG read once and continue the task.
   Do not stop at the auth preflight.

Do not redirect `TWG_CONFIG_DIR`, copy credentials into the workspace or `/tmp`, expose credential
files, or ask the user to run the command manually while command approval is available. Give the
manual command only when approval is unavailable or declined.

This exception covers only non-interactive `twg auth refresh`. Do not run login, setup, install,
update, upkeep, forced refresh, connector auth, or other credential commands unless explicitly
requested for setup/auth/repair or required by a specific companion skill.

Bitbucket API tokens and Git SSH credentials are separate from TWG OAuth. Do not
run `twg auth refresh` for a Bitbucket token failure. Report `invalid or expired`
when that is the provider's evidence; do not assert expiration or reuse an earlier
chat's auth failure as current evidence. For Bitbucket operations, try the native
`bb` CLI before browser fallback when it supports the required operation. Native
`bb` OAuth can remain healthy while TWG's API token fails. Use installed `bb` help
for command shapes and the `auth-preflight` skill for that client's bounded
recovery; a failed aggregate `code` profile does not invalidate its healthy targets.
Use other available read-only sources for missing evidence. Browser fallback is
appropriate when explicitly requested, the CLI lacks a required capability, or the
relevant CLI path remains blocked after recovery. If access still blocks a required
fact, name that fact and continue the work the available evidence supports.

## Errors in Sandboxes

For sandboxed `twg bb pipeline get`, `wait`, `grep`, or `tail`, follow `references/OUTPUT.md`.
Log restrictions are not evidence of auth failure.

## On gathering evidence

1. Give the first call all known references.
2. Check current claims against live fields. Date past evidence; report unknown facts.
3. Stop at the first policy denial, or when the same error repeats.
4. Treat a named project or service as exact. Keep its qualifiers and check that you found it. Use a renamed or successor item only if a source says so. If you find no evidence, say so.

## Batch Reads

When a command accepts many inputs, send about twenty IDs per call with
`--agent-fields @compact`. Use query or tree metadata when it is enough.
Tell the user what you omitted. After five repeated calls, check for a batch call.

For page edits, load `../twg-confluence/SKILL.md` and follow its body-format and edit rules.

## Assorted Discovery guidelines

- Use `twg trello search "<query>"` for Trello. It has no workspace scope.
- Run `twg rovo list-apps -o json` only when availability is unknown. Reuse auth.
  Do not auto-login or fabricate "none found".
- `docs query` and `work query` return user activity, not title or content matches.
  For topic text, use `docs search` or `work search`. See `references/ROUTING.md`.
- Pass supported original references to native reads. Avoid separate resolve calls.
  Person handoffs: `references/USER-IDENTIFIERS.md`.
- Command shape guardrails:
  - `work query` uses `--scope me|user`, never `--scope global`.
  - Inferred teams need explicit `--include-inferred`; see `references/inferred-teams.md`.
- Use `search-code` with the explicit repo and host. If the request has no anchor, omit `--app` for available indexed SCM.
  Report gaps in the index.
- Use `assets graph` to traverse object, owner, team, or Jira relationships. See `references/ASSETS_GRAPH.md`.
  Otherwise use `assets search`, `assets query --aql`, or `assets object get`.

## Rules

- Never guess IDs, flags, slugs, ARIs, or write contracts.
- For a write, load the product skill, follow live help, and read the current state.
  Tell the user what the write will change, unless they asked you to run it.
- Do not inspect local files, caches, or schemas, unless the user asks about local state.
