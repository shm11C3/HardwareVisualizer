# Windows Sensor Service Release Notes

Prepared copy for the release introducing the service; this is not a published
release announcement. [Japanese copy](windows-sensor-service.ja.md).

The Windows MSI now includes the sensor service during new installation and
updates under Program Files, including unattended updates. With PawnIO and
supported modules installed, CPU temperature/power and motherboard readings can
be collected while HardwareVisualizer runs with ordinary user permissions.
Installation and repair still require administrator approval. PawnIO remains a
separate optional driver; the update does not newly install it without consent.

Existing **Run as administrator on startup** preferences are preserved. If you
enabled it only for those sensors, turn it off in **Settings → Advanced**, exit
and launch normally. Some other privileged readings, including some storage
health data, may become unavailable. Changing the preference does not lower the
current process's privileges. User-created highest-privilege scheduled tasks
and compatibility settings must be changed separately by their owner.

The NSIS/portable app can use an already installed compatible service, but
service installation requires the protected MSI. Without it, available
monitoring and fallbacks continue. Successful upgrades do not show a migration
CTA. If an attempted sensor collection and its fallbacks still leave visible
readings unavailable, the existing failure notice links to sensor settings.
The settings show service connectivity and link to standard MSI recovery
instructions when the service is unavailable.
