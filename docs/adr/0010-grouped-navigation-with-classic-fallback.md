# Grouped Navigation with Classic Fallback

Status: accepted

The sidebar previously exposed five flat entries: Hardware Dashboard, Usage,
CPU, Insights, and Settings. Grouped navigation restructures those entry
points around one Grouped Dashboard destination, the existing Insights Screen,
and Settings. Inside the Grouped Dashboard, peer Performance and System
Specifications tabs separate the two jobs that the previous Hardware
Dashboard and Usage screens mixed together.

The Performance tab combines live current values with the short-window Usage
Graphs. Performance Layout Presets (Compact, Monitor, Detailed, and Custom)
arrange that monitoring surface. Detailed is a dense view of current and
short-window data, not a historical analysis surface. Long-term analysis
remains the responsibility of the existing Insights Screen.

The System Specifications tab presents the available hardware configuration,
platform, capability-dependent observations, and Hardware Report in one
surface. CPU, GPU, memory, storage, motherboard, platform, and network are
content sections, not sidebar destinations or separate screens. The initial
tab may reuse existing Hardware Dashboard blocks that still mix static facts
with current observations. Separating those blocks more deeply can be
delivered incrementally without changing the navigation hierarchy.

The existing Insights Screen and its main, GPU, Process, and Snapshot views are
outside this redesign. Its content and internal tabs remain unchanged. The
integration and behavioral changes in this decision are limited to the
Hardware Dashboard and Usage experiences.

This structure follows time-and-subject semantics: live readings and the
short-window Usage Graph describe now and belong on one monitoring tab, while
hardware and platform facts belong on the peer specifications tab. Archived
analysis is a different activity and stays in Insights. Merging the chart set
into one shared panel also gives upcoming chart features such as network usage
and graph grouping one implementation point shared with Classic Navigation.

Grouped navigation is the default, including for existing users. A
`navigationLayout` Application Preference in `settings.json` opts back into
Classic Navigation, which shows the previous five flat entries and preserves
their existing user-visible behavior and appearance. Classic screens may
internally reuse components shared with grouped navigation. Removing Classic
Navigation is a separate future decision and requires its own record.

Grouped navigation must not become the default in a stable release until the
Performance tab, the System Specifications tab, the Classic Navigation
fallback, and the migration notice are available together. This allows the
implementation to merge incrementally without exposing an incomplete default
experience in a stable release.

Only `navigationLayout` and the versioned acknowledgement of the restructure
announcement are Application Preferences. The selected screen, selected
Grouped Dashboard tab, active Performance Layout Preset, and Custom arrangement
stay UI-local in Tauri Store, matching the existing Dashboard item order and
Insights tab precedent. A selected-screen value is normalized for the target
layout only after Application Preferences load, so an explicit Classic
selection is not rewritten using the grouped default during startup.

Rendering cost must follow visible value. Only the active Grouped Dashboard
tab mounts. Within Performance, only panels of the active preset mount and
subscribe to live updates. Switching to System Specifications unmounts the
Performance subtree; switching back unmounts the specifications subtree.

The sidebar hierarchy, in-screen tabs, and content sections are distinct
levels. Requirements and tests for a navigation change must identify those
levels explicitly rather than inferring routes from a grouped label.
Grouped Dashboard, Performance Tab, System Specifications Tab, Performance
Layout Preset, and Classic Navigation are defined in `CONTEXT.md`.

## Consequences

### Positive

- Live values and short-window graphs share one monitoring tab.
- Hardware and platform information remains reachable from the same Dashboard
  destination without proliferating category routes.
- New chart capabilities have one shared implementation point.
- Existing users can return to the previous navigation without losing access
  to current screens.
- Long-term Insights remain isolated from the live monitoring redesign.
- Inactive live surfaces unmount instead of continuing hidden work.

### Negative

- Grouped and Classic Navigation must coexist for an unspecified transition
  period.
- Screen routing and persisted selection require normalization between two
  navigation models.
- The initial System Specifications tab may retain mixed-purpose Dashboard
  blocks until its information hierarchy is deepened.
- The Performance tab can mount more live-updating content than an existing
  single screen and therefore requires explicit subscription controls.

### Non-goals

- Redesigning the existing Insights Screen.
- Creating standalone CPU, GPU, Memory, Storage, or System routes in grouped
  navigation.
- Removing Classic Navigation.
- Completing the final information hierarchy of every System Specifications
  section.
- Changing the underlying data collection or archive model.
