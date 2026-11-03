# Opt-in Product Analytics

Status: accepted

RFC: [#2014](https://github.com/shm11C3/HardwareVisualizer/issues/2014).
Design: [`docs/design/product-analytics.md`](../design/product-analytics.md).
Hardware depth is handled separately by the Hardware Compatibility Report,
[#2500](https://github.com/shm11C3/HardwareVisualizer/issues/2500).

This record is the public decision and advance notice the RFC asks for. The
merge date of the pull request that added this file is **T0**; the git history
of this file is the record of that date. No stable HardwareVisualizer release
that can send product analytics is published before T0 + 30 days. The first
stable version that carries the client is recorded here when it ships.

## Context

HardwareVisualizer has no evidence of how many installations are active, how
many of one month's new installations are still active later, which screens
and features are used or left switched off, or on which hardware classes and
sensor providers the app runs. The maintainer's verification hardware is
limited, and product decisions are being made without usage or retention data.

DP-01 says the app sends no outbound telemetry, and both READMEs say the app
has no telemetry. Any product analytics therefore has to be an explicit,
publicly reviewed exception rather than an implementation detail.

## Decision

1. **Product Analytics is an opt-in exception to DP-01.** It is off by default
   for new and upgraded installations. No analytics request is made before the
   user grants Analytics Consent, and opting out stops transmission at once,
   clears any queued batch, and deletes the local seed. The application works
   identically whether analytics is on, off, or unreachable.
2. **Consent is a versioned, App-owned Application Preference.** It is
   persisted in `settings.json` through typed Rust IPC. Grant and decline have
   equal visual weight, and closing the consent dialog without choosing counts
   as decline. Widening the data contract requires a new consent version and
   renewed explicit consent from granted installations; a declined
   installation is never asked again, and the Settings switch stays available
   to both. A generic announcement acknowledgement is never reused as consent.
3. **Contract v1 sends aggregates only.** A batch carries the schema and
   consent versions, app version, OS family, CPU architecture, the Consent
   Cohort (the UTC month consent was granted), rotating per-UTC-day and
   per-UTC-month Installation Tokens, enumerated Product Event counters, and
   enumerated true-only daily State Flags (features switched on, navigation
   layout, Performance View, CPU and GPU vendor class, sensor provider
   availability). It never carries names, hostnames, account identifiers, IP
   as a payload field, MAC addresses, serial numbers, device IDs, model names,
   process or window names, paths, logs, stack traces, free-form text, raw
   hardware readings, exact timestamps, locale, or location. Tokens derive
   from a random local seed generated at grant; the seed never leaves the
   machine, and there is no persistent installation identifier. Retention is
   derived per Consent Cohort from monthly counts, never by linking one
   installation across periods. State Flags are never stored in combination.
4. **The boundary is auditable.** The frontend records only typed, enumerated
   Product Events through generated Tauri commands; the App crate enforces
   consent on every event, owns aggregation, batching, transport, timeouts,
   and lifecycle; Core is unchanged and never learns that analytics exists.
   Upload failure never blocks or fails startup, monitoring, shutdown, or any
   product capability.
5. **The service is a separate public repository.** A Cloudflare Worker with
   D1 validates every field against the published contract, rejects unknown
   fields and oversized payloads, deduplicates batches and period tokens,
   deletes token rows days after their period, keeps only tokenless
   aggregates, and publishes aggregate tables only, suppressing dimensions
   below a minimum cohort size. It does not persist IP addresses, headers, or
   request bodies. Because a public desktop client cannot keep a secret, the
   numbers are indicative product evidence, not an authenticated census.
6. **Governance lands before code.** This record, the DP-01 amendment, both
   READMEs, the vocabulary, and the public contract and privacy notice in
   [`docs/user/product-analytics.md`](../user/product-analytics.md) change
   together, and the contract document is what the consent surface links to.

## Alternatives Considered

- **Keep the no-analytics policy.** Rejected for now. It is the strongest
  privacy boundary and remains the default, but it leaves product decisions
  and verification priorities without evidence that a maturing community
  could otherwise supply.
- **Download counts and updater checks.** Rejected. Neither measures active
  installations, retention, or feature use, and both repurpose unrelated
  network behavior.
- **A permanent anonymous installation ID.** Rejected. Monthly cohort
  retention is the product question, and a repeated cohort label answers it
  without a portable identifier.
- **Settings and hardware as batch dimensions.** Rejected. A row that joins
  OS, version, cohort, and several settings is a fingerprint; as daily
  true-only flags stored one at a time they are a census.
- **Model names for hardware breadth.** Rejected. They are on the prohibited
  list, and public suppression would hide the rare configurations that matter.
  The Hardware Compatibility Report (#2500) carries them in the open instead.
- **Raw events from React.** Rejected by the RFC: it weakens consent
  enforcement and auditability.

## Consequences

- HardwareVisualizer gains a third network use beside release updates and
  user-initiated downloads, and the only one that is not user-initiated per
  request. It is gated by consent and documented in both READMEs.
- DP-01 keeps local ownership as the default for hardware data, settings,
  history, and identity, and names this exception.
- The privacy notice must not claim that no personal or personal-related data
  is ever processed: Cloudflare processes connection metadata such as the
  source IP to deliver the request, and rotating tokens are online
  identifiers during their validity period.
- The README's "no telemetry" wording is replaced before any analytics-capable
  binary exists, so the notice period starts from documentation, not from a
  release.
- Hardware depth (chip identifiers, failure detail) stays out of analytics by
  design and is served by the user-posted Hardware Compatibility Report.
