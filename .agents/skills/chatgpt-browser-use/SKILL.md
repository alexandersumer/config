---
name: chatgpt-browser-use
description: Complete browser tasks and live UI verification in ChatGPT or Codex, including authenticated sites. Use for browser interaction, not ordinary web search. Claude should follow its own browser integration instructions.
---

Complete the requested browser task. Establishing a connection is only the first step.

## Discover and connect

Discover the controls exposed to this chat and follow their current initialization instructions. Read documentation returned by the selected integration before acting. Use documented capabilities rather than remembered runtime names, paths, versions, or APIs. An installed integration may still be unavailable to this chat.

Preserve the requested browser and authenticated session. When the user requires independent control, choose browser-scoped input that leaves the desktop pointer alone. Use dedicated task tabs or a group when supported, preserving existing tabs. Groups organize tabs; they do not isolate account or application state. Never transplant cookies or profiles to establish access.

Make a lightweight supported call, read the target page, and confirm the relevant account. Exercise a harmless control when interaction needs verification, then continue the requested task. Reuse working bindings; refresh discovery when the integration changes or reports a disconnection or API mismatch.

## Act and recover

Choose actions from fresh page state using the integration's documented controls. Recheck the intended resource after navigation or context changes. Carry existing authorization forward while honoring any confirmation required by the tool.

Recover from the observed failure using current integration documentation. Distinguish unavailable controls, connection problems, stale tabs, authentication prompts, and application errors before choosing recovery. Retry only when the documented procedure or changed evidence supports it. Avoid unchanged checks, speculative configuration changes, and private runtime bridges. Request human action only for an evidenced requirement; report the decisive failure and supported next step if blocked.

## Verify and finish

Verify the requested outcome in the live page, including persistence when relevant. Inspect uncertain submissions before retrying. Keep concise evidence and distinguish browser access from proof of the requested workflow. Close task-created tabs unless needed for a deliverable or handoff, preserve existing work, and report any unverified result.
