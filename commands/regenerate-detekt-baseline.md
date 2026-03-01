---
name: regenerate-detekt-baseline
description: Regenerate detekt baselines for Main and Test source sets
---

First, find and read the README to learn how to run checks locally — do not skip this.

Run the detekt baseline regeneration tasks for both Main and Test source sets. Use `detektBaselineMain` and `detektBaselineTest` — never the vanilla `detektBaseline` task. If the project uses Gradle modules, check for module-specific tasks and run them for every module that has a detekt baseline file.

After regeneration, verify the baseline XML files were updated by checking git status. Then run the full detekt check suite (Main and Test) to confirm the baselines are valid and checks pass cleanly.
