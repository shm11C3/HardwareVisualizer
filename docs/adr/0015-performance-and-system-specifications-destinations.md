# Performance and System Specifications as Sidebar Destinations

Status: accepted

Supersedes the single-destination part of
[ADR 0010](0010-grouped-navigation-with-classic-fallback.md). Builds on
[ADR 0014](0014-performance-views-and-specification-sheet.md).

ADR 0010 consolidated five flat sidebar entries into one Grouped Dashboard
destination holding peer Performance and System Specifications tabs. After
ADR 0014 finished the presentation work, the result showed two symptoms:

- The grouped sidebar carried two section headers, each labelling exactly one
  entry. Section headers exist to group siblings, so a header per single child
  is decoration standing in for structure that is not there.
- The Performance screen still stacked two rows of horizontal controls: the
  Grouped Dashboard tab row, then the Performance toolbar. ADR 0014 removed
  the preset pill row for exactly this reason, but the tab row survived it.

Performance and System Specifications become peer sidebar destinations. The
Grouped Dashboard destination and its tab strip are removed.

## Structure

Grouped navigation lists three destinations plus Settings, with no section
headers:

- Performance — the live monitoring screen and the default landing target.
- System Specifications — the hardware and platform reference sheet.
- Insights — archived analysis, unchanged.
- Settings — unchanged, still pinned to the bottom of the rail.

Classic Navigation keeps its five flat entries and its behavior. The gap
between the two layouts narrows as a result; whether Classic is still worth
carrying is a separate future decision and is not reopened here.

## Why tabs were the wrong control

Tabs suit peer views a user alternates between often, where the switching cost
must stay near zero. Performance is watched continuously; System
Specifications is consulted occasionally, when troubleshooting, reporting a
machine, or planning an upgrade. The pair is rarely alternated, so the tab
strip charged permanent screen space for a cheap switch nobody needed.

Making both destinations also lets each screen state its own name. "Dashboard"
covered both now-state and static facts, which is vague; "Performance" and
"System Specifications" are exact. The result maps one-to-one onto the app's
distinct jobs — now, facts, history, configuration — which matches the
time-and-subject rule the product principles already require.

ADR 0010's level discipline still holds: CPU, GPU, memory, storage,
motherboard, platform, and network remain content sections, not destinations.
This decision moves exactly one level boundary, and nothing below it.

## Stored selection

`normalizeDisplayTarget` owns the migration. A stored `groupedDashboard`
selection resolves to Performance, the destination users watch. Which tab was
open last was UI-local state, so `groupedDashboardTab` is orphaned rather than
migrated, matching how ADR 0014 treated the `systemSpecifications*` keys.
Normalization still runs only after Application Preferences load, so an
explicit Classic selection is never rewritten with a grouped default during
startup.

The restructure notice is reworded: it previously told users that Performance
and system specifications live under the Dashboard, which is no longer true.

## Consequences

### Positive

- The Performance screen loses a full row of chrome; one toolbar remains.
- Each sidebar entry names exactly what the screen is.
- Section headers disappear because the entries no longer need grouping.
- System Specifications is reachable in one click from anywhere instead of
  two, and its selected state is visible in the rail.

### Negative

- The sidebar gives a rarely-used reference screen the same visual weight as
  the screen users watch all day.
- Users who left the app on the specifications tab land on Performance after
  upgrading, because the last-open tab is not carried over.
- A second navigation change reaches users close behind the ADR 0010
  restructure notice.

### Non-goals

- Changing Classic Navigation, the Insights Screen, or anything below the
  destination level.
- Retiring Classic Navigation, which needs its own decision.
- Revisiting the ADR 0014 presentation decisions inside either screen.
