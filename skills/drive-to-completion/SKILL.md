---
name: drive-to-completion
description: Take ownership of unfinished engineering work and keep working until the outcome is delivered and verified. Use when asked to pick up a session, finish an effort, keep going until done, or drive remaining work through review, CI, merge, or acceptance.
---

# Drive to Completion

You own this outcome until it is delivered and verified. The user invoked this skill to stop supervising: when your turn ends, progress stops, and anything left open is theirs to rediscover and restart. End the turn only when every acceptance criterion is verified or its next action belongs to someone else. Otherwise the next action is yours, and you take it now.

One shipped increment, one status poll, or a report of what remains is not an ending. Share progress in passing updates while you keep working; the final message is for the exit gate.

## Authorization

Invoking this skill authorizes the whole delivery path without further confirmation: edit, test, commit, push, open or update a PR where the repository uses them, complete required forms and gates, rerun checks, address review feedback, renew credentials, merge your PR once its required approvals and checks pass, and run in dev or staging when acceptance needs live evidence. Change production only when the request or repository instructions call for it.

Confirm before destroying or overwriting work you did not create, force-pushing, or acting beyond the requested outcome. A boundary on one action does not stop the rest.

Persistence never licenses a shortcut: do not weaken, skip, or bypass a check, fake a result, or act as someone else to clear a gate. The only ways out are a real fix or an honest blocker.

## Loop

1. **Fix the criteria.** Write the outcome as observable acceptance criteria in the host's task list. Treat handoff notes and earlier claims as leads: verify them against the code, spec, and live state of the branch, reviews, hosted CI, and deployed revision. When another session or person is already working part of it, coordinate and take the rest.
2. **Advance every open criterion, highest impact first.** Implement, fix, publish, or unblock it, and update affected specs and runbooks. For a sub-task use the matching skill, such as `execute-plan`, `fix-failures`, `git-publish`, `clear-merge-blockers`, or `e2e-live-check`; its final report is a step result, so return here.
3. **Verify at the boundary.** Exercise the normal entry point and observe what users or callers see. For important regressions, show the same check failing for the intended reason on the baseline and passing with the fix. Verify hosted checks on the delivered commit and recheck after every change. For live acceptance, record the environment, deployed revision, and observed result. Local results do not verify hosted CI or a real environment, and a merge does not verify deployed behavior.
4. **Re-read the criteria** after every result, wait, sub-skill, and context resume. While one is open and not blocked, go to step 2.

## Not blockers

The next action on each of these is yours:

- **Pending, queued, or scheduled work:** a check, pipeline, merge queue, deploy, soak, or bot review. Work independent criteria, then wait inside the turn until it reaches a terminal state: block on a watch command or a sleep-and-check loop sized to the remaining runtime, and repeat it when it times out. Hand the wait to a background job only on a host that resumes you by itself when the job exits. One status poll is not a wait. Never end the turn on "CI is running" or "queued".
- **Failure:** a red check, rejected push, tool error, busy shared resource, or failed attempt. Diagnose it, then fix it, take another route, or retry after a pause. A failure you did not cause is still yours when the outcome needs that check green: fix it, or prove it fails without your change and name its owner.
- **Expired or missing credential.** Renew it with `auth-preflight`, starting any sign-in or MFA prompt, and resume.
- **Choice with a defensible default.** Take the reversible option that best serves the outcome, note it for the report, and continue.
- **Question for the user.** Finish everything that does not depend on the answer, then ask once in the report.
- **Delegated or background work.** Keep working on what does not depend on it, and continue when it reports.
- **Natural pause or sheer size:** a pushed commit, open PR, passing local tests, finished sub-skill, written plan, status summary, long remaining list, or lengthy session.
- **Work you would call a follow-up, next step, or out of scope** when a criterion depends on it.

## Blockers

A criterion is blocked only when its next action belongs to someone else: a human approval or review, access you tried and failed to obtain, a decision with no safe default, an outage that persists across spaced retries, or a check stuck well beyond its normal runtime. Make the request first: request the review, comment on the PR, or message the owner; where you may not contact them, write the exact message for the user to send, using `write-review-request` for a review. "X needs to…" with no request made is still your action. Record what is needed, from whom, what you tried, and the step that resumes the work, then continue with every unblocked criterion.

## Exit gate

Before the final message, check each criterion: `verified` by evidence observed after the last relevant change, or `blocked` as defined above. If any is neither, or the draft report offers to continue, lists next steps you could take yourself, or waits on a check or queue, return to the loop. If the host tracks an active goal, complete it only when this gate passes.

Then report the outcome first, each criterion's state and evidence (what shipped, the delivered revision, the checks observed on it), who must act on each blocker and the step that resumes the work, and the defaults you chose.
