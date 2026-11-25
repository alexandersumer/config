---
name: project
description: Start, resume, or hand off a tracked project whose state lives in a Jira ticket and, where one exists, a spec. Use when asked to start a project, to drive or continue work from a ticket key, to hand off work that outlives the session, or to sweep recent untracked work.
---

# Project

A project is work that needs more than one session, more than one PR, or a rollout. Its state lives in a ticket and, where the design warrants one, a spec, so any agent can resume from the ticket key alone. Everything else stays untracked: create no ticket or spec for questions, reviews, single fixes, or investigations.

Use `twg-jira` for every Jira read and write, and the repository's planning instructions for specs. If Jira is unreachable, continue the engineering work and report the tracking step as not done.

## Write boundary

Invoking this skill authorizes creating, updating, commenting on, and transitioning tickets assigned to me. Ask once before creating an epic. Ask before closing, retitling, reassigning, or re-parenting a ticket that someone else reported or owns.

## Start a project

1. Find the tracker. Use the Jira space named in the request or in the repository instructions. Ask when neither names one.
2. Search that space for an existing epic or ticket that covers the outcome, including unassigned and older ones. Adopt a match instead of creating a duplicate.
3. Propose in one message the epic title, three to six slices that can each ship on their own, and whether a spec is warranted. A spec is warranted when the work fixes hard-to-change decisions such as data models, contracts, boundaries, or a staged rollout. Proceed on confirmation.
4. Create the epic and one ticket per slice, assigned to me. Give each an outcome, a done-when, and links. Leave out progress, PR state, and dates, which go stale.
5. Set each ticket's real status, set the epic's priority explicitly, and apply the parent and labels that comparable tickets in the space use.
6. When a spec exists, record the ticket key in the spec's metadata and link the spec from the ticket.
7. Create the worktree and branch named after the key of the slice being started, then continue the work.

## Resume from a key

1. Read the ticket, its parent epic, its latest handoff comment, and the linked spec's open tasks. Treat them as leads and verify them against the branch, open PRs, and deployed state.
2. Check for another live session on the same key: an open PR, or a branch or worktree named after the key with recent changes. If one exists, say so and take a different slice.
3. Reuse the worktree named after the key, or create it.
4. Move the ticket to its in-progress status and continue the work.

## Hand off

1. If the work has no ticket and will outlive this session, run **Start a project** first.
2. Bring the spec's task list in line with what shipped.
3. Add one comment to the ticket: what is done, what is in flight with its PR and worktree, the next action, and each blocker with the person or approval that clears it. Keep evidence the comment cites in the repository or in the workspace's `.artifacts/` directory.
4. Set the ticket status. Close it only when its done-when is verified.
5. Reply with the resume line: `drive <KEY>`.

## Sweep

On request, list my pull requests merged in the given period whose branch carries no ticket key, and group them by outcome. For each group that meets the project definition, propose adopting an existing ticket or starting a project. Create nothing without confirmation.
