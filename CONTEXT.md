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

**Performance Screen**:
The grouped-navigation view for inspecting current hardware state together with short-window Usage Graphs.
_Avoid_: Insights Screen, Hardware Insight, hardware specifications

**Performance Layout Preset**:
A UI-local arrangement of panels on the Performance Screen, such as Compact, Monitor, Detailed, or Custom.
_Avoid_: Insights tab, saved report, navigation layout

**Hardware Category Screen**:
A grouped-navigation view that organizes hardware information for one category: CPU, GPU, Memory, Storage, or System.
_Avoid_: Performance Layout Preset, Hardware Insight, dashboard item

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

**Classic Navigation**:
The opt-out navigation layout that preserves the previous five flat entries and their user-visible behavior while grouped navigation is the default.
_Avoid_: legacy screen, old dashboard, fallback mode

**UI-local State**:
Transient interface state that can be reset without losing an explicit user configuration.
_Avoid_: Application preference, persisted setting

**Tray Widget**:
A compact hardware-metrics display opened from or near the system tray.
_Avoid_: Close to Tray, tray setting, tray

**Close to Tray**:
The window-close behavior where the app stays running in the system tray instead of exiting.
_Avoid_: Tray Widget, tray setting, minimize to tray

**Elevated Startup Mode**:
A user preference for starting HardwareVisualizer with operating-system administrator privileges so privileged hardware access stays available across launches.
_Avoid_: Admin mode, privileged core mode, installer option

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

**Storage Wear**:
A storage health signal estimating how much of a storage device's expected lifetime has been consumed, distinct from capacity usage.
_Avoid_: Used, disk usage, capacity used, storage utilization

**Storage Health Display**:
Presenting already available Storage Health Records in the user interface, including dashboard summaries and historical views.
_Avoid_: SMART collection, storage health acquisition

**Focus Storage Device**:
The storage device surfaced first in a Storage Health Display because its health state most needs the user's attention.
_Avoid_: Representative drive, selected disk, SMART target

**Selected Storage Device**:
The storage device the user explicitly chooses as the primary subject of a Storage Health Display, distinct from the automatically chosen Focus Storage Device.
_Avoid_: Display Target, selected disk, SMART target

**Live Storage Health**:
Current storage device health signals collected for immediate display and not retained as history, distinct from the daily Storage Health Record.
_Avoid_: Storage Health Record, storage health snapshot, realtime SMART

**Storage Device Refresh**:
The user-initiated action that re-detects connected storage devices, collects current health signals, updates today's Storage Health Record, and reflects added or removed devices in displays.
_Avoid_: Rescan, reload, auto-detection, live polling

### Motherboard Sensors

**Motherboard Sensor Display**:
Presenting all available live motherboard temperature and fan-speed readings on the Hardware Dashboard.
_Avoid_: CPU thermal zones, GPU sensors, Storage Health Display

**Sensor Source Label**:
A compact label that identifies where a live sensor reading came from, such as a provider, chip, or platform source.
_Avoid_: Debug output, hardware report, install guidance

**Fan Speed Reading**:
A live fan-speed value reported for a hardware fan source, typically shown as RPM.
_Avoid_: Fan health, fan control, PWM setting

**Inactive Fan Reading**:
A fan-speed reading of 0 RPM, meaning the fan is not currently spinning or not reporting rotation; it does not by itself prove disconnection or fault.
_Avoid_: Disconnected fan, failed fan, missing fan

**Active Fan Reading**:
A fan-speed reading above 0 RPM, meaning the fan is currently reporting rotation.
_Avoid_: Healthy fan, connected fan

**Invalid Fan Reading**:
A fan-speed reading that cannot be trusted for display because the reported value is outside the accepted reading shape.
_Avoid_: Failed fan, broken fan

### Sensor Availability

**External Component Guidance**:
A user-facing notice shown after hardware collection tries and cannot use an optional external runtime component, and fallback collection still leaves user-visible hardware data unavailable.
_Avoid_: Startup dependency check, install prompt, dependency error, missing component alert
