---
name: acpx-peer-review
description: Explicit opt-in external peer-agent review through Axiom's controlled ACPX tool. Use only when the user asks for ACPX, peer-agent comparison, or external agent orchestration. Do not use as fallback for review-deep.
---

# ACPX Peer Review

Use this only when the user explicitly asks for ACPX, external peer-agent comparison, or cross-agent perspective. This is not the default deep-review path and must never run as fallback when Axiom Reviewer is unavailable.

1. **Confirm explicit intent.** Proceed only when the request names ACPX or asks for external peer-agent comparison/orchestration. If the user asked for normal review, use `review-deep` or `review-solo` instead.

2. **Prepare a bounded packet.** Build a focused prompt with pasted diff/artifact/context and the exact question for the peer agent. Do not pass secrets, credentials, or broad unrelated files. Prefer the smallest packet that can produce useful candidate evidence.

3. **Use the controlled tool only.** Call `run_acpx_peer_check` with an allowlisted profile, bounded `timeoutSeconds`, bounded `maxTurns`, and the current `workspaceRoot` when needed. Never run `acpx`, `npx acpx`, `claude`, `codex`, `cursor-agent`, or `toad` through shell commands.

4. **Handle structured outcomes.** If the tool returns `timeout`, `auth_failed`, `unavailable`, `invalid_output`, or `failed`, report that status and do not invent findings. Do not retry endlessly; at most one smaller-packet retry is allowed when the first output is invalid or timed out.

5. **Validate directly in Axiom.** Treat ACPX output as candidate evidence, not authority. Axiom must inspect the relevant code, artifacts, tests, and conventions itself before reporting any issue. Drop candidates that cannot be demonstrated concretely.

6. **Report with provenance.** Label surviving findings as externally suggested and Axiom-validated. If none survive, say no ACPX-suggested finding was validated. Never approve, merge, or delegate validation to another agent.
