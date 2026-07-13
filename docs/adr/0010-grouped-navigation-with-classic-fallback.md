# Grouped Navigation with Classic Fallback

Status: accepted

The sidebar today is five flat entries: Hardware Dashboard, Usage, CPU,
Insights, and Settings. We decided to restructure navigation into two grouped
sections: a Dashboard section containing a Performance Screen and per-category
Hardware Category Screens (CPU, GPU, Memory, Storage, System), and an Insight
section containing the existing Insights Screen. The Performance Screen merges
the live current values from the Hardware Dashboard with the short-window
Usage Graphs that currently live on the separate Usage screen, arranged through
Performance Layout Presets (Compact, Monitor, Detailed, Custom). Detailed is a
more information-dense view of current and short-window data; it is not a
historical analysis surface. Long-term analysis remains the responsibility of
the existing Insights Screen.

The existing Hardware Dashboard also mixes two different user tasks: checking
the machine's current operating state and inspecting its hardware
specifications or configuration. Its live-monitoring task overlaps with the
standalone Usage screen. Several usability requests can be interpreted as
consequences of this mixed responsibility. Addressing those requests card by
card would preserve the underlying information-architecture problem. This
decision instead separates current-state monitoring into the Performance
Screen and specification-oriented content into Hardware Category Screens.

The initial Hardware Category Screens may reuse and reorganize existing
content. A deeper specification-oriented redesign can be delivered
incrementally without changing this navigation decision.

The existing Insights Screen and its main, GPU, Process, and Snapshot views are
outside this redesign. Grouped navigation changes only how users reach that
screen; it does not merge, relocate, or otherwise change its content. The
integration and behavioral changes in this decision are limited to the
Hardware Dashboard and Usage screen.

The grouping follows time-and-subject semantics: live readings and the
short-window Usage Graph describe "now" and belong on one monitoring surface,
while archived analysis is a different activity and stays in its own section.
Merging the chart set into one shared panel also gives upcoming chart features
(network usage chart, graph grouping mode) a single implementation point that
serves both the new and the classic screens.

Grouped navigation becomes the default, including for existing users. A
`navigationLayout` Application Preference in `settings.json` opts back into
Classic Navigation, which shows the previous five flat entries and preserves
their existing user-visible behavior and appearance. Its screens may internally
reuse components shared with grouped navigation. Classic screens are retained,
not duplicated: grouped mode hides them from navigation instead of rendering
them twice, and the Monitor preset of the Performance Screen is expected to
cover the standalone Usage screen's leave-it-running use case. Removing Classic
Navigation is a separate future decision and requires its own record.

Grouped navigation must not become the default in a stable release until the
Performance Screen, the initial Hardware Category Screens, the Classic
Navigation fallback, and the migration notice are available together. This
allows the implementation to merge incrementally without exposing an
incomplete default experience in a stable release.

Only `navigationLayout` and the versioned acknowledgement of the restructure
announcement are Application Preferences. The selected screen, the active
Performance Layout Preset, and the Custom arrangement stay UI-local in Tauri
Store, matching the existing Dashboard item order and Insights tab precedents.
A single selected-screen value is normalized for the target layout on load and
layout switch (for example `usage` maps to `performance` under grouped
navigation, while `performance` maps to the Hardware Dashboard under Classic
Navigation). The previous selection is not separately retained per layout.

Rendering cost must keep following visible value: the Performance Screen shows
more live-updating panels at once than any current screen, so only the mounted
panels of the active preset may subscribe to 1Hz updates, and hidden panels are
unmounted rather than hidden. Performance Screen, Performance Layout Preset,
Hardware Category Screen, and Classic Navigation are defined in `CONTEXT.md`.

## Consequences

### Positive

- Live values and short-window graphs share one monitoring surface.
- Hardware configuration becomes easier to navigate by category.
- New chart capabilities have one shared implementation point.
- Existing users can return to the previous navigation without losing access
  to current screens.
- Long-term Insights remain isolated from the live monitoring redesign.

### Negative

- Grouped and Classic Navigation must coexist for an unspecified transition
  period.
- Screen routing and persisted selection require normalization between two
  navigation models.
- The Performance Screen can mount more live-updating content than existing
  screens and therefore requires explicit subscription and rendering controls.
- Reusing existing content in the initial Hardware Category Screens may create
  temporary inconsistencies until their deeper redesign is complete.

### Non-goals

- Redesigning the existing Insights Screen.
- Removing Classic Navigation.
- Completing the final visual design of every Hardware Category Screen.
- Changing the underlying data collection or archive model.
