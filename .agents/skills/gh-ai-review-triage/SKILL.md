---
name: gh-ai-review-triage
description: Triage and optionally address AI-generated GitHub PR review feedback from Copilot, CodeRabbit, or similar bots. Use when the user asks to check AI review comments, decide whether comments require action, ignore non-actionable suggestions, fix required/should-fix/worth-fixing feedback, or summarize bot review findings on a pull request.
---

# GitHub AI Review Triage

## Goal

Separate AI review noise from useful feedback. Inspect review threads, classify each comment, fix comments that are required, should be addressed, or are clearly worth addressing, and leave optional/nitpick comments alone unless the user asks to address them.

## Workflow

1. Resolve the target PR.
   - Use a PR number or URL from the user when provided.
   - Otherwise infer the PR from the current branch and repository.
   - Prefer thread-aware data over flat comments when available, because resolution and outdated state matter.

2. Collect review context.
   - Fetch inline review threads with resolved/outdated state.
   - Fetch top-level PR comments and review submissions to catch bot summaries, approvals, and nitpick-only reviews.
   - Note the author for each item. Treat Copilot, CodeRabbit, and similar bot comments as advisory until verified.
   - Extract binding constraints from the user request, explicit maintainer corrections, and accepted canonical decisions. Use them as the classification boundary rather than treating them as advisory inputs equal to bot feedback.

3. Use the best available retrieval method.
   - If GitHub connector tools are available, prefer `_list_pull_request_review_threads` for inline threads and `_fetch_pr_comments` for the merged PR timeline.
   - If the GitHub MCP server is available, use `get_pull_request_comments` and `get_pull_request_reviews` as a fallback; note that flat comments may not fully preserve thread resolution state.
   - If only `gh` is available, use `gh pr view` to resolve the PR, then `gh api graphql` for `reviewThreads` and REST endpoints for `/pulls/{number}/comments`, `/pulls/{number}/reviews`, and `/issues/{number}/comments`.
   - If `gh` auth or network access fails, report the blocker and ask for re-authentication or permission instead of guessing from incomplete data.

4. Classify each comment.
   - `Required`: correctness bug, build failure, clippy/lint warning that reproduces, test failure, security/soundness issue, or behavior that conflicts with the user request. Note: a comment may only be classified `Required` after verification (see Step 5); if verification is not feasible, classify as `Should Fix` with a 'needs verification' tag instead.
   - `Should Fix`: non-blocking but clearly correct feedback that removes misleading code, unrealistic tests, stale comments, fragile behavior, or likely reviewer follow-up.
   - `Worth Fixing`: small low-risk change that improves clarity, test realism, performance in a hot path, user-visible behavior, or future maintainability.
   - `Optional`: nitpick, style preference, docstring/coverage suggestion outside project requirements, or UX improvement with fallback already implemented.
   - `Ignore`: outdated, resolved, duplicate, incorrect after local verification, unrelated to this PR, or conflicts with a binding maintainer constraint without evidence that the decision must be reopened.

5. Verify before editing.
   - Check the proposed edit against the binding constraints before judging only its local technical correctness. A plausible implementation can still be the wrong product behavior or scope.
   - Reproduce claimed CI-impacting issues locally when feasible.
   - Inspect the referenced code, not only the bot wording.
   - If a bot says "warnings-as-errors" or similar, run the relevant check before treating it as required.
   - Only mark Required if the issue is verifiable with reproduction/test evidence or deterministically verifiable from available data; if verification isn't possible, downgrade to Should Fix with a 'needs verification' tag (or Ignore if clearly stale/duplicate).
   - If a comment conflicts with a binding constraint, preserve the constraint and report the conflict. If the comment exposes new correctness, safety, or consistency evidence, return that evidence to the maintainer instead of silently overriding the direction or implementing a compromise that weakens it.

6. Decide action.
   - Treat `Required`, `Should Fix`, and `Worth Fixing` as fix targets by default. Note: fix targets assume verification feasibility has been assessed per Step 5.
   - If the user explicitly says "必須だけ" or "only required", implement only `Required` items and report the skipped `Should Fix` / `Worth Fixing` items.
   - Do not address `Optional` or `Ignore` items unless the user explicitly asks.
   - Do not post replies, resolve threads, or submit reviews on GitHub unless the user explicitly requests that write action.

7. Implement and validate when changes are needed.
   - Keep fixes narrowly scoped to the review comment.
   - Run the smallest relevant tests/checks first, then broader checks if the fix touches shared behavior.
   - Commit/push only when the user asks for that.

## Output Format

When only triaging, summarize in Japanese if the user is using Japanese:

```text
確認しました。修正対象は N 件です。

- Required: ...
- Should Fix: ...
- Worth Fixing: ...
- Optional/Ignore: ...

対応する/しない理由:
...
```

When changes are made, include:

- which comments were addressed
- which comments were intentionally ignored and why
- validation commands and results
- whether GitHub replies/resolution were left untouched
- the sufficiency verdict (`SUFFICIENT` / `INSUFFICIENT` / `UNCERTAIN`) with
  its rationale, anchored on what would concretely break by merging now

## Convergence

Bot reviewers are generators, not gates: they re-review every push and can
always produce another finding, so "respond until silent" never terminates.
The PR #1944-#1958 cycle showed the failure shape — each round's fix became
the next round's finding, and prose grew conditional structure that outweighed
the content it guarded. These rules bound the loop:

1. Split findings by evidence class before responding.
   - Reproducible: anything a deterministic check can verify — code, tests,
     types, CI, and also guidance failures such as validator errors, broken
     links, or invalid frontmatter. Verify by reproducing, and fix what
     reproduces. These converge — defects are finite.
   - Subjective (wording, naming, document or design shape): there is no
     experiment that proves the bot right, so these do not converge. Fix only
     a self-contradiction or a factual error against current code; decline
     the rest with the reasoning stated once.

2. Do not restructure in response to a subjective finding. If such a finding
   implies a different structure — a type model, a document's architecture, a
   module boundary — file an issue or put it to the maintainer; restructuring
   mid-review is how each round manufactures the next round's findings. A
   verified `Required` defect whose necessary fix changes structure is fixed,
   not deferred — the ban is on reshaping to satisfy taste, never on
   repairing a reproduced defect.

3. Watch for the oscillation signature: a finding that targets text or code
   added in response to the previous finding. Verify it like any other first —
   a response can introduce a real regression, and that gets fixed. When it
   does not reproduce, stop forward-fixing; prefer reverting the accumulated
   response-structure to something simpler, or freeze and decline.

4. Stop-loss: the normal exit from review iteration is a `SUFFICIENT`
   verdict (next section); this round limit is the backstop for when rounds
   keep ending without one. After two response rounds on the same PR, stop
   editing for subjective findings — a newly verified `Required` defect is still fixed,
   in any round. Summarize the remainder as declined-with-evidence or filed
   issues and hand the trade-off to the maintainer; resolve the threads only
   under the same explicit authorization Step 6 requires for any GitHub
   write action. An approval obtained by satisfying a generator is not worth
   a document shaped by one.

## Sufficiency Judgment

Finding discovery, validation, and fixing end with an explicit shippability
verdict, not with silence. Processing a round's valid findings means deciding
fix, decline, or file-an-issue for each, with stated reasoning — processing is
not the same as fixing. After processing, judge the change as a whole:

| Verdict | Meaning |
| --- | --- |
| `SUFFICIENT` | the change achieves its purpose; no remaining item justifies blocking the merge |
| `INSUFFICIENT` | a problem affecting correctness, safety, or the acceptance criteria remains |
| `UNCERTAIN` | agent-available evidence cannot settle it; a human decides |

Sufficient does not mean zero findings. It means all of:

- the task's purpose and acceptance criteria are met;
- no credible correctness, security, or regression concern is open;
- the required verification (CI, tests, guidance checks) passes;
- no remaining finding has a rational reason to be fixed in this PR rather
  than declined or filed — additional fixing now costs more than it returns.

Answer these before the verdict:

1. Does the change meet its purpose and acceptance criteria?
2. Is any credible correctness / security / regression concern open?
3. Do CI and the required verifications pass?
4. Does any remaining finding have to land in this PR, rather than an issue?
5. Is a proposed addition actually a fix — or refactoring, taste, or future
   work wearing a finding's clothes?
6. What concretely breaks if this merges now?

Question 6 is the gate that ends loops. "This function could be split
further" sounds reasonable, but if merging now causes no specific problem for
the PR's purpose, correctness, safety, or maintainability, the verdict is
`SUFFICIENT` — however polished the suggestion. On `SUFFICIENT`, end the
iteration and state the verdict with its rationale. On `INSUFFICIENT`, fix
and re-judge. On `UNCERTAIN`, put the open question to the maintainer instead
of iterating further.

## Heuristics

- AI approvals and bot overview comments usually do not require code changes.
- CodeRabbit "Nitpick" sections are optional by default, but promote them to `Worth Fixing` when the change is clearly safe and improves the PR directly.
- "Consider ..." wording is usually optional unless it points to a real bug, misleading test/comment, measurable hot-path issue, or low-risk improvement with clear value.
- A stale or incorrect bot claim should be called out briefly and left unchanged.
- For Rust PRs, local `cargo clippy` success is stronger evidence than an AI claim about unused imports.
