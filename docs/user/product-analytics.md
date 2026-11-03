# Product Analytics and Privacy

[English](product-analytics.md) | [Japanese](product-analytics.ja.md)

This page is the public data contract and privacy notice for HardwareVisualizer
product analytics. The in-app consent dialog and the Settings screen link to
it. It is published in advance of any release that can send analytics; see
[Status](#status).

## Status

**Planned, not yet in any release.** No released version of HardwareVisualizer
sends product analytics today. This notice was published first, on purpose, so
that the decision is public before the code ships
([ADR 0028](../adr/0028-opt-in-product-analytics.md)). The first version that
includes the opt-in is named here when it is released, and no stable release
includes it earlier than 30 days after this notice was merged.

## What product analytics is

Aggregate counts that help the maintainer see how many installations are
active, how many of the installations that opted in during one month are
still active later,
which screens and features are used or left switched off, and on which
hardware vendor classes and sensor providers the app runs.

It is not crash reporting, not diagnostics, and not a detailed or time-ordered
record of what you do in the app: it sends only daily totals per screen or
capability. It never contains your hardware readings.

## Your choice

- Product analytics is **off by default** for every new and upgraded
  installation. Nothing is sent until you turn it on.
- The first time an eligible version starts, a dialog asks once. "Share" and
  "Don't share" have the same weight. Closing the dialog without choosing
  counts as "Don't share".
- You can turn it on or off at any time in **Settings → Advanced → Product
  analytics**.
- Turning it off cancels any upload in progress, sends nothing further,
  discards anything queued, and deletes the local random seed described
  below. A batch that was already delivered stays in that day's and month's
  totals. Turning it on again later creates a new seed, so your installation
  counts as a new one.
- If a future version wants to send more than this page lists, the contract
  version increases and the app asks you again before sending anything new.
  If you declined, you are not asked again.

## What is sent (contract version 1)

Each batch is one small JSON document, sent over HTTPS to a service the project
runs on Cloudflare. A batch is sent about a minute after start, then about
every six hours while the app runs, and once right after you turn analytics on.

| Field | Example | Purpose |
| --- | --- | --- |
| Schema and consent version | `1`, `1` | So the service and this page can say exactly which contract a batch follows |
| App version | `1.12.0` | Adoption of new versions |
| OS family | `windows`, `macos`, `linux` | Platform split |
| CPU architecture | `x86_64`, `aarch64` | Platform split |
| Consent month | `2026-11` | The month you turned analytics on; used to compute how many installations from that month are still active later |
| Day token and month token | 32 hex characters each | The day token is in every batch; the month token only in the first batch of each month. Together they count your installation once per UTC day and once per UTC month; see below |
| Batch id | random UUID | Lets the service ignore a batch it already received if the response was lost |
| Product Event counters | `screen_view.performance: 12` | How often each screen or capability was used, per UTC day |
| State Flags | `feature_on.tray_widget` | Which features were on, which layout and view were in use, CPU and GPU vendor class, and whether a sensor provider was available, in the first batch of each UTC day |

### Product Events

- `screen_view.dashboard`, `screen_view.performance`,
  `screen_view.system_specifications`, `screen_view.usage`,
  `screen_view.cpu_detail`, `screen_view.insights`, `screen_view.settings`
- `capability.hardware_report_exported`, `capability.tray_widget_opened`,
  `capability.external_component_setup_started`
- `capability.insight_opened.hardware`, `.cooling`, `.gpu`, `.process`,
  `.snapshot`

A counter is a number per UTC day. It does not say when during the day
something happened.

### State Flags

Only flags that are true are sent, as one complete set at most once per
installation per UTC day. If the app has not finished detecting your
hardware and sensor providers before an upload, the set waits for a later
upload that day instead of being sent incomplete:

- `feature_on.hardware_archive`, `feature_on.storage_health`,
  `feature_on.tray_widget`, `feature_on.close_to_tray`,
  `feature_on.elevated_startup_mode`, `feature_on.ambient_sensor`,
  `feature_on.burn_in_shift`, `feature_on.background_image`
- `navigation.grouped`, `navigation.classic`
- `view.performance.panels`, `view.performance.compact`,
  `view.performance.monitor`
- `hardware.cpu_vendor.intel`, `.amd`, `.apple`, `.other`
- `hardware.gpu_vendor.nvidia`, `.amd`, `.intel`, `.apple`, `.other` (one per
  detected GPU vendor)
- `provider_available.pawnio_cpu`, `provider_available.superio`

The service adds each flag to its own daily total as it receives the batch
and keeps no record per installation, so nothing stored says which flags
arrived together, and the data cannot say, for example, "installations with
the archive off and the tray widget on".

## What is never sent

- Names, email addresses, usernames, hostnames, or account identifiers.
- Your IP address as part of the data. (The connection itself has one; see
  [How the service handles data](#how-the-service-handles-data).)
- MAC addresses, serial numbers, device IDs, or any hardware identity the app
  stores locally.
- CPU, GPU, storage, motherboard, or other model names.
- Process or application names, window titles, file paths, logs, stack
  traces, or error text.
- Hardware readings: usage, temperature, power, fan speed, health, capacity.
- Exact timestamps of anything you did.
- Locale, location, or country.

If you want to tell the project which exact hardware you have, for example to
help verify a sensor chip, that is a separate, manual
Hardware Compatibility Report that you post yourself
([#2500](https://github.com/shm11C3/HardwareVisualizer/issues/2500)).

## How installations are counted

When you turn analytics on, the app generates 32 random bytes and stores them
in your settings file. The seed never leaves your computer. From it, the app
derives a different token for each UTC day and each UTC month. No token can
be turned back into the seed, and no token can be computed from another one.
Within a day, every batch carries the same day token; that is how the
service counts you once that day. The month token is sent only in the first
batch of each month, so it travels next to exactly one day token; the service
counts each of them and does not keep the pair. Token rows are deleted a few
days after their period ends.

There is no permanent installation identifier. The consent month is a group
label shared by every installation that opted in that month; it lets the
service compute "how many of the installations that opted in during November
were active in February" from monthly totals without following any single
installation.

## How to read the published numbers

The published numbers describe installations that chose to send analytics,
not people and not every copy of the app. The project reads them as trends,
never as a count of its users.

- **Active Installations** count opted-in installations whose upload for a
  day or month was accepted. An installation counts if the app ran long
  enough to upload, even if it only sat in the tray. One person on two
  computers counts twice; turning analytics off and on again, resetting
  settings, or reinstalling can make the same computer count as a new
  installation; a computer that is offline, blocks the service, or runs the
  app for less than a minute is not counted. The numbers are not compared
  with download counts.
- **Consent Cohort Retention** is the share of the installations that opted
  in during one month that are still counted in a later month. It starts when
  you opt in, not when you install, and it ends when uploads stop for any
  reason, including turning analytics off. The first month after release
  mostly contains installations that were already in use, so it is labeled
  separately.
- **State Flag percentages** are computed per day and operating system
  against the number of installations that sent a complete set of flags that
  day, not against all Active Installations, so an installation whose flags
  did not arrive is never counted as "off". A percentage over several days is
  a share of installation-days. Percentages based on fewer than 20
  installations are not shown.
- **Data quality.** The service cannot verify that an upload came from a real
  copy of the app, so anyone can send fabricated data, and some real data is
  always missing. The service rejects malformed uploads and limits request
  rates, publishes how many uploads it refused and why, and marks a period as
  suspect when it looks manipulated. Because it keeps no per-installation
  record, it cannot remove fabricated data afterwards; that is the same
  property that keeps your installation from being followed.

## How the service handles data

- The service is a Cloudflare Worker with a D1 database, in a separate public
  repository. Its link is added to this page before the first release.
- Cloudflare delivers the HTTPS request. In doing so it processes connection
  metadata, including your source IP address, under Cloudflare's own terms.
  The analytics dataset does not store IP addresses, request headers, or raw
  request bodies, and the service does not log them. Rate limiting at the edge
  uses the connection, not stored data.
- Rotating tokens are online identifiers during the day or month they are
  valid. This notice therefore does not claim that no personal-related data is
  ever processed; it claims that nothing stored can identify you or follow
  your installation across periods.
- Retention: day-token rows are deleted three days after their day, month-token
  rows three days after their month. Batch ids are kept seven days to drop
  duplicates. Aggregate totals (counts per period, per flag, per event) are
  kept as time series and contain no tokens.
- Published results: the project website shows aggregate tables only. Any
  breakdown with fewer than 20 installations is suppressed.
- The D1 processing location and the service's source are documented in the
  service repository, and this page links to both before release.

## Where to look

- Client source (after implementation): `src-tauri/src/analytics/` in the
  HardwareVisualizer repository.
- Decision record: [ADR 0028](../adr/0028-opt-in-product-analytics.md).
- Design: [`docs/design/product-analytics.md`](../design/product-analytics.md).
- Public RFC and discussion:
  [#2014](https://github.com/shm11C3/HardwareVisualizer/issues/2014).

## Changelog

- **Contract version 1** (notice published; not yet in a release): initial
  contract as listed on this page.
