---
name: claude-browser-use
description: Complete browser tasks and live UI verification in Claude, including authenticated sites, through Claude's browser integrations. Use for browser interaction, not ordinary web search or fetching.
---

Complete the requested browser task. A working connection is only the first step.

## Connect

Find the browser tools available in this conversation, loading deferred tools first, and follow their current instructions. Load the tools you need together and read the context or setup output they return. Rely on current tool documentation, not remembered names, versions, or APIs.

Use the browser the user asked for. If they want their signed-in session, use an integration that drives their own browser; a separately launched browser has no session, so don't substitute it silently. Prefer browser-scoped input over desktop pointer control. Work in a dedicated task tab or group, not the user's tabs. Tab groups organize tabs; they don't isolate accounts or site state. Never copy cookies or profiles.

Prove access before continuing: open the target page, read it, confirm the signed-in account, and use a harmless control if interaction matters.

## Act and recover

Act on fresh page state, and recheck the target after navigation or tab changes. Honor confirmations the tool or user requires; don't add approvals the task already grants.

On failure, identify the cause first: missing tools, no connected browser, a stale tab, a sign-in prompt, or an application error. Apply the documented recovery for that cause. Retry only when the documentation says to or the evidence has changed. If the user must install, pair, sign in, or choose a browser, give the exact step and continue independent work.

## Finish

Verify the outcome in the live page, including persistence when relevant. Inspect an uncertain submission before resubmitting. Report evidence briefly, and don't present browser access as proof of the requested workflow. Close tabs you created unless the user needs them.
