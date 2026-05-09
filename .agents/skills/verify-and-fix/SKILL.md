---
name: verify-and-fix
description: Verify behavior and fix proven defects. Use when the user asks to check, validate, audit, or confirm behavior and repair only real gaps.
---

Verify `scope`, `$ARGUMENTS`, recent changes, relevant artifact, or conversation behavior.

Do not rubber-stamp green tests. Verification means you can prove the behavior from source and exercise it through a real path.

Build expected behavior from primary sources only: request, artifact, public API docs, schemas, types, intended-behavior tests, callers, migrations. If there is no source, say the question is open.

Trace the important contracts through real control, data, and error flow. Name the bug that would matter if the contract were wrong. Then run or add the smallest check that would catch that bug.

Do not use mocks of the system under test as proof. Do not accept tests that only prove implementation details. If existing checks cannot catch the bug class, add the missing check or say the gap remains.

Fix only proven defects or verification gaps. No suppressions, baselines, dependency bumps, skipped/deleted/weakened tests, sleeps, broad catches, or silent fallbacks.

Write the result plainly. Lead with the verdict: verified, fixed, blocked, or not proven. Then give the source proof, executable evidence, changed files, and any remaining gap. No padded audit template. `No issues found` is valid only when important behavior has both source proof and executable evidence.
