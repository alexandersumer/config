Remove fully rolled out feature flag `{feature_flag_key}`. Keep the NEW/enabled behavior, delete OLD/disabled code paths.

Search thoroughly for ALL references: full key, enum constant, short name, and any string variants. Check test directories explicitly as they are often missed.

Remove flag checks and conditionals, keeping only the enabled code path. Delete tests for disabled/old behavior as they are now invalid. Remove the flag definition from enum/config after all references are gone. Clean up unused imports and dead code left behind.

Run the build and test suite. If checks fail, fix missed references or tests expecting old behavior, then re-run until green.
