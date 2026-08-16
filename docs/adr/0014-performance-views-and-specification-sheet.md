# Performance Views and the Specification Sheet

Status: accepted

Refines: [ADR 0010](0010-grouped-navigation-with-classic-fallback.md)

The first Grouped Dashboard implementation shipped four Performance Layout
Presets (Compact, Monitor, Detailed, Custom) and reused the Hardware Dashboard
card grid for the System Specifications tab. Maintainer review of the rendered
result identified concrete problems: every panel explained itself with heading
plus description prose, the "Last 60 seconds" caption repeated once per metric
card, two stacked pill rows (Dashboard tabs plus preset tabs) read as redundant
chrome, Compact carried too little information to justify a preset, Detailed
and Custom differed only in whether an always-visible editor panel occupied the
top of the screen, and the specifications card grid produced large blank areas
because card heights did not match while static facts stayed mixed with live
readings.

This decision keeps the ADR 0010 navigation hierarchy (Grouped Dashboard with
peer Performance and System Specifications tabs) and replaces the presentation
inside both tabs.

## Performance Views

The Performance Tab exposes three Performance Views: Panels (default), Compact,
and Monitor. The retired Detailed and Custom presets both normalize onto
Panels; Compact and Monitor stored values remain valid. The UI-local store keys
(`performanceLayoutPreset`, `performanceCustomLayout`) are reused with
normalization instead of migrated.

Panels combines a fixed Instrument Strip with one reorderable panel stack:

- The Instrument Strip is the always-mounted live header: one card per
  top-level metric (CPU, RAM, GPU), each keeping the classic Hardware Dashboard
  identity — the same `DoughnutChart` gauges (usage plus a temperature gauge
  where the platform reports one, staggered diagonally when the card is too
  narrow for two side by side), the hue-coded card icon, and no card border —
  extended with a short-window sparkline. Secondary readings (clock,
  core/thread count, used/total memory, VRAM, fan) render inside the card and
  only when the platform reports them.
- The panel stack is ordered with dnd-kit. Usage Graphs and Live Processes are
  visible by default; Per-core Usage and Motherboard Sensors ship hidden until
  explicitly enabled, so the default screen only carries what most sessions
  watch. Newly introduced panels join a stored layout hidden unless they are
  visible by default. An empty visible set is valid because the Instrument
  Strip stays mounted.
- Arrangement controls (drag handles, hide buttons, the hidden-panel strip)
  exist only inside an explicit edit mode entered from a toolbar button. There
  is no permanently mounted layout editor.
- The stack renders in one or two columns, chosen by the user and stored as
  UI-local state. Two columns are an upper bound rather than a guarantee: below
  the wide breakpoint the grid collapses to one column so panels stay readable,
  while the stored choice survives for when the window is wide again.
- Only visible panels mount and subscribe to live updates, preserving the
  ADR 0010 rendering-cost rule.

Compact is a dense one-row-per-metric strip (label, level bar, percent,
secondary reading, sparkline) intended for a small window kept in a screen
corner. Rows and footer entries are declarative: metrics the backend does not
collect yet (disk activity, network throughput, process count) become new row
builders without layout work, and rows without data are omitted rather than
rendered empty. Compact deliberately overlaps the Tray Widget; the strip is
the in-window variant and the Tray Widget remains the out-of-window variant.

Compact also has an expanded mini-monitor mode, its primary use: an expand
control replaces the whole screen with the strip alone — no navigation, tabs,
title, or view switcher — with rows sharing the full height. The layer renders
in a portal and marks the app root inert, so the hidden UI is neither
focusable nor exposed to assistive technology rather than merely painted over.
It carries one labelled exit control and also exits on Escape, and the choice
is stored as UI-local state so a dedicated screen stays configured across
restarts.

Monitor remains the full-height combined Usage Graph, now without a
surrounding card frame so the graph sits directly on the window background.

Panel headers use a single small eyebrow label. Explanatory sentences under
panel titles, per-card time captions, and the page description are removed;
the time axis is stated once by the graph.

## System Specifications sheet

The System Specifications tab renders a flat single-column sheet of sections
(CPU, GPU, Memory, Storage, Motherboard, Platform, Network) with hairline
key-value rows. Section height follows content, so the card-grid blank areas
disappear. The sheet has no drag-and-drop arrangement and no visibility
selector; the former `systemSpecifications*` store keys are orphaned rather
than migrated.

Static facts and live readings are separated: live motherboard temperatures
and fan speeds move to the Performance Motherboard Sensors panel, and the
sheet keeps facts plus the existing storage block, which continues to include
the daily-record Storage Health summary and capacity chart. A Platform section
(operating system, version, architecture) is added from the OS plugin. The
Hardware Report copy action stays on the sheet.

Storage capacity stays a specification fact rather than a Performance
reading. Free and used space come from the one-shot `getHardwareInfo` fetch,
not the 1 Hz update stream, so presenting them next to live metrics would
claim a liveness the data does not have. They also belong with the daily
Storage Health record and the drive facts they describe. What belongs on the
Performance side is disk *activity* (throughput, busy time), which the backend
does not collect yet; when it does, it joins the Compact rows and the panel
stack as a live metric. A capacity reading may only appear on a Performance
surface if it is labelled as a snapshot with its own timestamp instead of
being mixed into the short window.

## Consequences

### Positive

- One customizable monitoring layout replaces two near-identical presets and a
  permanently mounted editor.
- Wide windows can show two panel columns without the layout breaking on
  narrow ones, because the column count degrades instead of scrolling
  sideways.
- The default Performance screen carries only the panels most sessions watch;
  everything else is opt-in and unmounted while hidden.
- Compact has a defined job (small-window monitoring) and a defined growth
  path for not-yet-collected metrics.
- The specifications sheet reads as one reference document and no longer
  mixes static facts with live readings, and storage keeps its facts, capacity,
  and health record together in one place.
- The classic doughnut identity is preserved by reusing the existing
  `DoughnutChart`, so the gauge keeps the tween length that ADR-adjacent
  performance work tuned to settle inside the 1 Hz update interval.
- The Live Process Table keeps its own card and gains no second frame, because
  panels that already render as a card get no outer panel chrome.

### Negative

- Stored Detailed/Custom selections silently become Panels; a user who relied
  on switching between two arrangements loses that distinction.
- Compact duplicates part of the Tray Widget's job inside the window.
- The Storage and Network sections reuse legacy block components inside the
  sheet, so their internal layout does not yet match the key-value rows.

### Non-goals

- Changing the ADR 0010 navigation hierarchy, Classic Navigation, or the
  Insights Screen.
- Collecting new metrics (disk activity, network throughput) for Compact.
- Restyling the reused Storage and Network blocks into key-value rows.
