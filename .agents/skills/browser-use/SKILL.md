---
name: browser-use
description: Complete browser tasks and live UI verification in ChatGPT, Codex, or Claude, including authenticated sites. Use for browser interaction, not ordinary web search or fetching.
---

Complete the requested browser task. A working connection is only the first step.

## Select the integration

Identify the host running this conversation, not the website being visited. Read only the matching guidance: [ChatGPT and Codex](references/chatgpt-codex.md) or [Claude](references/claude.md). If the host is unclear, use the browser tools exposed to this conversation to establish the supported integration before acting.

Discover available browser controls and follow their current initialization instructions. Read the selected integration's returned documentation before acting. Use documented capabilities rather than remembered tool names, runtime paths, versions, or APIs. An installed integration may still be unavailable to this chat.

## Connect and act

Preserve the requested browser and authenticated session. A separately launched browser may have no session; do not silently substitute it. Prefer browser-scoped input over desktop pointer control, and honor requests for independent control. Use dedicated task tabs or groups when supported, preserving existing tabs. Groups organize tabs; they do not isolate account or application state. Never copy cookies or profiles to establish access.

Read the target page and confirm the relevant account. Exercise a harmless control when interaction needs verification, then continue the task. Act from fresh page state and recheck the intended resource after navigation or context changes. Reuse working bindings. Carry existing authorization forward while honoring confirmations required by the tool or user.

## Recover and verify

Distinguish unavailable controls, connection problems, stale tabs, authentication prompts, and application errors before choosing recovery. Follow current integration documentation; retry only when its procedure or changed evidence supports it. Avoid unchanged checks, speculative configuration changes, and private runtime bridges. Request human action only for an evidenced requirement, giving the supported next step. Continue independent work when possible.

Verify the requested outcome in the live page, including persistence when relevant. Inspect uncertain submissions before retrying. Distinguish browser access from proof of the requested workflow, and report any unverified result. Close task-created tabs unless needed for a deliverable or handoff; preserve existing work.
