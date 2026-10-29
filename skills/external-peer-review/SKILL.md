---
name: external-peer-review
description: Explicit opt-in external peer-agent review through a controlled peer-review tool exposed by the current harness. Use only when the user asks for external peer-agent comparison or external agent orchestration. Do not use as fallback for review-deep.
---

# External Peer Review

Use this only when the user explicitly asks for external peer-agent comparison, external orchestration, or a cross-agent perspective. This is not the default deep-review path and must never run as fallback when the normal deep-review reviewer mechanism is unavailable.

1. **Confirm explicit intent.** Proceed only when the request asks for external peer-agent comparison or orchestration. If the user asked for normal review, use `review-deep` or `review-solo` instead.

2. **Prepare a bounded packet.** Build a focused prompt with pasted diff/artifact/context and the exact question for the peer agent. Do not pass secrets, credentials, or broad unrelated files. Prefer the smallest packet that can produce useful candidate evidence.

3. **Use a controlled harness tool only.** Use the current harness's controlled peer-review facility when one is available, with bounded timeout and turn limits when the tool supports them. Never shell out to arbitrary agent CLIs, unmanaged wrappers, or background orchestration as a substitute. If no controlled tool is available, report that peer review is unavailable in the current harness.

4. **Handle structured outcomes.** If the tool returns timeout, authentication failure, unavailable, invalid output, or failed status, report that status and do not invent findings. Do not retry endlessly; at most one smaller-packet retry is allowed when the first output is invalid or timed out.

5. **Validate directly in this session.** Treat peer output as candidate evidence, not authority. Inspect the relevant code, artifacts, tests, and conventions yourself before reporting any issue. Drop candidates that cannot be demonstrated concretely.

6. **Report with provenance.** Label surviving findings as externally suggested and locally validated. If none survive, say no externally suggested finding was validated. Never approve, merge, or delegate validation to another agent.
