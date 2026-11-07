# Hardware Compatibility Reports

Open **Settings → About → Hardware Compatibility Report** or use the report
button beside the Hardware Report export. Review the Markdown preview. You can
save it locally, or copy it and open the
[public GitHub issue form](https://github.com/shm11C3/HardwareVisualizer/issues/new?template=hardware_compatibility_report.yml).
Paste the report, add an optional note and submit it yourself. Reports with
working sensors are welcome.

The report includes model names, provider outcomes available to the app and
whether readings were available. It uses inventory and sensor results already
available to the app. **Not recorded** means the app does not have evidence for
the field. Missing readings may reflect startup or a transient gap in sampling.
For smartctl, the report shows any recorded guidance separately
from whether Storage Health records were available. For Super I/O, the report
includes the model or family already identified in a regular sensor reading.
A reading can be available through a fallback even when the primary provider
is unavailable. The app generates the report locally and opens the form
without including report data in the URL.

## Compatibility table

**Reported working** applies to the stated sensor path on the reported
configuration. **Experimental** identifies an implemented path with limited
verification. **Not yet verified** means this table lacks a working report for
the scope shown. A report for one model does not establish coverage for a whole
vendor or family.

| Hardware | Sensor path | Status | Dump wanted | Contributor |
| --- | --- | --- | --- | --- |
| AMD Family 17h | PawnIO package temperature | Not yet verified by a public compatibility report | No | — |
| AMD Family 19h | PawnIO package temperature | Reported working on a Ryzen 7 7800X3D ([validation record](../development/sensor-handoff/evidence/2026-09-26-pawnio-modules-0.2.11-validation.md)) | No | [@shm11C3](https://github.com/shm11C3) |
| AMD Family 1Ah | PawnIO package temperature | Experimental | No, report availability first | — |
| Intel CPUs | PawnIO package temperature | Not yet verified by a public compatibility report | No | — |
| NVIDIA GPUs | NVAPI on Windows | Not yet verified by a public compatibility report | No | — |
| AMD GPUs | ADL on Windows, DRM/hwmon on Linux | Not yet verified by a public compatibility report | No | — |
| Intel GPUs | intel_gpu_top on Linux | Not yet verified by a public compatibility report | No | — |
| Nuvoton NCT6799D | Windows motherboard temperatures and fans | Reported working on one configuration ([validation record](../development/sensor-handoff/evidence/2026-09-26-pawnio-modules-0.2.11-validation.md)) | No | [@shm11C3](https://github.com/shm11C3) |
| Nuvoton NCT677x / NCT679x | Windows motherboard temperatures and fans on recognized experimental profiles | Experimental | Yes, especially when temperatures or fans are missing | — |
| ITE IT8728F/EX | Windows motherboard temperatures | Experimental | Yes, especially when temperatures are missing | — |
| Nuvoton NCT668x | Windows motherboard temperatures and fans | Not yet verified | Yes | — |

When a report is added to the table, link its public evidence and credit the
contributor by GitHub login.

## Dump wanted

The table marks the families for which a hardware dump would help. Start with a
compatibility report. If a maintainer requests a dump, follow the separate
[hardware validation instructions](../development/sensor-handoff/02-hardware-validation.md)
for the [clean-room sensor work](https://github.com/shm11C3/HardwareVisualizer/issues/1635).
Keep raw dumps separate from the compatibility report.
