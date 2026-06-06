# HardwareVisualizer

HardwareVisualizer monitors local hardware health and performance. This glossary keeps product terms consistent across dashboard, persistence, and platform-specific hardware access discussions.

## Language

### Monitoring Views

**Hardware Monitoring**:
The app's ongoing activity of collecting and presenting local hardware status and utilization.
_Avoid_: Monitoring, Storage Monitoring, telemetry

**Hardware Dashboard**:
The current-status view where users inspect live CPU, memory, GPU, storage, network, process, and motherboard information at a glance.
_Avoid_: Dashboard settings, insight page, system report

**Dashboard Item**:
An information block on the Hardware Dashboard that can be shown, hidden, or reordered.
_Avoid_: Display Target, insight tab, graph target

**Live Process Table**:
The Hardware Dashboard table that shows currently observed processes and their current resource usage.
_Avoid_: Process Insight, process history, process table

**Hardware Report**:
An export of the hardware configuration information currently available to the app.
_Avoid_: Hardware Dashboard, Hardware Archive, utilization report, debug log

**Usage Graph**:
A short-window live history chart of recent hardware utilization.
_Avoid_: History, insight chart, hardware archive, snapshot chart

**Display Target**:
A hardware category selected for display in Usage Graphs.
_Avoid_: Dashboard item, visible item, insight tab

### Historical Views

**Hardware Insight**:
A historical view of archived hardware utilization, shown over a user-selected period.
_Avoid_: History, live graph, dashboard graph

**Insights Screen**:
The app screen that groups historical views such as Hardware Insight, Process Insight, and Insight Snapshot.
_Avoid_: Hardware Insight, Storage Health Display, dashboard

**Process Insight**:
A historical view of notable processes from the Hardware Archive.
_Avoid_: Live process table, task manager

**Insight Snapshot**:
A filterable historical view that relates archived CPU or memory usage to process records for the same period.
_Avoid_: Snapshot, storage health snapshot, live metrics snapshot, screenshot

### Persistence And History

**Hardware Archive**:
Persisted CPU, memory, GPU, and process utilization history used to power Hardware Insights.
_Avoid_: Storage health history, live history, dashboard state, settings archive

**Retention Period**:
The length of time a persisted history is kept before old records are eligible for deletion.
_Avoid_: Retention, refresh interval, cleanup schedule

### Preferences And App Behavior

**Application Preference**:
A user-facing choice that users reasonably expect to persist as part of the app configuration.
_Avoid_: UI cache, transient state, local selection

**UI-local State**:
Transient interface state that can be reset without losing an explicit user configuration.
_Avoid_: Application preference, persisted setting

**Tray Widget**:
A compact hardware-metrics display opened from or near the system tray.
_Avoid_: Close to Tray, tray setting, tray

**Close to Tray**:
The window-close behavior where the app stays running in the system tray instead of exiting.
_Avoid_: Tray Widget, tray setting, minimize to tray

**Burn-in Shift**:
A display-protection feature that subtly moves long-lived UI content over time.
_Avoid_: Screen saver, layout animation, window positioning

### Storage Health

**Storage Monitoring**:
Presenting current storage device capacity, free space, storage type, and filesystem information.
_Avoid_: Storage health, SMART status, storage archive

**Storage Health Collection**:
Obtaining storage device health signals so the application can retain a health record for a device.
_Avoid_: SMART display, dashboard health row

**Storage Health Record**:
A daily record of storage device health signals for one storage device on one local date.
_Avoid_: Storage Health Snapshot, Hardware archive, raw SMART data, dashboard item

**Storage Health Device Identity**:
A local, app-installation-scoped identifier used only to associate Storage Health Records for the same physical storage device across dates.
_Avoid_: User identity, cross-device sync identity, raw serial number, globally stable storage device ID, externally shareable device identity

**Storage Health Display**:
Presenting already available Storage Health Records in the user interface, including dashboard summaries and historical views.
_Avoid_: SMART collection, storage health acquisition
