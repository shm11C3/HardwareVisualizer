---
id: LRN-20260905-keep-required-codeql-configurations
status: promoted
cause_status: confirmed
scope: .github/workflows/codeql.yml and develop code-scanning merge protection
trigger: changing CodeQL language matrices, job conditions, or path filters
failure_signature: an approved pull request with a successful Merge Gate remains blocked by CodeQL reporting 1 configuration not found for /language:rust
root_cause: develop had a Rust CodeQL analysis category that a condition excluded from the pull request, so required code-scanning protection could not compare all base configurations
guardrail: .github/workflows/codeql.yml keeps every configured CodeQL category present on pull requests; for same-repository pull requests whose Rust inputs are identical to the analysed base commit it re-uploads that base analysis instead of omitting the category, and it explains why Rust must not be path-filtered
canonical_refs: .github/workflows/codeql.yml
verification: on a non-Rust pull request, confirm the base-analysis step reports reuse, the upload step re-uploads /language:rust, Analyze (rust) and the aggregate CodeQL check succeed, and the develop merge gate passes; on a Rust-changing pull request, confirm the full Rust analysis runs and uploads /language:rust; run actionlint and npm run check:agent-guidance
evidence: "Issue #2080; PR #2079 check 101265371495 reproduced the missing Rust category; PR #2076 demonstrated a complete non-Rust analysis; PR #2070 run 33948618970 demonstrated successful Rust-changing analysis"
revalidate_when: the develop ruleset stops requiring CodeQL, or GitHub supports per-category merge policy for omitted pull-request categories
---

# Keep Required CodeQL Configurations Present On Pull Requests

Required code-scanning protection compares each CodeQL configuration on the
target branch with the pull request analysis. A path filter that omits Rust
from a non-Rust pull request does not mean the Rust configuration is unchanged;
it means the comparison has no pull request result and cannot satisfy the
required policy.

Keep all languages configured on `develop` in every pull request analysis.
Accept the additional Rust analysis time while CodeQL is a required merge gate.
Changing that trade-off requires changing the ruleset policy explicitly, not
silently removing one required configuration from selected pull requests.

Reuse is the supported way to avoid the repeated cost. For a same-repository
pull request, `.github/workflows/codeql.yml` finds the newest `/language:rust`
analysis on the base branch and compares the Rust inputs (`core`, `src-tauri`,
Cargo manifests and lockfile, toolchain pin, `.cargo`, and the workflow itself)
between that analysed commit and the pull request's merge commit. When they are
identical it re-uploads the stored analysis under the pull request's
`/language:rust` category; otherwise, or when no base analysis exists, it runs
the full analysis. A path filter remains the wrong tool: it removes the
category, while reuse keeps it present.
