# Spec: AMD Vermeer B0 CCD temperatures through Linux k10temp

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Draft — not implementation-ready |
| Scope | Candidate Linux x86-64 CCD temperatures for one physical Vermeer B0 AM4 package through an already bound kernel k10temp device. |
| Issue phase | [#2515](https://github.com/shm11C3/HardwareVisualizer/issues/2515) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | AMD [Ryzen 9 5950X specifications](https://www.amd.com/en/products/processors/desktops/ryzen/5000-series/amd-ryzen-9-5950x.html) and [Ryzen Processor Software Optimization](https://gpuopen.com/download/GDC_AMD_Ryzen_Processor_Software_Optimization.pdf#page=22), GDC 2022, slide 22. AMD [Family 19h Model 21h Revision B0 PPR](https://www.scribd.com/document/1050633015/PPR-for-AMD-Family-19h-Model-21h), 56214-B0 rev 3.05, §2.2.1 pp. 74, 76–79 and §4.2.1 p. 328. | Primary for exact Vermeer product identity, two physical CCDs, CPUID encoding and the AM4 package field. |
| S2 | [Ryzen 9 5950X raw CPUID](https://github.com/InstLatx64/InstLatx64/blob/a7c87bf859a71295dedab9629a9fbccae37b8301/AuthenticAMD/AuthenticAMD0A20F10_K19_Vermeer_CPUID1.txt#L1-L29), Git blob `901184c36c81fe8cfc0fd3da9cbe56c7ee8598ee`. | Independent exact identity observation. It includes signature `0x00A20F10` and AM4 package field `0x2` without CCD readings. |
| S3 | [5950X B0 Linux sensor report](https://github.com/lm-sensors/lm-sensors/issues/389), March 2022, original report on Manjaro Linux 5.16.7. | First-hand `k10temp-pci-00c3` observation with `Tccd1` at 30.5 °C and `Tccd2` at 31.5 °C. It supplies decoded output without per-sample validity or canonical sysfs paths. |
| S4 | [Separate 5950X B0 Linux boot record](https://paste.cachyos.org/p/1a62637.log), Linux 7.1.6-1-cachyos. | First-hand family 19h model 21h stepping 0 and PCI function `0000:00:18.3` with vendor/device `1022:1443`. This boot record does not supply the CCD temperature samples in S3. |
| S5 | Linux v6.18 [hwmon sysfs ABI](https://docs.kernel.org/6.18/hwmon/sysfs-interface.html#temperatures) and [CPU topology](https://docs.kernel.org/6.18/admin-guide/cputopology.html). | Primary for read-only temperature attributes, millidegrees Celsius, and kernel-reported present CPUs and physical package IDs. |
| S6 | Linux [v6.18 k10temp](https://github.com/torvalds/linux/blob/v6.18/drivers/hwmon/k10temp.c), [v7.3 k10temp](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/k10temp.c), and v7.4-rc2 [k10temp source](https://github.com/torvalds/linux/blob/v7.4-rc2/drivers/hwmon/k10temp.c), Git blob `03f849bb958d72ed6f69d43196422b46fbd41407`. See the v7.4-rc2 [per-read branch](https://github.com/torvalds/linux/blob/v7.4-rc2/drivers/hwmon/k10temp.c#L250-L258), [probe filter](https://github.com/torvalds/linux/blob/v7.4-rc2/drivers/hwmon/k10temp.c#L412-L432) and [PCI ID definition](https://github.com/torvalds/linux/blob/v7.4-rc2/include/linux/pci_ids.h#L575). | **lead-only (copyleft)** for PCI driver matching, PCI-node ownership, published channel numbering, probe-only validity and post-probe decoding. |
| S7 | Linux v7.4-rc2 [AMD node read path](https://github.com/torvalds/linux/blob/v7.4-rc2/arch/x86/kernel/amd_node.c#L81-L130) and [hwmon device registration](https://github.com/torvalds/linux/blob/v7.4-rc2/drivers/hwmon/hwmon.c). | **lead-only (copyleft)** for the kernel-owned indexed read, transport error handling, and canonical hwmon parent relationship. |
| S8 | [Vermeer CCD Windows specification](cpu-amd-vermeer-ccd-smn.md), rev 5, Detection and Read procedure. | Project requirement to check the validity bit on each CCD sample. The Windows transport is separate from this Linux path. |

## Detection

These conditions describe a candidate scope for a future qualified kernel
ABI. The current scope is disabled.

| Condition | Gate kind | Source |
| --- | --- | --- |
| CPUID vendor `AuthenticAMD`, full signature `0x00A20F10`, and CPUID leaf `0x80000001` EBX bits 31:28 equal to `0x2` for AM4. | mechanism | S1–S2 |
| Complete kernel CPU topology identifies one physical package. An unreadable package ID, an incomplete present CPU set, or a changing snapshot cannot qualify the sample. | mechanism | S1, S5 |
| One canonical PCI function 3 with vendor/device `1022:1443` is already bound to the kernel PCI `k10temp` driver. The canonical `k10temp` hwmon child belongs to that PCI device. Resolve both the hwmon class entry and its device association to verify the child and parent separately. | safety | S4–S7 (lead-only for driver and child registration) |
| The running kernel is positively identified as providing per-read CCD validity on this binding. A registered attribute or successful decimal read cannot establish this property. | mechanism | S6 (lead-only for the missing check), S8 |

The PCI ID is shared with other silicon and cannot substitute for CPUID.
The observed PCI address and sensor names shown by libsensors are
evidence identities. A single NUMA node does not establish one physical
package. The admitted single package, if proven, is the only package to
which readings from its qualified endpoint can be attributed (S4–S7).

## Published attributes and decode

| Attribute on the same canonical child | Observed meaning | Source |
| --- | --- | --- |
| `name` | Driver chip name `k10temp`. | S5, S6 (lead-only for the binding) |
| `temp3_label` and `temp3_input` | `Tccd1` and its signed decimal input in millidegrees Celsius. | S3, S5, S6 (lead-only for the exact pairing) |
| `temp4_label` and `temp4_input` | `Tccd2` and its signed decimal input in millidegrees Celsius. | S3, S5, S6 (lead-only for the exact pairing) |

The driver uses bit 11 to determine whether to publish a CCD channel at
probe. On subsequent reads of a published channel in the checked
versions, a transport error can fail the read. The ordinary successful
path decodes bits 10:0 at 125 m°C per increment with a −49,000 m°C
offset and does not check bit 11 again (S6–S7, lead-only).

For example, synthetic words `0x0AC8` and `0x02C8` both produce
40,000 m°C. The first has bit 11 set and the second has it clear.
Label, channel presence and plausibility checks cannot distinguish these
samples (S6, lead-only). This example does not represent a captured
invalid hardware sample.

## Read procedure and safety

A future qualified route can use bounded reads of the standard hwmon
name, labels and inputs on demand. The class index `hwmonN` can change.
Keep each matched label/input pair on its validated child and retain
the CCD slot indicated by the exact label. Missing channels leave holes.
A missing or unavailable first slot leaves a valid second slot in its
original position. Existing package temperature collection remains
independent (S3, S5–S6 and S8).

The already bound kernel driver owns any SMN selector and data access.
The application route is confined to standard read-only sysfs attributes
and read-only CPU/PCI identity metadata. Raw PCI/SMN, debugfs, MSR access,
module operations, and hardware control are outside this scope
(S5–S7, lead-only for the kernel mechanism).

## Open questions

- Blocking. Identify a released kernel and a reliable runtime
  qualification rule that establish per-read invalid-sample reporting for
  the candidate. An unavailable error such as `ENODATA` on the same
  `tempX_input` read could satisfy this property. The checked releases
  through v7.4-rc2 use the probe-only check (S6).
- Pin a complete single-package CPU topology and canonical binding
  contract for the qualified runtime. Validate that a missing,
  malformed, duplicated or foreign endpoint cannot yield a CCD
  reading. Cover an invalid sample after successful channel discovery,
  independently valid sibling output, and incomplete or changing
  package topology in synthetic fixtures. S3 and S4 corroborate
  distinct parts of the candidate and do not establish that contract
  together in one capture.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-10 | Record the exact Vermeer B0 candidate and the post-probe CCD validity blocker in Linux k10temp. |
