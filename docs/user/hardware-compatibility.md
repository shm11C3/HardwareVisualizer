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
| AMD Matisse B0 | Windows PawnIO [two CCD temperatures on single-package AM4 with signature `0x00870F10`](../specs/sensors/cpu-amd-matisse-ccd-smn.md) | Experimental | No | — |
| AMD Castle Peak B0 | Windows PawnIO [eight CCD slots on single-package sTRX4 with signature `0x00830F10`](../specs/sensors/cpu-amd-castle-peak-ccd-smn.md) | Experimental | No | — |
| AMD Vermeer | Windows PawnIO [CCD temperatures on qualified single-package AM4 models](../specs/sensors/cpu-amd-vermeer-ccd-smn.md) | Experimental | No | — |
| AMD Raphael | Windows PawnIO [CCD temperatures on single-package AM5 with signature `0x00A60F12`](../specs/sensors/cpu-amd-raphael-ccd-smn.md) | Experimental | No | — |
| AMD Granite Ridge | Windows PawnIO [CCD temperatures on single-package AM5 with signature `0x00B40F40`](../specs/sensors/cpu-amd-granite-ridge-ccd-smn.md) | Experimental | No | — |
| Intel CPUs | PawnIO package temperature | Not yet verified by a public compatibility report | No | — |
| Intel Core DTS | Windows PawnIO core temperatures on [selected legacy Core and Atom models, plus Alder Lake and Raptor Lake profiles](../specs/sensors/cpu-intel-core-dts-msr.md) | Includes DTS-only CPUs. The specification lists the status of each profile. | No | — |
| Intel Skylake U, U23e, and Y | Windows PawnIO [core temperatures with signature `0x000406E3`](../specs/sensors/cpu-intel-skylake-mobile-dts-msr.md) | Experimental | No | — |
| Intel Meteor Lake C0 | Windows PawnIO [core temperatures with signature `0x000A06A4`](../specs/sensors/cpu-intel-meteor-lake-dts-msr.md) | Experimental | No | — |
| Intel Arrow Lake 200S, 200S Plus, 200HX, and 200HX Plus | Windows PawnIO [core temperatures with signature `0x000C0662`](../specs/sensors/cpu-intel-arrow-lake-dts-msr.md) | Experimental | No | — |
| NVIDIA GPUs | NVAPI on Windows | Not yet verified by a public compatibility report | No | — |
| NVIDIA GPUs | Linux NVML usage, temperature, and dedicated VRAM usage for DRM adapters and PCI display functions bound to the NVIDIA driver | Implemented. PCI-only readings require an active runtime state. Native Linux compatibility report not yet recorded. | No | — |
| AMD GPUs | ADL on Windows, DRM/hwmon on Linux | Not yet verified by a public compatibility report | No | — |
| Intel GPUs | intel_gpu_top on Linux | Not yet verified by a public compatibility report | No | — |
| Nuvoton NCT6799D | Windows motherboard temperatures and fans | Reported working on one configuration ([validation record](../development/sensor-handoff/evidence/2026-09-26-pawnio-modules-0.2.11-validation.md)) | No | [@shm11C3](https://github.com/shm11C3) |
| Nuvoton NCT677x / NCT679x | Windows motherboard temperatures and fans on [recognized profiles](../specs/sensors/superio-nuvoton-nct67xx.md) | Experimental | Yes, especially when temperatures or fans are missing | — |
| ITE IT8728F/EX | Windows motherboard temperatures and [FAN1–2 on the qualified GA-B75M-D3H profile](../specs/sensors/superio-ite-it8728f-ex-ga-b75m-d3h-fans.md) | Experimental | Yes, especially when readings are missing | — |
| ITE IT8620E, IT8628E, IT8716F D, IT8772E FX | Windows [primary LPC profiles](../specs/sensors/superio-ite-primary-lpc-expansion.md) for TMPIN1–3 and FAN1–3 on IT8620E and IT8628E, FAN1–3 on IT8716F D, and FAN2–3 on IT8772E FX | Experimental for qualifying channels | Yes, especially when readings are missing | — |
| MSI PRO B850-P WIFI, MAG B850M MORTAR WIFI, and MPG X870E CARBON WIFI | Linux [NCT6687-family hwmon profiles](../specs/sensors/superio-nuvoton-nct6687-linux-hwmon.md) for temperatures and fans on PRO, SYS_FAN1 on MORTAR, and one diode temperature and two fan inputs on CARBON | Experimental with the already bound native driver | No, report availability first | — |
| HARDKERNEL ODROID-H4 | Linux [DDR5 DIMM temperature for the exact Dasharo firmware and Samsung SO-DIMM profile](../specs/sensors/memory-ddr5-odroid-h4-spd5118.md) using a bound SPD5118 hub | Experimental when the live DIMM identity matches | No, report availability first | — |

When a report is added to the table, link its public evidence and credit the
contributor by GitHub login.

## Dump wanted

The table marks the families for which a hardware dump would help. Start with a
compatibility report. If a maintainer requests a dump, follow the separate
[hardware validation instructions](../development/sensor-handoff/02-hardware-validation.md)
for the [clean-room sensor work](https://github.com/shm11C3/HardwareVisualizer/issues/1635).
Keep raw dumps separate from the compatibility report.
