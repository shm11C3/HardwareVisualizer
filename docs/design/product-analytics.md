# Opt-in Product Analytics Design

Status: decided under
[ADR 0028](../adr/0028-opt-in-product-analytics.md); not yet implemented.
Design for the RFC in
[#2014](https://github.com/shm11C3/HardwareVisualizer/issues/2014). The
governance slice (ADR, DP-01, READMEs, vocabulary, public contract) is the
advance notice; the client and service slices follow, and no stable release
carries the client before T0 + 30 days.

This document turns the RFC's requirements into HardwareVisualizer ownership,
runtime shape, data contract, and verification. It does not restate the RFC's
problem statement, prohibited-field list, or legal references; those stay in
the issue and move into the public contract when the governance slice lands.

## Product claim

A user who explicitly opts in lets HardwareVisualizer send a small, enumerated,
locally aggregated usage summary to a public Cloudflare service, so the
maintainer can see Active Installation trends, how many of one month's new
installations are still active in later months, which screens and capabilities
are used and which features stay switched off, and on which hardware classes
and sensor providers the app actually runs. Nothing is sent before opt-in,
nothing is sent after opt-out, and the application behaves identically whether
analytics is on, off, or unreachable.

The evidence gap this closes has two sides, and the maintainer named both:
product decisions lack usage and retention data, and verification hardware is
limited. Analytics answers the first side fully and the second side only in
breadth (vendor classes and provider availability, never models). Depth for
the second side, such as which Super I/O chip failed to decode, belongs to a
user-posted Hardware Compatibility Report that is tracked separately and sends
nothing to a server; see
[Relationship to the compatibility report](#relationship-to-the-compatibility-report).

Design review outcome: **ADR required**. DP-01 says the app sends no outbound
telemetry. Product analytics is an explicit, publicly reviewed exception, so the
decision, its limits, and its consequences go into an ADR before any
analytics-capable binary ships.

## Vocabulary

These terms are added to `CONTEXT.md` in the governance slice. "Telemetry" is
already an avoided alias of Hardware Monitoring and stays avoided here too:
telemetry suggests hardware readings leaving the machine, which this feature
never does.

**Product Analytics**:
Aggregated counts of which screens and capabilities an installation used, sent
only after Analytics Consent to a public service owned by the project.
_Avoid_: telemetry, tracking, usage statistics, crash reporting, diagnostics

**Analytics Consent**:
The explicit, versioned Application Preference that enables Product Analytics
for one installation. It is granted or declined with equal weight and is never
implied by another acknowledgement.
_Avoid_: announcement acknowledgement, terms acceptance, privacy setting

**Active Installation**:
One installation of HardwareVisualizer that sent a Product Analytics batch in a
UTC day or month. One person on two machines is two installations; a reset or
reinstall may become a new one.
_Avoid_: active user, DAU, MAU, device, account

**Installation Token**:
A rotating per-UTC-day or per-UTC-month value derived locally from a random
seed, used only to count an installation once per period.
_Avoid_: installation ID, device ID, user ID, fingerprint, seed

**Product Event**:
One enumerated screen view or capability use that increments a local counter.
It carries no payload beyond its name.
_Avoid_: log, event log, metric, telemetry event

**State Flag**:
An enumerated true-or-false fact about an installation on one UTC day, such as
a feature being switched on, a navigation layout being in use, a CPU vendor
class, or a sensor provider being available. It is reported once per day as a
count of one and is never stored together with other flags.
_Avoid_: setting value, property, dimension, fingerprint, hardware profile

**Consent Cohort**:
The UTC month in which an installation granted Analytics Consent. It lets the
service count how many of that month's installations are still active in a
later month without linking any installation across months.
_Avoid_: install date, sign-up date, user cohort, retention ID, installation age

## Chosen approach

### Ownership

| Concern | Owner | Why |
| --- | --- | --- |
| Consent value, consent version, seed, Consent Cohort | App settings service, `productAnalytics` section of `settings.json` | An Application Preference; App-owned because Core never reads it |
| Consent enforcement, counters, State Flag sampling, tokens, batching, transport, lifecycle | App, new `src-tauri/src/analytics/` module and one managed service | RFC section 3; Core stays Tauri-independent and never learns that analytics exists |
| Product Event and State Flag vocabulary | App Rust enums; the event enum is exported through the generated bindings, the flag enum is App-internal | The enums are the allowlist; the frontend cannot invent a name and never sees flags |
| Vendor-class and provider-availability mapping | App analytics module, reading App-side hardware DTOs and the existing provider availability state | Hardware knowledge stays out of Core utilities; Core exposes facts, App classifies them for the wire |
| Recording a Product Event | Frontend app layer (screen views) and feature hooks (capabilities) through one shared hook | Features stay unaware of transport; ADR 0027 layers hold |
| Consent surface | Frontend: one startup dialog in `src/app/` and one Settings section | Same arbitration as the other first-run dialogs |
| Data contract and privacy notice | `docs/user/product-analytics.md` and `.ja.md` | Published by the website; the consent surface links to it |
| Ingestion, deduplication, retention, public aggregates | Separate public repository (Cloudflare Worker + D1) | RFC section 5 |
| Public display | `hardviz-webpage` reads the public aggregate endpoint | No private table or account access |

Core gains nothing. The seed and tokens are App data because they describe the
installation's relationship to the service, not a hardware fact.

### Consent model

`productAnalytics` is one App-owned section so that a grant or a withdrawal is
one atomic write:

```json
"productAnalytics": {
  "consent": "undecided",
  "consentVersion": 0,
  "seed": null,
  "cohortMonth": null,
  "reportedDay": null,
  "reportedMonth": null
}
```

- `consent` is `undecided`, `granted`, or `declined`. The default, for new and
  upgraded installations alike, is `undecided`: the key is absent in every
  existing `settings.json`.
- `consentVersion` is the contract version the user saw when they decided.
  `CURRENT_CONSENT_VERSION` is a constant in the App crate and starts at `1`.
- `seed` is 32 random bytes (`getrandom`, already a Core dependency and added
  to App) as lowercase hex, present only while `consent` is `granted`.
- `cohortMonth` is the UTC month (`YYYY-MM`) of the grant, present only while
  `consent` is `granted`. It is the Consent Cohort every batch carries.
- `reportedDay` is the UTC day whose State Flags were last sealed into a
  batch, and `reportedMonth` the UTC month whose month token was last sealed.
  They make "once per day" and "once per month" survive a restart without any
  per-installation row on the service. Both are `null` unless `granted`.

Transitions, all through the settings service:

| From | Action | Result |
| --- | --- | --- |
| `undecided` or `declined` | grant | `granted`, `consentVersion = CURRENT`, new seed, `cohortMonth` = current UTC month, `reportedDay = reportedMonth = null` |
| `granted` with `consentVersion < CURRENT` | renew | `granted`, `consentVersion = CURRENT`; seed, `cohortMonth`, `reportedDay`, and `reportedMonth` are kept |
| any | decline | `declined`, `consentVersion = CURRENT`, `seed`, `cohortMonth`, `reportedDay`, `reportedMonth` all `null`; runtime aborts any in-flight upload, clears its queue, and stops |
| `granted` with `consentVersion < CURRENT` | app start | treated as not granted: no upload, and the consent dialog asks again (its grant is the renew transition) |
| `declined` with `consentVersion < CURRENT` | app start | stays declined and is not asked again |

A declined installation is never re-prompted. A contract expansion widens what
a granted user shares, so they must agree again; it does not change what a
declined user shares, which is nothing. The Settings section stays available
for both. This follows DP-06: the explicit choice survives until the user
changes it.

The seed never reaches the frontend. The wire DTO carries only
`consent`, `consentVersion`, and `currentConsentVersion` (the same shape as
`currentUiAnnouncementVersion`: wire metadata, not a persisted value). The
service's unknown-key preservation and per-field recovery cover the new
section like every other field; an unparseable section recovers to the
default, which is `undecided`, never `granted`.

Re-granting after a decline generates a fresh seed and a fresh Consent Cohort,
so the installation counts as a new Active Installation in a new cohort. That
is the intended limit of the identity. A renewed grant after a contract
version bump keeps the seed and the cohort: the installation did not change,
only what it shares.

### Consent surface

**Startup dialog.** One more entry in the `StartupDialogs` arbitration, lowest
priority: it shows only when no Close to Tray, External Component Guidance,
database conversion, or NSIS notice flow is pending, and only when the
effective state is not granted-at-current-version and not declined. Its two
buttons have the same visual weight and no default emphasis. The description
names in plain words what is sent (screen and capability counts, which
features are switched on, CPU and GPU vendor class, whether a sensor provider
works, app version, OS family, CPU architecture, the month consent was given,
rotating per-day and per-month tokens) and what is never sent (hardware
readings, device model names, serial numbers, persistent device or user
identifiers, error text), and links to the contract,
the privacy notice, the client source directory, and the service repository.
Links open in the system browser through the existing `openURL` path.

Closing the dialog without choosing counts as decline. The alternative, leaving
the installation `undecided` and asking again next launch, is the nagging that
RFC section 2 rules out, and the privacy-preserving outcome of "I did not
answer" is no data.

**Settings section.** A new "Product analytics" block in the Advanced section,
beside Elevated Startup Mode: a switch bound to grant/decline, a one-line state
("Sharing aggregate usage since contract v1" / "Not sharing"), and the same
four links. Write failures surface through `ActionFailure` under the control,
like every other settings write.

A `granted` installation whose `consentVersion` is behind sees the dialog
again on the next start and, in Settings, the switch off with a note that the
contract changed. It does not see a diff of the contract inside the app; the
linked document carries a changelog per version.

### Contract v1: Product Events and State Flags

Two Rust enums are the only allowlist. Every leaf variant is a unit variant,
and a non-leaf variant carries nothing but a closed enum: a Product Event or
State Flag has no properties, text, number, or timestamp anywhere in its
shape.

Product Events are counted; State Flags are sampled. The difference matters
for what a reader can learn: an event count says how often something was done,
a flag says how many installations were in a state on a given day.

```text
ScreenView(Screen)
  Screen = Dashboard | Performance | SystemSpecifications | Usage
         | CpuDetail | Insights | Settings
Capability(Capability)
  Capability = HardwareReportExported
             | TrayWidgetOpened
             | ExternalComponentSetupStarted
             | InsightOpened(Insight)
    Insight = Hardware | Cooling | Gpu | Process | Snapshot
```

`Screen` mirrors `SelectedDisplayType`; `Insight` mirrors `InsightType`. The
wire name is `screen_view.<screen>` or `capability.<name>` in snake case, so a
reader of the public contract and a reader of the enum see the same list.

Counting rules:

- A screen view counts once per navigation to that screen, in the main
  window, while the window is visible. Re-renders, prefetch on hover, and
  restoring the window to the screen it already showed do not count.
- A capability counts when the user starts the action, not when it succeeds.
  Success and failure are not distinguished because the contract allows no
  outcome dimension.

State Flags (App-internal enum, never callable from the frontend):

```text
FeatureOn(Feature)
  Feature = HardwareArchive | StorageHealth | TrayWidget | CloseToTray
          | ElevatedStartupMode | AmbientSensor | BurnInShift | BackgroundImage
Navigation(NavigationLayout)
  NavigationLayout = Grouped | Classic
PerformanceView(View)
  View = Panels | Compact | Monitor
CpuVendor(CpuVendor)
  CpuVendor = Intel | Amd | Apple | Other
GpuVendor(GpuVendor)
  GpuVendor = Nvidia | Amd | Intel | Apple | Other
ProviderAvailable(Provider)
  Provider = PawnIoCpu | SuperIo
```

Wire names are `feature_on.<feature>`, `navigation.<layout>`,
`view.performance.<view>`, `hardware.cpu_vendor.<vendor>`,
`hardware.gpu_vendor.<vendor>`, and `provider_available.<provider>`.

Flag rules:

- Flags are sampled by the App service when a batch is sealed, from the
  settings service and the App-side hardware DTOs and provider availability
  state. Core is not changed; App classifies facts Core already exposes.
- A flag is reported at most once per installation per UTC day, and the
  client is the only enforcer: flags go into the first batch sealed for a UTC
  day, and sealing it writes `reportedDay` to `settings.json` before the batch
  is queued. A restart in the same day reads the persisted day and seals no
  flags. If that batch is later dropped, the day's flags are lost rather than
  re-sent; an undercount is accepted, an inflation is not.
- Only `true` is reported. An absent flag means "not on", "not that layout",
  or "not available", which the service reads as the complement of the day's
  Active Installation count. Each `GpuVendor` present on the machine is its
  own flag, so a machine with an integrated and a discrete GPU reports two.
- `Other` absorbs every vendor the mapping does not name. A raw vendor string
  never reaches the wire.
- **Flags are never stored in combination.** The service adds each flag to
  its per-flag daily total by OS family at ingest and keeps no
  per-installation flag row at all, so nothing persisted joins two flags, or
  a flag with an app version, for one installation. The request body is the
  only place an installation's flags sit side by side, and it is neither
  logged nor stored. A cross such as "Archive off and Tray Widget on" is not
  answerable, by design; the question v1 answers is which features are
  unused, not which users combine them.
- `ProviderAvailable` reports that a provider was attempted and answered, the
  same fact External Component Guidance already derives. It says nothing about
  which chip or which failure; that depth belongs to the compatibility report.

### Local aggregation and Installation Tokens

The App service holds, behind one mutex:

- counters keyed by `(utc_day, event_name)`, where `utc_day` is the system
  clock's UTC date at the moment of the event;
- a bounded queue of unsent batches (at most 8; the oldest is dropped);
- the current consent snapshot.

Tokens are derived from the seed with HMAC-SHA256 (`hmac` and `sha2` are
already Core dependencies and are added to App):

```text
day_token   = hex(HMAC(seed, "hardviz-analytics/v1/day/"   + "2026-11-03"))[..32]
month_token = hex(HMAC(seed, "hardviz-analytics/v1/month/" + "2026-11"))[..32]
```

The two labels make the tokens independent: holding every day token of a month
reveals nothing about the month token or about any other day, and nothing
about the seed. The seed is never transmitted and never logged.

That is a statement about derivation, not about what travels together. The
day token is in every batch, so within a day the service sees one token
repeatedly, which is what "once per day" means. The month token is sealed
into one batch per month (the first one sealed after `reportedMonth` changes),
so it travels beside exactly one day token in transit; the service counts both
and does not persist the pair. The notice says exactly this, and no more.

Clock skew is tolerated by the service, not the client: the Worker accepts
`day` within two days of its own UTC date and rejects the rest. A machine with
a badly wrong clock is simply not counted.

### Batches and cadence

A batch is built from everything accumulated since the previous batch and is
sealed: it gets a random batch id (UUID v4) and its counters never change
afterwards. On failure the sealed batch stays at the head of the queue and is
retried unchanged, with the same id, at the next flush; new events accumulate
into the next batch. This is what makes the batch id a deduplication key: the
service may have stored a batch whose response was lost, and a retry with the
same id and the same counters is idempotent. Merging new counters into a failed
batch would need a new id and would double count the stored one.

Flush points:

1. Sixty seconds after the service starts with consent granted, so the upload
   never competes with startup collection and the first window paint.
2. Every six hours while the process runs.
3. Immediately after consent is granted from the dialog or Settings, so the
   maintainer sees a new Active Installation the same day.

No flush runs at shutdown. The queue is in memory, so counters that are not
yet flushed when the process exits are lost. This under-counts capability use
in short sessions and is accepted, and so is the smaller bias on Active
Installations: a session that ends before the first flush sends nothing, so
an installation that only ever runs for under a minute on a given day is not
counted for that day. The RFC explicitly declines a durable retry queue until
measured need.

Every batch carries today's day token and the Consent Cohort, whether or not
any counter is non-zero; a batch with no counters is still an Active
Installation heartbeat. The first batch sealed in a UTC month also carries the
month token, and the first batch sealed in a UTC day also carries the State
Flags; `reportedMonth` and `reportedDay` are written when those batches are
sealed. A sealed batch that is retried keeps whatever it carried.

Payload (contract v1):

```json
{
  "schemaVersion": 1,
  "consentVersion": 1,
  "batchId": "0f2d1a7c-....",
  "appVersion": "1.12.0",
  "osFamily": "windows",
  "cpuArch": "x86_64",
  "cohortMonth": "2026-11",
  "installation": {
    "day": "2026-11-03",
    "dayToken": "…32 hex…",
    "month": "2026-11",
    "monthToken": "…32 hex…"
  },
  "flags": [
    "feature_on.hardware_archive",
    "navigation.grouped",
    "view.performance.panels",
    "hardware.cpu_vendor.amd",
    "hardware.gpu_vendor.nvidia",
    "provider_available.pawnio_cpu"
  ],
  "counters": [
    { "day": "2026-11-03", "name": "screen_view.performance", "count": 12 }
  ]
}
```

`installation.monthToken` and `flags` are present only in the first batch
sealed for that month and that day; other batches omit them. `flags`
describes `installation.day` only; a sealed batch that is retried on a later
day still carries the day it was sealed for, so its flags land on that day.
`cohortMonth` is never later than `installation.month`.

`appVersion` is `CARGO_PKG_VERSION` as built; `osFamily` is
`windows | macos | linux` and `cpuArch` is `x86_64 | aarch64`, both from
`std::env::consts` mapped through an App enum so an unexpected target fails at
compile time rather than leaking a new string. No field is free-form; the
Worker validates every one against the same enums and rejects unknown keys.

### Transport

- Fixed HTTPS endpoint, a constant in the App crate. There is no setting,
  environment variable, or remote switch that changes it in a release build.
- `reqwest` async client with a 10 second total timeout, the existing
  `HardwareVisualizer/<version>` user agent, and the `ring` provider installed
  the way `external_component_setup::download_client` does it (reqwest is
  compiled with `rustls-no-provider`; the updater installs its own provider
  only when used, so this boundary installs one too).
- No retry inside a flush. A network error, timeout, `429`, or `5xx` is
  logged at debug level without the payload, and the sealed batch waits for
  the next flush point. Any other `4xx` is terminal: the Worker rejected the
  batch's shape or its day, nothing in a retry changes that, so the batch is
  dropped and the queue advances. A batch whose `installation.day` is already
  outside the Worker's two-day window when a flush starts is dropped without
  being sent, for the same reason; this is what happens after a long suspend
  or outage.
- Debug builds never upload: the transport is a no-op that logs the sealed
  batch, mirroring `fetch_update`, which skips the updater in debug builds.
  Development data would otherwise pollute the dataset, and the contract
  between client and service is tested through shared example payloads, not a
  live call.
- The transport is a trait with the reqwest implementation and a recording
  test double, so the Rust tests that prove "zero requests before opt-in and
  after opt-out" count calls on the double rather than inspecting sockets.

Because a public desktop client cannot hold a secret, the endpoint accepts
unauthenticated batches. Edge rate limiting and validation bound abuse; the
resulting numbers are indicative product evidence, and the contract says so.

### Lifecycle integration

The service is a managed Tauri state created in `run()` after settings load,
with its flush loop as a tokio task registered in `WorkersState` so
`terminate_all` aborts it under the same `shutting_down` rule as every other
worker. Aborting an in-flight request is the shutdown path; shutdown never
waits for an upload. The command boundary:

- `record_product_event(event: ProductEvent)`: increments a counter if
  consent is granted at the current version; otherwise returns `Ok(())` and
  does nothing. It never fails the caller and never reports whether analytics
  is on.
- `set_product_analytics_consent(decision: Granted | Declined)`: the order
  of persistence and runtime change depends on the direction, so that either
  step failing leaves the installation sending nothing. A decline is
  effective in the runtime first: before any persistence operation can
  yield, the analytics service atomically invalidates flush admission by
  advancing a consent generation under the service lock, aborts any
  in-flight request the same way shutdown does, then clears counters and the
  queue. Only then does the settings service persist the transition. If that
  write fails, the runtime stays declined for the rest of the process, the
  command returns the error like any other settings write, and the Settings
  surface reflects the persisted state so the user can retry. A grant or
  renew persists first and reloads the runtime snapshot second, so nothing is
  ever sent for a consent that is not on disk. A flush seals its batch and
  reads the generation under the same
  lock, and re-checks that generation immediately before starting transport;
  if it changed, the flush ends without a request. A flush that already
  completed its request re-checks the generation again before touching
  service state, so a stale completion cannot repopulate a cleared queue or
  the reported day and month. Bytes already delivered cannot be recalled: a
  request that completed before the abort is counted by the service for its
  period, and the notice says so. Grant and renew schedule the immediate
  flush.

Commands stay thin: enforcement lives in the service, and the backend checks
consent on every event even though the frontend hook also checks it.

### Frontend shape

- `src/hooks/analytics/useProductEvent.ts` (shared hooks layer) returns a
  `record(event)` function. It reads the consent state from the settings atom
  and skips the IPC call unless consent is granted at the current version;
  this is a cost optimization, not the enforcement point. Failures go to
  `console.error` only; a failed analytics call is never a panel state.
- Screen views are recorded in the app layer where `SelectedDisplayType`
  changes, so no feature imports analytics for navigation.
- Capability events are recorded by the owning feature hook (Hardware Report
  export, Tray Widget open, External Component Setup start, Insight tab
  change) through the shared hook.
- `src/app/ProductAnalyticsConsentDialog.tsx` joins `StartupDialogs`;
  `src/features/settings/components/advanced/ProductAnalyticsSection.tsx`
  joins Advanced settings. Strings go into `en.json`, `ja.json`, and
  `ru.json`.
- The E2E mock layer gains a `recordProductEvent` and
  `setProductAnalyticsConsent` mock so the web E2E suite can assert that no
  event call happens before consent in the mocked flow.

### Service contract (owned by the separate repository)

The Worker repository owns its own design; this section fixes only what the
client and the public contract depend on.

- `POST /v1/batches`: body limit 8 KiB, at most 64 counters and 64 flags,
  strict JSON schema, unknown fields rejected, enums checked against the
  published list, `day` within two days of the Worker's UTC date,
  `cohortMonth` not after `installation.month` and not before the service
  went live. Responses are `202` accepted, `400` invalid, `413` too large,
  `429` rate limited. No request body or header is logged.
- Deduplication: `batch_id` is kept for 7 days; a repeated id is acknowledged
  with `202` and not counted again. `(period_kind, period, token)` rows are
  unique, so a repeated token in a period does not inflate counts. Flags need
  no service-side deduplication: the client sends them once per day, and the
  service adds them to per-flag totals at ingest. There is no per-installation
  flag row and no table that pairs a day token with a month token.
- Retention: day-token rows are deleted three days after their day;
  month-token rows three days after their month ends. Finalized Active
  Installation counts per
  `(period, cohort_month, os_family, cpu_arch, app_version)`, flag totals per
  `(day, name, os_family)`, and event counters per `(day, name, os_family)`
  are kept as aggregate time series without tokens.
- Retention of installations is derived, not stored: month `M` of cohort `C`
  divided by month `C` of cohort `C`. The service never links an installation
  from one month to the next; the cohort is a group label the client repeats.
- Edge rate limiting keys on the connection, not on anything persisted in D1.
- `GET /v1/public/active-installations`, `GET /v1/public/retention`,
  `GET /v1/public/feature-usage`, and `GET /v1/public/state-flags` return
  aggregate rows only. Any dimension breakdown whose cohort is below `k = 20`
  is suppressed; period totals are published regardless of size, because a
  total of "fewer than 20 installations" identifies nobody. Raw events,
  tokens, batch ids, request metadata, flag combinations, and rare
  combinations are never exposed.
- The repository ships the JSON Schema for contract v1 and example payloads;
  HardwareVisualizer's client tests serialize the same examples. Keeping the
  two in step is a manual review step named in both repositories' checklists.

The existing public `hardwarevisualizer-download-dashboard` repository (Hono on
Cloudflare Workers with D1 and cron triggers) is the stack precedent, not the
home: it ingests GitHub API data on the project's behalf, while this service
ingests data that users chose to send. Keeping them separate keeps the
"what does the app send" question answerable by one repository.

### Governance and release gate

The governance slice is documentation only and lands first:

- ADR 0028 "Opt-in Product Analytics": the DP-01 exception, consent and
  versioning rules, what is and is not sent, retention and publication rules,
  and the T0 + 30 day release gate. Status `accepted` on merge; its merge date
  is T0 and is written into the ADR by the merging pull request.
- DP-01 keeps "local ownership by default" and adds that Product Analytics is
  an opt-in exception governed by the ADR, with hardware data, settings,
  history, and identity still never leaving the machine.
- Both READMEs replace "has no telemetry" with the opt-in wording, together.
- `CONTEXT.md` gains the terms above.
- `docs/user/product-analytics.md` and `.ja.md` hold the privacy notice and
  contract v1: every allowed field and its purpose, the full Product Event and
  State Flag lists, the Consent Cohort and how retention is derived from it,
  the prohibited list from the RFC, retention, publication rules including the
  no-combination rule, Cloudflare's role and processing locations, consent
  withdrawal, renewed-consent behavior, and a changelog per contract version.
  The website publishes `docs/user/`.
- The RFC amendment that adds the Consent Cohort, State Flags, the flag
  deduplication and no-combination rules, and the #2500 pointer is posted on
  #2014; ADR 0028 records the amended contract.

There is no build-time feature flag. The shipped binary is the reviewed binary;
the gate is a release checklist line in the ADR: no release tag that contains
the client slices before T0 + 30 days. The project publishes no pre-releases,
so the rule has no second case. The ADR records the first version that
carries analytics once it ships.

## Relationship to the compatibility report

The maintainer's second evidence gap, limited verification hardware, needs
depth that this contract forbids: chip model names, which decode step failed,
and what the readings looked like. That depth is a Hardware Compatibility
Report: a user-initiated export, built on the existing Hardware Report, that
the user reads in full and posts to a public GitHub Discussion or issue
template. It sends nothing to a server, needs no privacy notice beyond "you
are posting this publicly", and feeds a public compatibility table under
`docs/user/` with wanted-hardware requests and contributor credit. It is
tracked in [#2500](https://github.com/shm11C3/HardwareVisualizer/issues/2500).

The two mechanisms split cleanly: analytics says how many installations have
a working or non-working Super I/O provider, the compatibility report says
which chips those are. Neither needs the other to ship.

## Why this shape

- **Cohort month instead of an installation ID.** Retention is a question
  about a group of installations, and a group label the client repeats answers
  it. A persistent ID would answer it too, but would also answer questions the
  RFC rules out. The cohort label is coarse (one of a few dozen months), it is
  reset when the seed is reset, and it never lets two batches be matched to
  one installation.
- **State Flags as tokenless daily counts.** "Which features are unused" needs
  a daily census, not a per-installation profile. Reporting only `true`
  flags once per day, with the client remembering the day it reported, lets
  the service store per-flag totals and nothing else: the census is exact and
  the combination is never persisted anywhere.
- **Month token once per month.** Sending it in every batch would pair it
  with every day token of the month in transit. Sending it once pairs it with
  one, and the same persisted "last reported" mechanism the flags need makes
  that free.
- **App-only, Core untouched.** Analytics has no hardware fact in it, and the
  RFC requires Core to stay unchanged. Placing even the token derivation in
  Core would give Core a network-facing concept it has no reason to know.
- **One settings section, not three fields.** Grant and withdrawal change
  consent, version, and seed together; a partial write would be either a seed
  without consent or consent without a seed. One section is one atomic merge
  into `settings.json`.
- **Sealed batches.** The simplest shape in which a client-generated batch id
  is actually a deduplication key. A mutable pending batch would need the
  service to reconcile overlapping counters, which the RFC's "exact
  deduplication" does not allow.
- **No shutdown flush.** The RFC rules out blocking shutdown and rules out a
  durable queue. A best-effort shutdown upload would either block or be so
  short it rarely completes; it adds a code path for a benefit the heartbeat
  already provides.
- **Decline on dismiss.** Equal-weight choices plus "no answer is no data"
  satisfy both the RFC's consent rule and its no-nagging rule without a
  third `undecided` state that persists past the first dialog.

## Alternatives considered

- **Persist the seed in its own file in the app data directory.** Rejected.
  It would add a second write path with its own failure modes, and it would
  separate the seed from the consent it depends on, so a failed second write
  could leave consent granted with no seed or a seed with no consent.
- **Gate the client behind a Cargo feature until T0 + 30 days.** Rejected.
  The reviewed artifact and the shipped artifact would differ, and the gate is
  a date, which a checklist expresses directly.
- **Record Product Events only from React with a consent check in the hook.**
  Rejected by the RFC: the backend must enforce consent. The hook's check is
  kept only to avoid IPC calls that would be discarded.
- **Send one event per interaction with a timestamp.** Rejected by the RFC's
  prohibited list; it would also turn the service into an event store with
  per-installation journeys.
- **A permanent anonymous installation ID for retention.** Rejected, as the
  RFC already did. Monthly cohort retention is the product question, and the
  Consent Cohort answers it without a portable identifier.
- **Settings and hardware as batch dimensions instead of flags.** Rejected.
  Dimensions on the Active Installation row would be stored jointly with OS,
  architecture, version, and cohort, and that joint row is a fingerprint. As
  flags they are stored one at a time.
- **Model names or chip identifiers for hardware breadth.** Rejected by the
  RFC's prohibited list, and public suppression at `k = 20` would hide the
  rare configurations that matter most. The compatibility report carries
  them in the open instead.
- **Service-side flag deduplication through the day token.** Rejected. It
  needs `(day, day_token, name)` rows, and those rows hold one installation's
  flag combination for as long as they live, which contradicts the notice.
  Per-flag tokens would avoid the join but add derivations for a problem the
  client's persisted `reportedDay` removes outright.
- **Weekly retention.** Deferred. It needs a week token and a cohort week;
  monthly answers the current question, and adding week granularity later is
  a contract expansion with renewed consent.
- **Reuse the download dashboard repository for ingestion.** Rejected; see
  the service section.

## Verification

Each claim maps to the narrowest evidence that proves it (DP-09):

| Claim | Evidence |
| --- | --- |
| Default is disabled for new and upgraded installations | Settings service test: absent key and legacy files parse to `undecided`; invalid section recovers to `undecided` |
| Unknown keys survive | Existing `serialize_preserving_unknown_settings` test extended with the new section, plus a Core-owned key beside it |
| No request before opt-in, none after opt-out | Rust test with the recording transport double: events recorded while undecided/declined produce zero transport calls; grant then decline leaves the queue empty and the next flush makes no call; a decline during an in-flight request aborts it and the double sees no completion; a flush that read `granted` before a decline and reaches transport admission after it makes no call (the test holds the flush between sealing and admission with an injected gate, then declines); a completion that lands after a decline leaves the queue and `reportedDay`/`reportedMonth` untouched; with a settings-write double that blocks, a decline followed by a flush attempting admission makes no call while the write is still pending, and a failing write leaves the runtime declined. Plus one manual network trace on a release build, recorded in the PR |
| Sealed-batch retry policy | Unit test: network error, timeout, `429`, and `5xx` keep the head batch; `400` and `413` drop it and the next batch is sent; a batch older than the Worker window is dropped unsent |
| Month token once per month | Unit test: with a persisted `reportedMonth`, the second batch of a month omits the month token; a new month seals it again |
| Version bump requires renewed consent | Service test: `granted` at version 1 with `CURRENT = 2` records nothing and the DTO reports the dialog condition |
| Opt-out clears queue, seed, and cohort | Service test asserts queue length 0, `seed = null`, and `cohortMonth = null` after decline; re-grant yields a different seed and the current month; a version-renewal grant keeps both |
| Only allowlisted events | Compile-time: the command takes the enum; a frontend test checks the generated binding type has no string variant |
| Flags carry no raw strings | Unit test: every vendor string fixture maps to a named variant or `Other`; the wire serializer has no string path |
| Flags once per day | Unit test: with a persisted `reportedDay`, a simulated restart in the same UTC day seals a batch without flags; the next UTC day seals them again |
| Flags never combined | Service-repository test: the schema has no table with a per-installation flag row, and ingesting a batch leaves only per-flag totals |
| Retention derived without linking | Service-repository test: two cohorts across three months produce the expected matrix from tokens that differ every month |
| Sealed batch retry is idempotent | Unit test: failed flush retries the same id and counters; new events land in a second batch |
| UTC day bucketing | Unit test with an injected clock crossing midnight |
| Upload failure never affects the app | Transport double returns errors and timeouts; service state and command results are unchanged; shutdown test aborts an in-flight flush within the worker deadline |
| Dialog arbitration and equal-weight buttons | Component test for the dialog; web E2E for ordering after the existing dialogs; rendered screenshot at desktop and compact viewports in the PR |
| Service validation, dedup, retention, suppression | Tests in the service repository |
| Website shows aggregates without private access | The website build uses only the public endpoint; checked in its repository |

## Slices

1. **Governance** (`docs/`): ADR 0028, DP-01, READMEs, `CONTEXT.md`,
   `docs/user/product-analytics(.ja).md`, `docs/README.md` index. Its merge
   date is T0.
2. **Service repository**: Worker, D1 schema, flag deduplication, cohort
   retention derivation, rollup and retention jobs, public endpoints, JSON
   Schema and examples, tests. Public before any client release.
3. **Consent persistence** (`feat/`, Rust): `productAnalytics` section with
   seed, Consent Cohort, and the reported day/month, service transitions
   including renew, DTO, commands, regenerated bindings, tests.
4. **Analytics runtime** (`feat/`, Rust): event and flag enums, counters,
   flag sampling and vendor mapping, tokens, sealed batches, transport trait
   and reqwest implementation, worker registration, tests with the transport
   double.
5. **Consent surface and event recording** (`feat/`, frontend): dialog,
   Settings section, shared hook, screen-view and capability call sites,
   language files, E2E mocks and tests, screenshots.
6. **Website** (`hardviz-webpage`): aggregate display from the public
   endpoint.
7. **Release**: first stable tag no earlier than T0 + 30 days; record the
   version in ADR 0028.

Slices 3 to 5 are stacked and each is independently reviewable; slice 2 can
proceed in parallel with slice 1 once the contract text is agreed.

## Open questions

- The initial `Capability` and `Feature` lists. The four capabilities above
  are the ones with a clear user-started action today; the eight features are
  the Application Preferences a user switches on or off. Anything else is a
  guess until v1 data exists.
- Whether `ProviderAvailable` should also cover Linux `smartctl` and
  `intel_gpu_top`. They are optional components too, but their availability
  is per-command rather than probed once, so the sampled fact is less clear.
- Endpoint hostname and service repository name. `analytics.hardviz.com` and
  `hardviz-analytics` are placeholders.
- Whether `k = 20` is the final suppression threshold, and whether month
  totals below `k` should be shown as "fewer than 20" rather than the number.
- Whether a declined installation should be told, in Settings, when the
  contract version changes. The current answer is no: nothing about their
  situation changed.
