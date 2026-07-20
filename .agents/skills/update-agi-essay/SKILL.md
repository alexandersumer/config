---
name: update-agi-essay
description: Use when researching, planning, editing, fact-checking, refreshing, or style-auditing the essay "What to Do If You Take AGI Seriously" against current AGI sources while preserving its verification-cost thesis, length discipline, and no-AI-phrasing voice.
---

## Proof policy

Reuse proof only when it is visible, same-scope, after the last relevant edit, and not invalidated by touched files, config, dependencies, fixtures, generated output, runtime state, or environment. Otherwise run the narrowest check that proves the claim, artifact, or behavior; broaden only when risk or policy requires it. Final reports must separate reused proof, new commands, and checks not run.

Apply `scope`, else `$ARGUMENTS`, else infer the requested update mode from the conversation. Use this skill when the user asks to research, plan, update, refresh, edit, verify, or style-audit `src/content/blog/what-to-do-if-you-take-agi-seriously.md`.

Keep this as one coherent editorial workflow. Do not split the job across separate research, planning, editing, or audit skills. Do not commit, push, deploy, or publish; use `$git-publish-to-origin` only after the user asks to publish.

## Core contract

Preserve the essay's central argument: AI progress is fastest where correctness is cheaply verifiable and slowest where it is not. Updates should strengthen that framework across capability, deployment, productivity, alignment, misuse, labor exposure, and individual planning.

Keep the essay roughly the same length. Prefer replacing weaker or stale material over adding new paragraphs. New facts should earn their space by clarifying the argument, correcting the timeline, or improving the evidence base.

The essay must read like the existing author, not like a source roundup. Avoid pasted-on "as of today" commentary except in explicit status paragraphs where dated claims are the point.

## Start state

1. Resolve the site repo first from an explicit path, the active repository, or a locally discoverable `alexandersumer.com` checkout under existing configured workspace roots. Do not assume a personal absolute path or broadly crawl the home directory. Ask for the repo path if no bounded candidate contains `src/content/blog/what-to-do-if-you-take-agi-seriously.md`.
2. Inspect repository context before edits:
   - `git status --short`
   - `wc -w src/content/blog/what-to-do-if-you-take-agi-seriously.md`
   - read the essay before proposing changes
3. Browse for any update beyond pure style or copyediting. Current AGI claims decay quickly, so do not rely on memory for model releases, evaluations, policy, compute, prices, labor data, or lab claims.
4. If the user asks only for planning, research and plan without editing.
5. If the user asks to edit, make the edit after gathering enough context and stating the intended change.

## Source hierarchy

Prefer primary and high-quality sources:

- Independent evals and measurement bodies: METR, Epoch AI, ARC-style evaluators, Apollo Research, peer-reviewed or arXiv papers with clear methods.
- Frontier-lab primary sources: OpenAI system cards and preparedness materials, Anthropic system cards, ASL materials, Economic Index, model release notes.
- Economics and labor: NBER, peer-reviewed economics, official statistics, reputable working papers with data access and methods.
- Policy and governance: official government documents, standards bodies, major court or agency material, official company policy when relevant.
- Infrastructure and financing: company filings, utility or regulatory documents, official announcements, and reputable reporting when primary sources are unavailable.

Classify load-bearing evidence in your notes as one of:

- independent evaluation
- lab claim
- peer-reviewed or formal working paper
- official policy or filing
- economic or labor-market evidence
- reputable reporting
- speculation or forecast

Never present lab claims as independent evidence. Use reporting for disputes, financing, infrastructure, and market reactions, but avoid letting reporting carry technical claims that need primary support.

## Update map

Before editing, identify:

- stale or over-specific claims
- claims that need a date
- claims that need softening
- claims that have gained stronger evidence
- claims that should be removed because they distract from the verification-cost framework
- figures, captions, alt text, and the writing-process note if their claims changed
- places where new evidence can replace older evidence instead of expanding the essay

For each proposed change, know which section it belongs in and what older text it displaces.

## Editing rules

Use integrated replacement, not append-only updates. The best update often deletes a weaker sentence and puts the current fact in its place.

Preserve the essay's voice:

- direct, analytical, plain-spoken
- skeptical without hedging every sentence
- concrete dates for current-state claims
- concrete nouns and verbs
- no grand summaries that sound detached from the author's own thinking

Avoid generic AI phrasing and formulaic contrast:

- no em dashes
- avoid `this is not X, it is Y`
- avoid `not just`, `not only`, `rather than`, and repeated `less X, more Y`
- avoid `the upshot`, `put simply`, `in other words`, `to be clear`, `crucially`, `importantly`, `ultimately`
- avoid `drift`, `lands`, `seam`, `robust`, `surgical`, `bolted on`, `crystal clear`
- avoid consultant filler such as `leverage`, `navigate`, `holistic`, `paradigm`, `pivotal`, `unlock`, `transformative potential`

Do not use visible prose to explain that the edit is integrated. The integration should be evident from the argument.

Treat front matter as deliberate. Do not change `title`, `date`, `description`, or `draft` unless the user asks or the repo convention clearly requires it.

## Style audit

After editing, run these scans against the essay and fix real hits. False positives are allowed only when the word is plainly necessary and not an AI tell.

```bash
rg -n -o "—|drift|drifts|lands|seam|seams|slop|this is not|This is not|not [^.,;:!?]+, but|rather than [^.,;:!?]+, but|not only|not just|The question is|the question is|The upshot|the upshot|That matters|that matters|in other words|put simply|Put simply|live hinge|serious signals|exactly what|moving fastest|not a single cliff|going well|same direction|same underlying|where things stand|Where things stand|to be clear|crucially|importantly|ultimately|robust|Robust|surgical|bolted|crystal|This is a decision|This is less|That is diffusion|That is more mature|That matches|span a wide range|So what" src/content/blog/what-to-do-if-you-take-agi-seriously.md
rg -n -i "delve|tapestry|landscape|realm|underscore|game.?changer|ever.?evolving|pivotal|crucial|vital|holistic|seamless|leverage|navigate|nuanced|complex and|at the end of the day|it is worth noting|worth noting|furthermore|moreover|in conclusion|needless to say|as we all know|unlock|transformative potential|paradigm|plays a crucial role|deep dive|key takeaway|takeaway|crystal|bolted|surgical|robust|slop" src/content/blog/what-to-do-if-you-take-agi-seriously.md
rg -n "not [^.?!]{1,140} but|not [^.?!]{1,140};|less [^.?!]{1,100} more|rather than|starts to look less|isn't [^.?!]{1,100}\\. It's|is not [^.?!]{1,100}\\. It is|This means|That means|The result is|The upshot|In practice|In short|At the same time|Moving forward|Going forward" src/content/blog/what-to-do-if-you-take-agi-seriously.md
```

Then read every edited section in context. The scans catch patterns; they do not replace judgment.

## Verification

For research-only or plan-only work:

- cite sources used
- report stale claims and proposed replacements
- do not edit unless the user asks

For edit work:

- run `wc -w` before and after
- run the style audit
- run `git diff --check`
- run `npm run lint:check -- src/content/blog/what-to-do-if-you-take-agi-seriously.md`
- run `npm run build` and `npm run test` when the user asks for final polish, completion, or publish readiness

Do not claim the essay is complete, current, or clean without fresh verification in the same turn. Do not promise a subjective guarantee about style; report the objective scans and the manual read.

## Final response

Report:

- what changed, by section
- important sources used, with links when browsing was used
- word count before and after
- style-audit result
- checks run and results
- remaining risks, especially current claims likely to decay soon

Keep the final response concise. Do not include a long source dump unless the user asked for research notes.
