---
name: resolve-conflict
description: Resolve Git merge, rebase, or cherry-pick conflicts while preserving both sides' intent. Use when the repository has unmerged index entries or an in-progress operation with conflicts.
---

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Resolve conflicts while preserving current branch intent and incorporating incoming changes. Do not choose one side wholesale just to remove markers.

Identify the in-progress operation from `MERGE_HEAD`, `REBASE_HEAD` or `rebase-merge/`, or `CHERRY_PICK_HEAD`. During a rebase, "ours" is the upstream being rebased onto and "theirs" is the commit being replayed.

For each conflicted file, read both sides' intent before editing: `git log --merge -p -- <file>` for a merge, the replayed commit for a rebase or cherry-pick, and the common base with `git show :1:<file>`. If the incoming side removed feature flags, dead code, deprecated APIs, or temporary constructs this branch did not introduce, accept the removal and adapt the branch code.

Treat only files that Git reports as unmerged as textual conflicts. Marker text in unrelated fixtures or docs is not a conflict. Edit a non-unmerged file only when targeted proof shows that a merge-caused semantic integration failure requires it. After editing, search the unmerged files for `<<<<<<<`, `=======`, and `>>>>>>>`; marker search is an additional content check, not proof that Git's index is resolved. Stage each intentional resolution or proven integration fix, then require `git ls-files -u` to be empty. Reuse proof when valid; otherwise run the build or the most relevant check if practical.

Leave the resolution staged and do not commit or continue unless the user asked to finish the operation. In that case, complete it with `git commit --no-edit`, `git rebase --continue`, or `git cherry-pick --continue`, and resolve each later rebase stop the same way. Do not report resolution while conflict markers, unmerged index entries, or known merge-caused failures remain.

Final:
- Resolved: `<files>`
- Preserved: `<branch intent>`
- Incorporated: `<incoming change>`
- Index: `<no unmerged entries | exact remaining paths>`
- Operation: `<staged, awaiting commit/continue | completed>`
- Checks: `<command>` -> `<result>`, `reused — <prior proof and why still valid>`, or `not run — <reason>`
