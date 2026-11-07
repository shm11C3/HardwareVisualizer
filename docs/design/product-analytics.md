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
maintainer can see Active Installation trends, how many of the installations
that opted in during one month still run in later months (Consent Cohort
Retention, which is not installation retention; see
[Metric definitions](#metric-definitions)), which screens and capabilities
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
One opted-in installation of HardwareVisualizer whose Product Analytics batch
for a UTC day or month was accepted by the service. It means the app ran with
Analytics Consent long enough to upload, not that a person used it, and it
counts only installations that opted in. One person on two machines is two
installations; a reset, reinstall, or re-grant after a decline may become a
new one. See [Metric definitions](#metric-definitions).
_Avoid_: active user, user count, DAU, MAU, device, account, install base

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

**Consent Cohort Retention**:
For a Consent Cohort and a later month, the month's Active Installations that
carry that cohort divided by the cohort's Active Installations in its own
month. It starts at consent, not at installation, and it ends at anything
that stops an upload, including opt-out, so it is neither user retention nor
installation retention.
_Avoid_: retention rate, user retention, install retention, churn rate,
continuation rate

**Flag Census**:
The number of accepted batches that carried a State Flag set for a UTC day
and OS family. The client sends at most one such batch per installation per
day, but the service counts batches and cannot verify that. It is the only
denominator for State Flag rates.
_Avoid_: active installations (as a flag denominator), flag total, sample size

## Chosen approach

### Ownership

| Concern | Owner | Why |
| --- | --- | --- |
| Consent value, consent version, seed, Consent Cohort | App settings service, `productAnalytics` section of `settings.json` | An Application Preference; App-owned because Core never reads it |
| Consent enforcement, counters, State Flag sampling, tokens, batching, transport, lifecycle | App, new `src-tauri/src/analytics/` module and one managed service | RFC section 3; Core stays Tauri-independent and never learns that analytics exists |
| Product Event and State Flag vocabulary | App Rust enums; the event enum is exported through the generated bindings, the flag enum is App-internal | The enums are the allowlist; the frontend cannot invent a name and never sees flags |
| Vendor-class and provider-availability mapping | App analytics module, reading App-side hardware DTOs and the optional sensor provider outcomes Core publishes in `MetricsSnapshot` | Hardware knowledge stays out of Core utilities; Core exposes facts, App classifies them for the wire |
| Optional sensor provider attempt outcome | Core collector, one field of `MetricsSnapshot` | A hardware fact Core already computes before ACPI fallback and guidance suppression; see [Provider outcome source](#provider-outcome-source) |
| Recording a Product Event | Frontend app layer (screen views) and feature hooks (capabilities) through one shared hook | Features stay unaware of transport; ADR 0027 layers hold |
| Consent surface | Frontend: one startup dialog in `src/app/` and one Settings section | Same arbitration as the other first-run dialogs |
| Data contract and privacy notice | `docs/user/product-analytics.md` and `.ja.md` | Published by the website; the consent surface links to it |
| Ingestion, deduplication, retention, public aggregates | Separate public repository (Cloudflare Worker + D1) | RFC section 5 |
| Public display | `hardviz-webpage` reads the public aggregate endpoint | No private table or account access |

Core gains no analytics concept. Its one change is to publish, in the snapshot
it already emits, whether each optional sensor provider's attempt answered: a
hardware fact with value beyond analytics, which Core computes today and then
discards. The seed and tokens are App data because they describe the
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
effective state is not granted-at-current-version and not declined. The NSIS
notice does not report a pending state today: it requests the slot only
after `getBundleType()` resolves. So the consent slice makes it report one
through `onPendingChange`, as the guidance and conversion dialogs do, and
"free" means no requested and no pending flow. Whether
it opens is decided once per launch, when the arbitration first becomes
free: it opens then unless the main window is on Settings, in which case it
is offered again at the next start and nothing is recorded. That moment is
normally at startup, but it follows the existing arbitration, so a slow
first guidance lookup can move it after the user has started using the
window; this design adds no startup-phase or completion signal of its own,
consistent with ADR 0029. The nonmodal corner notices outside the
arbitration (`SensorServiceNotice`, `ElevationUnavailableNotice`) do not
gate consent: they stay visible behind the dialog and after it closes, so
nothing is lost, and gating on them would defer consent indefinitely for an
installation whose notice reappears every launch. Its two
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

**Settings section.** A new "Product analytics" block at the end of the
Advanced section, after the startup, sensor service, and component setup
controls, because it is neither a sensor nor a startup control: a switch bound
to grant/decline, a one-line state
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
  settings service, the App-side hardware DTOs, and the latest optional
  sensor provider outcomes in `MetricsSnapshot`. App classifies facts Core
  exposes; Core never sees a flag. The one exception to the settings source
  is `view.performance.*`, which comes from the UI-local Tauri Store key
  `performanceLayoutPreset` and follows the frontend's
  `normalizePerformanceView`; an absent key is `panels`, and a store that
  cannot be read leaves the flag sources unsettled.
- A flag is reported at most once per installation per UTC day, and the
  client is the only enforcer: flags go into the first batch sealed for a UTC
  day whose flag sources are settled, and only when that day is later than
  `reportedDay`, so a clock set backwards never re-sends a day. Sealing it
  writes `reportedDay` to `settings.json` before the batch is queued; if that
  write fails, the batch is sealed without flags instead. A restart in the
  same day reads the persisted day and seals no
  flags. If that batch is later dropped, the day's flags are lost rather than
  re-sent; an undercount is accepted, an inflation is not.
- A flag set is complete or absent, never partial. The client seals flags
  only when every source has a determined value: settings are loaded, the
  CPU and GPU DTOs exist, and no provider that exists on this OS family is
  `pending` in the latest snapshot. If any
  source is still undetermined when a batch is sealed, that batch carries no
  `flags` field, `reportedDay` is not written, and the next sealed batch of
  the day tries again. A session that ends before its sources settle sends no
  flag set for the day; it is missing from the Flag Census rather than
  counted as "off".
- Only `true` is reported, inside a complete set. Within a complete set an
  absent `FeatureOn` flag means "not on" and an absent `ProviderAvailable`
  flag means "attempted and did not answer". `Navigation`, `PerformanceView`,
  and `CpuVendor` are exclusive groups: a complete set carries exactly one
  variant of each. `GpuVendor` is a multi-valued group: each vendor present
  on the machine is its own flag, so a machine with an integrated and a
  discrete GPU reports two, and a machine with no enumerated GPU reports
  none. `ProviderAvailable` is sampled only on the OS family where the
  provider exists, which is Windows for both v1 providers.
- The `flags` field is present exactly when the batch carries the day's
  complete set, which is never empty because the exclusive groups always
  contribute one variant each. The service counts its presence as
  one Flag Census entry; absent flags are read against that census, never
  against Active Installations. See
  [State Flag rates](#state-flag-rates).
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
- `ProviderAvailable` reports that a provider was attempted and answered.
  It says nothing about which chip or which failure; that depth belongs to
  the compatibility report.

### Provider outcome source

Since [ADR 0029](../adr/0029-windows-sensor-service.md), every PawnIO-backed
reading on Windows comes through the Windows sensor service; the app reaches
PawnIO no other way. The facts the App receives cannot express a provider's
attempt outcome:

- `MetricsSnapshot` carries no availability for CPU package temperature or
  motherboard sensors, and the CPU temperature availability is merged with
  the ACPI fallback before it leaves the platform layer.
- `motherboard_fan_support` is `Unsupported` for a recognized ITE chip whose
  temperatures are read, so it is not an answer signal.
- `cpu_power_support` is `Unknown` both while the service connection is in
  progress and after the service is found missing.
- External Component Guidance candidates exist only for some failures, are
  cleared when ACPI supplies a CPU temperature, and are removed by the App
  once the user acknowledges a key, so their absence means nothing.

Without a determined outcome, a Windows installation without the service
would never settle its flag set and would drop out of the Flag Census,
biasing every Windows rate toward installations that have the service.

Core therefore publishes one more field in `MetricsSnapshot`: for each
optional sensor provider (PawnIO CPU package temperature and Super I/O
motherboard sensors, both Windows only), whether its latest attempt is
`pending`, `answered`, or `notAnswered`. Core computes it where it already
knows it, before the ACPI merge and before guidance suppression. `pending`
covers a service connection in progress and a provider not sampled yet;
`answered` means the attempt returned a reading; `notAnswered` covers a
missing or incompatible service, a missing driver or module, and an
unsupported chip alike. The Rust shape is settled in that slice with its
tests.

The App maps it: provider sources are settled when no provider is `pending`;
`provider_available.pawnio_cpu` is set when CPU package temperature is
`answered`, and `provider_available.superio` when motherboard sensors are
`answered`.

No flag for the sensor service itself is added. The service is the only
PawnIO path, so `ProviderAvailable` already reports whether the service,
PawnIO, and the chip work together, and `feature_on.elevated_startup_mode`
reports how many installations still start elevated, which is what the
migration question needs. Telling "service missing" from "chip unsupported"
would need either a service status read at seal time, which ADR 0029 keeps
out of automatic paths, or a reason dimension the contract does not allow.

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
month token, and the first batch sealed in a UTC day with settled flag sources
also carries the State Flags; `reportedMonth` and `reportedDay` are written
when those batches are
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

`installation.monthToken` is present only in the first batch sealed for that
month, and `flags` only in the batch that carries the day's complete flag set
(normally the first batch of the day); other batches omit them. `flags`
describes `installation.day` only; a sealed batch that is retried on a later
day still carries the day it was sealed for, so its flags land on that day.
`cohortMonth` is never later than `installation.month`.

`appVersion` is `CARGO_PKG_VERSION` as built; `osFamily` is
`windows | macos | linux` and `cpuArch` is `x86_64 | aarch64`, both from
`std::env::consts` mapped through an App enum so an unexpected target fails at
compile time rather than leaking a new string. No field is free-form; the
Worker validates every one against the same enums and rejects unknown keys.

### Transport

- Fixed HTTPS endpoint `https://analytics.hardviz.com`, a constant in the App
  crate. There is no setting, environment variable, or remote switch that
  changes it in a release build.
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
- `set_product_analytics_consent(decision: Granted | Declined)`: the whole
  command, runtime change and settings write together, runs under one
  consent lock held by the analytics service, so two transitions never
  interleave: a grant that arrives during a decline waits until the decline
  has finished, then runs in full. After a transition that completes
  successfully, the persisted decision and the decision that controls flush
  admission are the same one, and the last successful transition wins. A
  transition whose settings write fails is not such a transition: it fails
  closed for the current process, as described below, and leaves the
  persisted decision unchanged. The flush loop and `record_product_event`
  do not take the consent lock; they read the generation and snapshot under
  the shorter service lock that guards counters and the queue.

  Within a command, the order of persistence and runtime change depends on
  the direction, so that either step failing leaves the installation sending
  nothing. A decline is effective in the runtime first: before any
  persistence operation can yield, the analytics service atomically
  invalidates flush admission by advancing a consent generation under the
  service lock, aborts any in-flight request the same way shutdown does, then
  clears counters and the queue. Only then does the settings service persist
  the transition. If that write fails, flush admission stays closed for the
  rest of the process, the persisted decision stays `granted`, the command
  returns the error like any other settings write, and the Settings surface
  reflects the persisted state so the user can retry. A grant or renew
  persists first and reloads the runtime snapshot second, so nothing is ever
  sent for a consent that is not on disk.

  A flush seals its batch and reads the generation under the service lock,
  and re-checks that generation immediately before starting transport; if it
  changed, the flush ends without a request. A flush that already completed
  its request re-checks the generation again before touching service state,
  so a stale completion cannot repopulate a cleared queue or the reported day
  and month. Bytes already delivered cannot be recalled: a request that
  completed before the abort is counted by the service for its period, and
  the notice says so. Grant and renew schedule the immediate flush.

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

The Worker repository, `hardviz-product-analytics`, owns its own design; this
section fixes only what the client and the public contract depend on. The
name follows the Product Analytics term, so a reader checking what the app
sends finds the same word as ADR 0028 and the consent surface, and it is not
mistaken for website traffic analytics beside `hardviz-webpage`.

- `POST /v1/batches`: body limit 8 KiB, at most 64 counters and 64 flags,
  strict JSON schema, unknown fields rejected, enums checked against the
  published list, `day` within two days of the Worker's UTC date,
  `cohortMonth` not after `installation.month` and not before the service
  went live. A `flags` array must hold exactly one variant of each exclusive
  group, and `provider_available.*` is valid only with
  `osFamily = windows`. Every counter `count` is between 1 and a
  published per-batch maximum. Responses are `202` accepted, `400` invalid,
  `413` too large, `429` rate limited. No request body or header is logged.
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
  `(day, name, os_family)`, the Flag Census per `(day, os_family)`, and event
  counters per `(day, name, os_family)` are kept as aggregate time series
  without tokens. An Active Installation's dimensions are those of its first
  accepted batch in the period, so the breakdown by any dimension sums to the
  period total.
- Consent Cohort Retention is derived, not stored: month `M` of cohort `C`
  divided by month `C` of cohort `C`. The service never links an installation
  from one month to the next; the cohort is a group label the client repeats.
  See [Metric definitions](#metric-definitions).
- Quality counters per day, aggregate and tokenless: accepted batches,
  repeated batch ids, and rejected requests by reason (invalid, too large,
  rate limited, outside the day window). They are published beside the
  metrics so a reader can see how much traffic was refused and why.
- A period can be marked suspect by the maintainer after an anomaly review.
  The mark and its reason are published with the period's rows; the rows are
  neither deleted nor rewritten, because no per-installation data exists to
  clean them with. See [Data quality](#data-quality).
- Edge rate limiting keys on the connection, not on anything persisted in D1.
- `GET /v1/public/active-installations`, `GET /v1/public/retention`,
  `GET /v1/public/feature-usage`, and `GET /v1/public/state-flags` return
  aggregate rows only. Any dimension breakdown whose cohort is below `k = 20`
  is suppressed; period totals are published regardless of size, because a
  total of "fewer than 20 installations" identifies nobody. A retention row
  is published only when its cohort's own-month size is at least `k`, and a
  State Flag rate only when its Flag Census is at least `k`; below that the
  ratio is too noisy to read and is withheld, not shown as zero. Raw events,
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

### Metric definitions

These definitions are part of the public contract. The service computes each
metric exactly this way, and every published table names it with the term
below and carries its "is not" line.

| Metric | Definition | Is not |
| --- | --- | --- |
| Daily Active Installations, day `d` | Distinct day tokens accepted for `d` | Daily active users, or all installations that ran on `d` |
| Monthly Active Installations, month `m` | Distinct month tokens accepted for `m` | Monthly active users, or the sum or maximum of the month's daily counts |
| Consent Cohort size, cohort `C` | Monthly Active Installations in month `C` that carry cohort `C` | Installations, downloads, or new users in month `C` |
| Consent Cohort Retention, cohort `C` at month `M > C` | Monthly Active Installations in `M` that carry `C`, divided by the size of `C` | Installation retention or user retention |

**What "active" measures.** An installation is active in a period when a
process with Analytics Consent at the current version stayed up until its
first flush, reached the service, and had a batch accepted.
HardwareVisualizer can start with the operating system and stay in the tray,
so an active installation may not have shown a window that day. Active
Installations measure that the app ran. How much it was used is answered only
by Product Event totals, which are not per installation; dividing them by
Active Installations gives an indicative mean that is biased low, because
counters not yet flushed at shutdown are lost.

**Why the count is not a user count.** The definition counts some machines
more than once and some not at all, and neither can be corrected without the
persistent identifier ADR 0028 rejects:

- More than one per person or machine: one person on several machines; a
  settings reset or reinstall that loses the seed; a decline followed by a
  re-grant, which creates a new seed, so the machine counts twice in that day
  and month and moves to the new cohort.
- Fewer than actually ran: only opted-in installations; sessions that end
  before the first flush; networks that block the service; batches dropped by
  the transport rules; a settings file restored or cloned onto a second
  machine, which shares the seed so both machines count once. A dropped first
  batch of a month loses that month's token, so a monthly count is not
  guaranteed to be at least the month's largest daily count.
- Never scaled: Active Installations are never divided by downloads or used
  to estimate the installed base. The opted-in population is self-selected
  and its share of all installations is unknown, so no "opt-in rate" is
  derived either.

**Why Consent Cohort Retention is not installation retention.**

- Entry is consent, not installation. When analytics first ships, existing
  installations that opt in join that month's cohort, so the launch cohort
  consists mostly of installations that had already stayed. Its curve is
  expected to sit above later cohorts' and is labeled the launch cohort in
  every published table. Cohort comparisons use later cohorts.
- Exit is anything that stops accepted uploads: uninstalling, not running in
  the month, opting out, a settings reset, a re-grant (which moves the
  installation to a new cohort), a blocked network, or a replaced machine.
- A contract version change pauses every installation until it renews, and
  installations that never renew leave their cohort. Months with a version
  change are labeled in the retention table.
- The own month is partial, from the grant day; later months are whole.
- The ratio is not clamped. A cohort whose own-month batch was lost can be
  counted later, so a value above 1 is possible in a small cohort and is
  read as a data-quality signal, not as growth.

**Reading rules.** The maintainer uses these numbers as product evidence
under three rules: compare trends and ratios inside the opted-in population,
never absolute counts against other sources; compare cohorts with cohorts,
excluding the launch cohort and noting version-change months; and treat a
change as evidence only when it holds across several periods none of which
is marked suspect.

### State Flag rates

A State Flag rate is computed per `(day, os_family)` against the Flag Census
of that day and OS family, and only from the per-flag totals the service
stores.

| Group | Rate | Complement |
| --- | --- | --- |
| `FeatureOn` (independent) | Flag total / census | Off rate is `1 - rate` |
| `Navigation`, `PerformanceView`, `CpuVendor` (exclusive) | Variant total / census; the shares sum to 1 | None; every census entry has exactly one variant |
| `GpuVendor` (multi-valued) | Installations with at least one GPU of that vendor / census | `1 - rate` is "no GPU of that vendor"; the shares can sum to more than 1, and no "no GPU" share is derived |
| `ProviderAvailable` (independent, Windows only) | Flag total / Windows census | `1 - rate` is "attempted and did not answer"; no rate exists for other OS families |

Active Installations are never the denominator. A day's count includes
installations whose complete flag set never arrived: a session that ended
before its sources settled, a dropped flag batch, or a failed `reportedDay`
write. Dividing by it would read each of those as "off" or "unavailable".
It also carries dimensions that flag totals do not have.

Further rules:

- A rate over several days is the sum of flag totals over the sum of census
  entries. It is a share of installation-days and is labeled as such, never
  as a share of installations, because flags are not deduplicated across
  days.
- No rate exists for a flag crossed with another flag, an app version, or a
  cohort; those totals are not stored.
- Census coverage, the Flag Census divided by Daily Active Installations for
  the same day and OS family, is published beside the rates. Low coverage
  means short sessions are under-represented in the rates. Coverage above 1
  is a data-quality signal.
- A rate whose census is below `k` is withheld.

### Data quality

Every published number is indicative. The design accepts that the dataset is
incomplete, duplicated in known ways, and open to fabrication, and names each
case instead of correcting for it silently.

| Case | Source | Effect | Handling |
| --- | --- | --- | --- |
| Missing | Opt-in only; sessions shorter than the first flush; blocked or offline networks; queue overflow; terminal `4xx`; batches outside the day window; clocks skewed beyond it; counters lost at shutdown; flags or month token lost with their batch | Undercount; short sessions and restricted networks are under-represented | Not corrected. Census coverage and rejection counters are published; reading rules apply |
| Repeated batch | A lost response retried with the same id | None | Batch ids are kept 7 days and the day window is 2 days, so every retry the Worker accepts falls inside deduplication |
| Repeated token | Several batches in one period | None | Token rows are unique per period |
| Repeated flags | A restart, or a clock set backwards | None while the client rule holds. If it fails, the census and that installation's flag totals grow together, so the installation is over-weighted in that day's rates | The client seals flags only for a day later than `reportedDay`; the service cannot detect a repeat |
| Split installation | Re-grant, settings reset, reinstall | One machine counts twice in that period and appears to leave its cohort | Accepted; undetectable without a persistent identifier |
| Merged installation | A settings file restored or cloned onto another machine | Two machines count once | Accepted |
| Fabricated batch | The endpoint is unauthenticated; anyone can post schema-valid batches with random tokens, versions, and flags | Inflates any count, rate, or cohort | Bounded, not prevented: edge rate limiting; strict schema, exclusive-group, OS, and count checks reject malformed batches; rejection counters show refused volume; an anomaly review can mark a period suspect |
| Modified client or fork | A build that keeps the endpoint | Indistinguishable from the official client | Accepted |

Fabrication cannot be removed after the fact. Telling a fabricated batch
from a real one would need per-installation history, which is the persistent
identifier ADR 0028 rejects, and a public desktop client cannot hold a
secret that would prove it is genuine. The privacy guarantee and the
inability to clean the data are the same property, and the published pages
say so.

The service repository defines the anomaly review: which signals open one
(for example a jump in one dimension's daily count, a spike in rejected
requests, census coverage above 1, or retention above 1), the thresholds,
which are set after the first weeks of data, and how a suspect mark is
recorded and published.

### Governance and release gate

The governance slice is documentation only and lands first:

- ADR 0028 "Opt-in Product Analytics": the DP-01 exception, consent and
  versioning rules, what is and is not sent, retention and publication rules,
  and the T0 + 30 day release gate. Status `accepted` on merge; its merge date
  is T0. It landed in
  [#2501](https://github.com/shm11C3/HardwareVisualizer/pull/2501) on
  2026-11-03, so no release carrying the client is published before
  2026-12-03.
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
- **App-owned analytics, one Core fact.** Analytics has no hardware fact in
  it, and the RFC requires core hardware collection to stay unchanged.
  Placing even the token derivation in Core would give Core a network-facing
  concept it has no reason to know. The provider outcome is the one Core
  change because it is a hardware fact, Core already computes it, and only
  Core can read it before the ACPI merge and guidance suppression erase it.
  Publishing it changes what Core reports, not how it collects.
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
- **A fresh-install marker on the Consent Cohort.** Rejected for v1. It would
  separate new installations from existing ones that opt in later, but it is
  an installation-age attribute stored jointly with the cohort, and consent
  timing would still confound it. Labeling the launch cohort and comparing
  later cohorts answers the retention question without it.
- **Active Installations as the State Flag denominator.** Rejected. Any
  installation whose flag set did not arrive would be read as "off", and the
  error grows exactly where sessions are short. The Flag Census costs no new
  wire value: it is the presence of the `flags` field.
- **An `Unknown` variant in each flag group.** Rejected. A complete-or-absent
  flag set gives the same information with one rule, and an unsettled source
  then shrinks the census instead of adding a value every reader must
  exclude.
- **Weekly retention.** Deferred. It needs a week token and a cohort week;
  monthly answers the current question, and adding week granularity later is
  a contract expansion with renewed consent.
- **Reuse the download dashboard repository for ingestion.** Rejected; see
  the service section.
- **Derive provider outcomes in the App from the monitor DTO.** Rejected.
  The DTO's support fields and guidance candidates do not distinguish
  "connecting" from "service missing" and lose the answer once ACPI or a
  guidance acknowledgement intervenes; see
  [Provider outcome source](#provider-outcome-source).
- **A `feature_on.sensor_service` flag.** Rejected for v1. The service is the
  only PawnIO path, so `ProviderAvailable` already covers it, and sampling
  the service's own state would add an automatic status read ADR 0029 keeps
  in Settings.
- **`ProviderAvailable` for Linux `smartctl` and `intel_gpu_top`.** Rejected
  for v1. Their availability is decided per command rather than probed once,
  so a daily sampled fact would not say what a reader expects. Adding them
  later is a contract expansion with renewed consent.

## Verification

Each claim maps to the narrowest evidence that proves it (DP-09):

| Claim | Evidence |
| --- | --- |
| Default is disabled for new and upgraded installations | Settings service test: absent key and legacy files parse to `undecided`; invalid section recovers to `undecided` |
| Unknown keys survive | Existing `serialize_preserving_unknown_settings` test extended with the new section, plus a Core-owned key beside it |
| No request before opt-in, none after opt-out | Rust test with the recording transport double: events recorded while undecided/declined produce zero transport calls; grant then decline leaves the queue empty and the next flush makes no call; a decline during an in-flight request aborts it and the double sees no completion; a flush that read `granted` before a decline and reaches transport admission after it makes no call (the test holds the flush between sealing and admission with an injected gate, then declines); a completion that lands after a decline leaves the queue and `reportedDay`/`reportedMonth` untouched; with a settings-write double that blocks, a decline followed by a flush attempting admission makes no call while the write is still pending, a decline whose settings write fails leaves flush admission closed (the next flush makes no call) and the persisted decision unchanged at `granted`; a grant or renew started while a decline is in progress completes after it, and overlapping decline/grant and decline/renew pairs in both orders, with every write succeeding, end with the persisted decision and flush admission agreeing. Plus one manual network trace on a release build, recorded in the PR |
| Sealed-batch retry policy | Unit test: network error, timeout, `429`, and `5xx` keep the head batch; `400` and `413` drop it and the next batch is sent; a batch older than the Worker window is dropped unsent |
| Month token once per month | Unit test: with a persisted `reportedMonth`, the second batch of a month omits the month token; a new month seals it again |
| Version bump requires renewed consent | Service test: `granted` at version 1 with `CURRENT = 2` records nothing and the DTO reports the dialog condition |
| Opt-out clears queue, seed, and cohort | Service test asserts queue length 0, `seed = null`, and `cohortMonth = null` after decline; re-grant yields a different seed and the current month; a version-renewal grant keeps both |
| Only allowlisted events | Compile-time: the command takes the enum; a frontend test checks the generated binding type has no string variant |
| Flags carry no raw strings | Unit test: every vendor string fixture maps to a named variant or `Other`; the wire serializer has no string path |
| Flags once per day | Unit test: with a persisted `reportedDay`, a simulated restart in the same UTC day seals a batch without flags; the next UTC day seals them again; a clock earlier than `reportedDay` seals no flags; a failed `reportedDay` write seals the batch without flags |
| Flag set complete or absent | Unit test: while a Windows provider outcome is `pending`, a sealed batch has no `flags` field and `reportedDay` is unchanged; once both are `answered` or `notAnswered`, the next batch carries exactly one variant of each exclusive group; on macOS and Linux no `provider_available` flag is ever sealed |
| Provider outcome is a Core fact | Core test: a service still connecting reports `pending`; a missing service reports `notAnswered` for both providers even when ACPI supplies the headline CPU temperature; a recognized ITE chip with temperatures and unsupported fans reports motherboard `answered`; an acknowledged guidance key does not change the outcome |
| Flag rates use the Flag Census | Service-repository test: a batch with `flags` increments the census once for its day and OS family, a batch without it does not; a rate is flag total over census; a `flags` array missing an exclusive group, or a provider flag from a non-Windows batch, is rejected with `400`; a rate below `k` is withheld |
| Metric definitions | Service-repository test: per-dimension Active Installation rows sum to the period total; a cohort below `k` publishes no retention row; a retention value above 1 is published unclamped; quality counters record accepted batches, repeated ids, and each rejection reason |
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
2. **Service repository**: Worker, D1 schema, per-flag totals and the Flag
   Census, cohort retention derivation, rollup and retention jobs, public
   endpoints, JSON Schema and examples, tests. Public before any client
   release.
3. **Consent persistence** (`feat/`, Rust): `productAnalytics` section with
   seed, Consent Cohort, and the reported day/month, service transitions
   including renew, DTO, commands, regenerated bindings, tests.
4. **Provider outcome** (`feat/`, Core): the optional sensor provider
   outcome in `MetricsSnapshot`, with the Core tests above. Independent of
   slice 3.
5. **Analytics runtime** (`feat/`, Rust): event and flag enums, counters,
   flag sampling and vendor mapping, tokens, sealed batches, transport trait
   and reqwest implementation, worker registration, tests with the transport
   double. Needs slices 3 and 4.
6. **Consent surface and event recording** (`feat/`, frontend): dialog,
   Settings section, shared hook, screen-view and capability call sites,
   language files, E2E mocks and tests, screenshots.
7. **Website** (`hardviz-webpage`): aggregate display from the public
   endpoint.
8. **Release**: first stable tag no earlier than T0 + 30 days (2026-12-03);
   record the version in ADR 0028.

Slices 3, 5, and 6 are stacked and each is independently reviewable; slice 4
can proceed beside slice 3, and slice 2 beside all of them once the contract
text is agreed.

## Open questions

- The initial `Capability` and `Feature` lists. The four capabilities above
  are the ones with a clear user-started action today; the eight features are
  the Application Preferences a user switches on or off. Anything else is a
  guess until v1 data exists.
- Whether `k = 20` is the final suppression threshold, and whether month
  totals below `k` should be shown as "fewer than 20" rather than the number.
- Whether a declined installation should be told, in Settings, when the
  contract version changes. The current answer is no: nothing about their
  situation changed.
