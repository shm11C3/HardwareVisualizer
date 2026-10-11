---
id: LRN-20261011-cache-native-e2e-compilation
status: promoted
cause_status: confirmed
scope: .github/workflows/ci.yml native E2E and bindings checks
trigger: frontend-only CI runs still spend minutes compiling Rust
failure_signature: native E2E has an exact driver cache hit but recompiles app dependencies
root_cause: native smoke builds the app inside its runner while setup-rust cached only registry and installed driver binaries
guardrail: trusted native compiler cache and producer-scoped bindings condition enforced by the CI policy test
canonical_refs: .github/workflows/ci.yml, .github/scripts/test-e2e-ci-policy.mjs, docs/development/e2e-captures.md
verification: node .github/scripts/test-e2e-ci-policy.mjs; inspect native job compilation logs and cache markers
evidence: CI run 38100815964 job 114356249095; frontend-only PR 2387 CI run 38098794478
revalidate_when: native build profile, Tauri asset embedding, cache pruning policy or binding generation inputs change
---

On 2026-10-11, the native job reported an exact `tauri-driver` cache hit with
`targets=false` yet the app's dev-profile build took 8m 34s. The whole native
job took 646s; frontend-only PR #2387 took 807s. The driver install itself took
at most one second. An exact hit is evidence only for the cache's stored scope.

Applying the existing C++-aware compiler cache took 754s on the first fill
(30.3% C++ hits), then 320s on reuse (99% C++ hits). Both ran the real native
smoke successfully. No target cache was restored, so the final workflow keeps
the existing driver/registry cache instead of adding another target entry.
See `docs/development/benchmarks/native-e2e-ci-2026-10-11.json` for raw metrics.

Keep the native scenario on frontend changes: a cached whole application would
contain the old embedded frontend. Binding consumers do not change the generated
contract; gate regeneration on producers and the generated file, while retaining
unknown-input and develop-push safeguards. The CI policy test enforces these
boundaries without duplicating the unchanged paths-filter implementation.
