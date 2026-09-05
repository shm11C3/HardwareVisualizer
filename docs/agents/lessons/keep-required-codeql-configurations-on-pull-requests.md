---
id: LRN-20260905-keep-required-codeql-configurations
status: promoted
cause_status: confirmed
scope: .github/workflows/codeql.yml and develop code-scanning merge protection
trigger: changing CodeQL language matrices, job conditions, or path filters
failure_signature: an approved pull request with a successful Merge Gate remains blocked by CodeQL reporting 1 configuration not found for /language:rust
root_cause: develop had a Rust CodeQL analysis category that a condition excluded from the pull request, so required code-scanning protection could not compare all base configurations
guardrail: .github/workflows/codeql.yml keeps every configured CodeQL language present on pull requests and explains why Rust must not be path-filtered
canonical_refs: .github/workflows/codeql.yml
verification: on a non-Rust pull request and a Rust-changing pull request, confirm Analyze (rust) and the aggregate CodeQL check succeed; run actionlint and npm run check:agent-guidance
evidence: "Issue #2080; PR #2079 check 101265371495 reproduced the missing Rust category; PR #2076 demonstrated a complete non-Rust analysis; PR #2070 run 33948618970 demonstrated successful Rust-changing analysis"
revalidate_when: the develop ruleset stops requiring CodeQL, or GitHub supports per-category merge policy or safe reuse of base analyses for omitted pull-request categories
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
