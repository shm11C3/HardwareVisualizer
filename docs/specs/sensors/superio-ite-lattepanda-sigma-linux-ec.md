# Spec: LattePanda Sigma ITE EC temperatures and fan through Linux hwmon

| Field | Value |
| --- | --- |
| Revision | 2 |
| Status | Draft — not implementation-ready |
| Scope | Experimental read-only board and CPU-proximity temperatures and CPU fan RPM through the already bound native LattePanda Sigma EC Linux hwmon endpoint. |
| Issue phase | [#2517](https://github.com/shm11C3/HardwareVisualizer/issues/2517) |

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | DFRobot [LattePanda Sigma product documentation](https://www.lattepanda.com/lattepanda-sigma), [BIOS firmware](https://docs.lattepanda.com/content/sigma_edition/BIOS_Firmware/#lp-bs-wtadlc1r210-32g-a) and [separate optional EC firmware](https://docs.lattepanda.com/content/sigma_edition/EC_Firmware/#lp-ec-wtadlc1r210_flashupdate2) | OEM identification of the Sigma board, processor and cooling configuration. The BIOS and EC have separate update artifacts. |
| S2 | DFRobot [LP-BS-WTADLC1R210-32G-A factory BIOS ZIP](https://github.com/LattePandaTeam/LattePanda-Win10-Software/blob/master/BIOS%20for%20LattePanda%20Sigma/32GB%20version/LP-BS-WTADLC1R210-32G-A.zip), June 13, 2023, SHA-256 `3b76f4243d2925abe10202488b81d985d678bd6a80ac1809b534e434060a761a`. ZIP member `BIOS/LP-BS-WTADLC1R210-32G-A.bin`, SHA-256 `6b2d8b484c2ee6d01140dc56ebbd51c212e2f2fbe781e5fd6ae905833d88a0de`. [Independent firmware inventory](https://browser.geekbench.com/v6/cpu/baseline/3782776). | Static OEM ACPI evidence below. The independent inventory reports this 32 GB build with AMI version 5.27. |
| S3 | Linux [LattePanda Sigma EC hwmon ABI](https://docs.kernel.org/hwmon/lattepanda-sigma-ec.html), [source documentation at v7.2](https://github.com/torvalds/linux/blob/v7.2/Documentation/hwmon/lattepanda-sigma-ec.rst), Git blob `8a521ee1fef1e1c4704eb40537f7cb1c4ebd68ed). [Original author hardware observation](https://lkml.iu.edu/2602.3/10244.html) and [ACPI analysis](https://lkml.iu.edu/hypermail/linux/kernel/2603.0/02464.html). | Public Linux ABI, IT8613E attribution, channel labels and observed BIOS version. First-hand observation verifies the fan reporting zero at rest. |
| S4 | Linux [v7.1](https://github.com/torvalds/linux/blob/v7.1/drivers/hwmon/lattepanda-sigma-ec.c) and [v7.2](https://github.com/torvalds/linux/blob/v7.2/drivers/hwmon/lattepanda-sigma-ec.c), Git blob `f060974222019f548fc835f00410a550334e6814`. [Transactions and bounds](https://github.com/torvalds/linux/blob/v7.2/drivers/hwmon/lattepanda-sigma-ec.c#L35-L159), [channels](https://github.com/torvalds/linux/blob/v7.2/drivers/hwmon/lattepanda-sigma-ec.c#L162-L241), [probe and binding](https://github.com/torvalds/linux/blob/v7.2/drivers/hwmon/lattepanda-sigma-ec.c#L256-L342). | **lead-only (copyleft)** for exact platform device and driver names, DMI and force handling, exclusive Linux I/O reservations, probe read and complete steady-state input callbacks. |
| S5 | Linux v7.2 [platform device naming](https://github.com/torvalds/linux/blob/v7.2/drivers/base/platform.c#L704-L725) and [hwmon attribute serialization](https://github.com/torvalds/linux/blob/v7.2/drivers/hwmon/hwmon.c#L426-L466) and [hwmon parent assignment](https://github.com/torvalds/linux/blob/v7.2/drivers/hwmon/hwmon.c#L973-L987), hwmon Git blob `29dc90a2c3fe7086b80d81e3972be5b3f3dbdac3`. | **lead-only (copyleft)** for kernel-owned hwmon topology and serialization within the registered hwmon endpoint. |
| S6 | Linux v7.2 [module naming](https://github.com/torvalds/linux/blob/v7.2/scripts/Makefile.lib#L17-L23), [loadable module parameter registration](https://github.com/torvalds/linux/blob/v7.2/kernel/module/sysfs.c#L337-L381), [read-only boolean display](https://github.com/torvalds/linux/blob/v7.2/kernel/params.c#L320-L331), [parameter directory](https://github.com/torvalds/linux/blob/v7.2/kernel/params.c#L630-L641) and [built-in parameter visibility](https://github.com/torvalds/linux/blob/v7.2/kernel/params.c#L781-L837). | **lead-only (copyleft)** for the native `force` parameter namespace and read-only value on loadable and built-in paths with sysfs support. |
| S7 | UEFI Forum [ACPI 6.6, Embedded Controller Interface](https://uefi.org/specs/ACPI/6.6/12_Embedded_Controller_Interface_Specification.html), §§12.1–12.3, 12.7–12.8 | Primary for private versus shared interfaces, status polling, response consumption, read command versus configuration or event-query commands, and firmware arbitration requirements. |
| S8 | Linux v7.2 [ACPI EC namespace resource requirement](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/ec.c#L1457-L1488), [late I/O reservation behavior](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/ec.c#L1723-L1746), [early ECDT setup](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/ec.c#L2018-L2088) and [driver name](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/ec.c#L2263-L2271), Git blob `64ad4cfa6208bd3d1f8f606004ed120170a0638d`. [Table instance names](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/sysfs.c#L342-L377), [static and dynamic table directories](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/sysfs.c#L514-L541), [ACPI device status attribute](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/device_sysfs.c#L532-L545). | **lead-only (copyleft)** for passive conflict detection through Linux metadata, the independent ECDT path and the reason the ACPI device status attribute is excluded. |
| S9 | Rui Carmo, [first-hand LattePanda Sigma Linux report](https://taoofmac.com/space/reviews/2026/09/13/1700), September 13, 2026, thermal and software sections | Corroborates 1497 RPM, board 36 °C, CPU proximity 43 °C from the named endpoint. The article identifies Fedora Silverblue 44 and Linux 7.1.5 in its software account. It does not publish the sample's force setting or BIOS and EC identifiers. |
| S10 | Linux v7.2 [early namespace EC setup](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/ec.c#L1807-L1852), [unbind](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/ec.c#L1755-L1765), [early `_STA` eligibility](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/acpica/nsxfeval.c#L669-L740), [later platform eligibility](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/scan.c#L2535-L2540) and [namespace device registration](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/scan.c#L1861-L1913). | **lead-only (copyleft)** for the namespace boot EC lifetime, including cases without a platform device or bound driver. |
| S11 | Linux v7.2 [initrd table replacement and addition](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/tables.c#L545-L649), [physical and built-in override](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/tables.c#L668-L698), [ACPICA override completion](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/acpica/tbinstal.c#L201-L264), [boot namespace and EC setup order](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/bus.c#L1472-L1530), and [taint ABI](https://www.kernel.org/doc/html/latest/admin-guide/tainted-kernels.html). | **lead-only (copyleft)** for the scope of the ACPI override taint and load order. |
| S12 | Linux v7.2 [SSDT configfs load](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/acpi_configfs.c#L32-L61), [ACPICA dynamic load](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/acpica/tbxfload.c#L307-L339) and [ACPI binary table permissions](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/sysfs.c#L364-L377). | **lead-only (copyleft)** for the runtime table load path and the permissions on loaded table bytes. |

## Detection and scoped enablement

| Condition | Gate kind | Source |
| --- | --- | --- |
| Exact DMI system vendor `LattePanda` and product `LattePanda Sigma` select this board's EC protocol and channel assignment. | mechanism | S1, S3, S4 (lead-only DMI binding) |
| Exactly one already bound native `lattepanda_sigma_ec` platform endpoint with a canonical same-parent hwmon child. Use only its documented label and input pairs. | mechanism | S3, S4/S5 (lead-only topology) |
| The kernel's readable `force` parameter is `N`, and passive metadata shows no known ACPI competitor. These are necessary checks. Exclusion of a surviving boot EC handler is unresolved. | safety | S4/S6/S8/S10–S12 (lead-only Linux behavior), S7 |

| Scope | Status | Default enablement |
| --- | --- | --- |
| LattePanda Sigma board EC with unforced native Linux platform hwmon | S1 identifies the platform, S2 supplies the ACPI observations for factory 32G-A and AMI 5.27, and S3 records ITE IT8613E and BIOS 5.27 as the driver's observed scope. S4–S6/S8 (lead-only) establish the binding, protocol, arbitration metadata and callbacks. S9 corroborates all three channels independently. Firmware versions are evidence identities. | Disabled pending a passive boot EC ownership gate. |
| Matching LattePanda Sigma with a missing, unqualified, forced, unreadable or competing endpoint | The platform matches, but the native endpoint or its safety conditions cannot be established. | Unavailable. |
| Other IT8613E profiles, other boards or other mechanisms | This is a board-specific native Linux ABI. | Unsupported by this revision. |

### Candidate endpoint and passive conflict checks

These checks describe a candidate gate. The unresolved boot EC path
below prevents reading any inputs under this revision.

- Resolve `/sys/bus/platform/devices/lattepanda_sigma_ec` to the
  native `/sys/devices/platform/lattepanda_sigma_ec` device. Its
  `subsystem` and `driver` links must resolve to `/sys/bus/platform`
  and `/sys/bus/platform/drivers/lattepanda_sigma_ec`, respectively.
  Require one `/sys/class/hwmon/hwmonN` entry named
  `lattepanda_sigma_ec` under that same device's `hwmon` directory,
  with the class child's `device` link pointing to that parent.
  Deduplicate aliases to the identical child. Refuse a name-only
  match, conflicting children, foreign files or a different
  driver. Hardware paths and the driver binding must be kernel-owned
  (S3–S5). The native driver has no numeric platform-device suffix.
- Read the kernel parameter
  `/sys/module/lattepanda_sigma_ec/parameters/force`. The pinned
  driver exposes it read-only. Linux displays its boolean state as
  `N` or `Y` followed by a newline. Require `N` from the native
  parameter. Missing, unreadable, malformed or `Y` is unavailable
  (S4/S6, lead-only).
- Before reading inputs, enumerate the kernel-owned
  `/sys/firmware/acpi/tables` and its `dynamic` directory. Refuse
  an `ECDT` entry or that signature followed by a numeric instance
  suffix. Check the existing `/sys/bus/platform/drivers/acpi-ec`
  binding links. Refuse any bound platform device owned by that
  driver. Use directory entries and canonical links only. A missing
  directory, unreadable metadata or an incomplete check is
  unavailable. An ECDT can initiate generic ACPI EC access before
  namespace enumeration. The generic driver can continue after a
  Linux port reservation warning, so the Sigma driver binding by
  itself is insufficient to exclude this competing path (S7/S8).
  An unbound ACPI EC platform device is also reason to refuse the
  candidate. Absence of that device is insufficient (S10).
- Do not read the ACPI device `status` attribute or evaluate any AML
  to establish ownership. The Linux status read evaluates `_STA`
  (S8, lead-only). The audited Sigma `_STA` has a side effect even
  though it returns zero (S2). The app does not read table bodies or
  inspect hardware registers at runtime. Recheck safety and ownership
  before using a changed endpoint. Discard affected samples if a
  conflict or binding change becomes visible during collection.

### Unresolved boot EC arbitration

Without ECDT, Linux v7.2 can install a boot EC handler from a namespace
object matching `PNP0C09` via its HID or CID and supplying the I/O
resources. The early walk accepts `_STA` with either Present or
Functioning set. Later platform enumeration requires Present, so an
early handler can exist without a platform device. When a platform
device is present and subsequently unbound, removal releases the I/O
reservations and retains the boot handler. The Sigma driver's binding
and its own serialization do not cover that handler (S4/S5/S8/S10).

The audited OEM `H_EC` remains a namespace object even when it
reports zero. Its identity alone cannot establish whether the live
namespace's resources and status match those in S2. Reading the
ACPI status attribute would evaluate `_STA`. The ACPI override bit
(bit 8) in `/proc/sys/kernel/tainted` is set by built-in custom DSDT
replacement. Initrd physical replacement and addition do not set it
in v7.2, and their tables can be present at the early namespace
walk. The configfs SSDT load path also does not set it. An unset bit
cannot qualify the endpoint (S8/S10–S12).

A hash comparison against the recovered DSDT alone would leave boot
SSDTs unexamined. Linux v7.2 exposes loaded table contents with mode
`0400`, so the standard unprivileged collector cannot use those
bodies to qualify the live AML. No reliable passive witness for
boot EC arbitration has been established. This revision authorizes
no Sigma EC input reads (S11/S12).

### OEM ACPI evidence

In the S2 image's standard LZMA section at `0x5F20C0`, the
checksum-valid DSDT at decoded offset `0xA98E64` is 602,629 bytes,
OEM ID `ALASKA`, table ID `A M I`, SHA-256
`d2e3be159d4965ee573020d41d02f5e3f9ce84b28d3b2358950bb91529b17eb0`.
The `H_EC` object under the LPC bridge is the sole `PNP0C09` in
that DSDT. It supplies neither an EC resource method nor a Global
Lock method. Its `_STA` returns zero after assigning a graphics
classification field. Its `ECRD` returns zero for all cases and
`ECWT` has no EC I/O. The bounded decode recovered that DSDT and 80
checksum-valid SSDTs. It found no valid ECDT or constant
`0x62`/`0x66` EC SystemIO declaration or standard port resource
descriptor in those tables. The one `ECDT` token in a signature
catalog has no valid table header. These are facts about the
recovered tables in that artifact.

The OEM separately distributes an EC firmware update (S1).
The observed ABI and recovered tables describe the candidate. A known
independent port owner, custom or forced binding, or conflicting ACPI
route rules it out. The artifacts do not attest the separate EC
firmware image, the absence of an SMM owner or the live boot EC
handler state.

## Input facts and read safety

These callback facts apply to a future qualified endpoint.

| Attribute | Required label | Meaning and units | Source |
| --- | --- | --- | --- |
| `temp1_input` | `Board Temp` | Board or ambient temperature, integer m°C. EC byte `0x60` is unsigned whole °C. | S3, S4 (lead-only callback) |
| `temp2_input` | `CPU Temp` | CPU-proximity temperature, integer m°C. EC byte `0x70` is unsigned whole °C. This is a motherboard sensor. | S3, S4 (lead-only callback) |
| `fan1_input` | `CPU Fan` | Direct integer RPM from the EC big-endian word at `0x2E:0x2F`. Zero was observed when the fan stopped. | S3, S4 (lead-only callback), S9 |

1. Read the corresponding `temp1_label`, `temp2_label` and
   `fan1_label` metadata and only the three named input attributes
   from the proven child. The label callbacks return constant
   strings without port access (S4, lead-only). A missing, wrong or
   unreadable label excludes its channel. Keep valid siblings.
2. For each admitted input, the kernel polls its command/status
   port `0x66` for IBF/OBF and transfers the ACPI read command
   `0x80`, the address byte and the response using `0x62` for
   address and response. The driver has bounded status waits.
   Reading the response acknowledges the output buffer flag as part
   of that transaction. The input callbacks issue no EC write,
   burst or event-query command and access no alarm, duty or
   configuration register. The RPM read takes high, low, high and
   refreshes low once if high changed. The kernel's hwmon lock
   serializes this access with other reads through the same child.
   It provides no arbitration for SMM or independent port clients
   (S4/S5/S7).
3. The native driver reserves both ports before registering hwmon
   and makes a one-time probe read of fan duty at `0x93` (S4,
   lead-only). The application does not request module loading,
   rebinding, probe or resource overrides. It does not issue port
   commands, AML queries, LPC configuration keys or sensor writes.
   No direct or generic fallback read is authorized for a refused
   endpoint.
4. Require complete decimal samples. For temperature accept
   1,000–125,000 m°C in whole 1,000 m°C steps, then convert to °C.
   For fan accept 0–65,534 RPM. Map a valid zero to an Inactive Fan
   Reading. S3 observed zero when the fan stopped. Zero reports no
   rotation and does not establish the fan's physical state.
   Reject all-ones RPM and out-of-range temperature as conservative
   project plausibility checks. No failed or missing input becomes
   zero. Use the existing motherboard temperature and fan paths,
   retaining independently available readings and existing CPU
   package telemetry.

## Synthetic fixture acceptance

These cases describe a future qualified scope. A matching Sigma stays
unavailable under this Draft, without reading inputs.

| Case | Required result |
| --- | --- |
| Native board and proven ownership including boot EC exclusion, force `N`, exact labels, values `36000`, `43000`, `1497` | Board 36 °C, CPU proximity 43 °C and CPU fan 1497 RPM. No extra input or control reads. |
| One wrong, missing or unreadable label or malformed/failed input, with independently valid siblings | Omit only the affected channel. Do not infer a value or relabel CPU-proximity temperature as CPU package. |
| DMI identifies a different board, or a different mechanism is in use | Unsupported for this profile. No input read. |
| Matching Sigma with absent endpoint, foreign driver, matching hwmon name at a false class path, competing canonical children or changed driver/device link | Unavailable. Do not read unqualified inputs. Discard samples from a changed binding and qualify again before reuse. |
| Matching Sigma with force `Y`, missing or unreadable force parameter, or inaccessible static/dynamic ACPI or driver metadata | Unavailable. No input reads, AML invocation, ACPI status attribute read or module activation. |
| Matching Sigma with `ECDT`, `ECDT1` or another numeric instance in either table directory, or an ACPI EC platform device, bound or unbound | Unavailable. No input reads, even if the native Sigma endpoint and labels are otherwise valid. |
| Temperatures `0`, `1000`, `125000`, `125001` and `255000` m°C. Fan `-1`, `0`, `65534` and `65535` RPM. | Accept only `1000` and `125000` m°C, and 0 and 65,534 RPM. Report a valid zero as an Inactive Fan Reading, distinct from unavailable. |
| No ECDT or ACPI EC platform device, taint bit 8 clear, but an initrd-supplied namespace EC installed a boot handler before platform enumeration or unbind | Unavailable. No input reads. Absence of a binding or a reservation is insufficient. |
| OEM-shaped ACPI namespace with only the expected `H_EC` identity, but no passive evidence of its early EC state | Unavailable. Do not evaluate `_STA` or infer safety from the namespace path. |

## Open questions

- Blocking for this candidate: a stable passive, unprivileged
  witness that excludes an active or retained boot ACPI EC handler
  on the Sigma port pair, or an OS-owned path that arbitrates the
  Sigma transactions with that handler. A BIOS inventory string
  cannot establish live ACPI ownership.
- A future eligible input requires the same-child label and a valid sample.
  A maintainer-accepted capture with the exact firmware and force
  state, canonical links and all three values is still needed for
  Verified coverage.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-09 | Propose Implementation-ready status for the exact Experimental LattePanda Sigma native Linux EC endpoint with passive overlap exclusion and independent board, CPU-proximity and fan readings. |
| 2 | 2026-11-09 | Return to Draft and disable the candidate. Linux v7.2 can retain a boot EC handler without a visible driver binding or port reservation, and no passive unprivileged ownership gate is established. |
