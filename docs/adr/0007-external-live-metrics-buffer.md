# External Live Metrics Buffer for 1Hz Frontend Metrics

HardwareVisualizer receives 1Hz CPU, memory, GPU, and processor utilization updates for the main window. We decided to keep these high-frequency live metrics in an external Live Metrics Buffer instead of writing each tick through Jotai atoms, so React state updates are limited to components that intentionally subscribe to a metric channel.

Jotai remains the owner for settings, display selection, GPU selection, hardware information, and other low-frequency UI state. Usage Graphs and current-value displays read live metrics through explicit subscriptions, and chart renderers may subscribe directly when React re-rendering is unnecessary. The Tray Widget is outside this frontend buffer because it already renders from the Core metrics stream without going through the main-window React tree.

This decision trades some state-management simplicity for lower steady-state rendering cost. Render-count regression tests should guard the boundary so future changes do not accidentally fan 1Hz updates back out through unrelated Dashboard, Usage, or CPU detail UI.
