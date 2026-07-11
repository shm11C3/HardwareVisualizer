# Live Storage Health on Demand

Status: accepted

Live Storage Health is current storage device health signals collected for immediate display, distinct from the retained daily Storage Health Record. We decided to deliver Live Storage Health on demand — a command invoked while a Storage Health Display is visible — instead of adding storage signals to the continuous metrics stream that carries CPU and GPU utilization. Collection therefore happens only while something is actually showing the data, and nothing is persisted.

This is a deliberate asymmetry with GPU temperature, which streams continuously. The metrics stream collects whether or not any view needs storage signals, and storage health reads are not uniformly cheap: outside the native Windows path they spawn external processes (`smartctl`) or open WMI connections. Polling those on a stream cadence would run exactly the kind of background work the on-demand shape avoids. A core-side minimum-interval guard deduplicates reads when multiple views poll at once.

Live reads use only the cheap native path (Windows `DeviceIoControl`, one IOCTL per device against a cached device list) with no fallback chain. Devices or platforms without a cheap path simply have no live signal, and displays fall back to the daily Storage Health Record — the prior behavior. The daily collection keeps its full fallback chain because it runs once a day, where heavier sources are acceptable.

Device enumeration is separated from reading: enumeration (WMI) runs at startup and on an explicit Storage Device Refresh, never on the live read cadence. Storage Device Refresh is also the only operation that deactivates devices that are no longer enumerated; a failed or empty enumeration never deactivates anything, so a transient collection failure cannot silently empty the device list.
