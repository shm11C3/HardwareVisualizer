---
id: LRN-20260921-render-untrusted-text-through-github-markdown-api
status: promoted
cause_status: confirmed
scope: .github/scripts/** and workflows that write PR or issue comments containing names, logs, or other text a pull request can influence
trigger: rendering or escaping untrusted text into a GitHub comment, job summary, or issue body, or reviewing a test that claims such text is inert
failure_signature: the CI telemetry comment HTML-escaped a PR-controlled workflow name inside a summary element and its unit test passed, yet GitHub rendered an at-mention in that name as a live user mention
root_cause: GitHub post-processes rendered HTML for mentions, issue references, and bare URLs after Markdown and entity escaping, skipping only text inside code, pre, or anchor elements; a string-level assertion cannot observe that pass
guardrail: untrusted names are rendered only inside a Markdown code span or, within raw HTML, inside a code element; render changes are checked against GitHub's Markdown API with a hostile fixture
canonical_refs: docs/development/ci-telemetry.md, .github/scripts/ci-telemetry/render.mts, .github/scripts/test-ci-telemetry-aggregate.mts
verification: node .github/scripts/test-ci-telemetry-aggregate.mts fails when the code element is removed from the summary; rendering the hostile fixture through gh api markdown yields no user-mention, issue-link, image, or external anchor
evidence: GitHub Markdown API output for a hostile workflow name before and after wrapping it in a code element, recorded while building the CI telemetry aggregator
revalidate_when: GitHub changes which elements its mention and autolink pass skips, or another workflow starts rendering pull-request-controlled text
---

# Render Untrusted Text Through GitHub's Markdown API

Escaping that looks complete at the string level does not prove that a GitHub
comment is inert. GitHub runs a mention, reference, and autolink pass over the
rendered HTML, so entity-escaped text inside raw HTML can still notify users or
link out. That pass skips only `code`, `pre`, and anchor content.

Put text a pull request can influence inside a Markdown code span, or inside a
`code` element when it must live in raw HTML. Prove a rendering claim with the
surface that can show it: `POST /markdown` renders without publishing anything,
so a hostile fixture can be checked for `user-mention`, `issue-link`, images,
and external anchors before a workflow with a write token ever posts it.
