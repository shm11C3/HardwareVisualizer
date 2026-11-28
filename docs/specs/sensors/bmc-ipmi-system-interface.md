# Spec: Local BMC temperatures and fan speeds through the IPMI system interface

| Field | Value |
| --- | --- |
| Revision | 1 |
| Status | Implementation-ready (rev 1) |
| Scope | Experimental, read-only BMC temperatures and fan RPM from threshold-based Full Sensor Records in the BMC's SDR Repository. The host reaches the BMC only through the operating system's own IPMI driver: on Windows the in-box Microsoft IPMI driver's `Microsoft_IPMI.RequestResponse` WMI method, called from the LocalSystem sensor service; on Linux the `ipmi_devintf` character device. The host sends four IPMI commands, all of them Get commands: Get Device ID, Get SDR Repository Info, Get SDR at record offset 0, and Get Sensor Reading. It never reserves the repository and sends no other command. Compact and event-only records, discrete fan sensors, linearized and non-linear sensors, sensors not owned by the BMC at LUN 0, voltage, current and power sensors, the Windows IPMI Provider CIM classes, direct KCS/BT/SSIF port access, OEM commands, the SEL, thresholds, remote (LAN, Redfish) access and every control command are out of scope or Unsupported. Every #2525 target has a disposition in the Target inventory. |
| Issue phase | [#2525](https://github.com/shm11C3/HardwareVisualizer/issues/2525) (parent [#2510](https://github.com/shm11C3/HardwareVisualizer/issues/2510)) |

The record formats, the commands, the completion codes and the reading
conversion are defined by the public IPMI v2.0 specification (S1). The
Windows transport is defined by Microsoft's documentation (S2–S6). The
Linux transport's public description is the kernel's driver
documentation (S8). Its numeric ABI (ioctl request codes, structure
layouts, address-type constants) is taken from the kernel UAPI headers and
is tagged lead-only under
[ADR 0023](../../adr/0023-copyleft-derived-facts-for-experimental-scopes.md)
(S9, S10). No BMC observation or hardware dump exists yet. Both enabled
scopes are therefore Experimental under
[ADR 0011](../../adr/0011-experimental-sensor-enablement.md). The synthetic
fixtures below test the software contract only.

## Sources

| ID | Source | Notes |
| --- | --- | --- |
| S1 | Intel, Hewlett-Packard, NEC, Dell, [*Intelligent Platform Management Interface Specification, Second Generation, v2.0*, Document Revision 1.1, October 1, 2013, April 21, 2015 E7 Markup](https://www.intel.com/content/dam/www/public/us/en/documents/specification-updates/ipmi-intelligent-platform-mgt-interface-spec-2nd-gen-v2-0-spec-update.pdf), file SHA-256 `76771741c30d64b1ec27140116f0e1131409329fbdeccf404e831371007ac10b`. PDF page numbers equal printed page numbers. Pinned: §5.1 Table 5-1 (p. 67), §5.2 Table 5-2 and §5.3 (pp. 68–72), §5.4 Table 5-3 and §5.5 Table 5-4 (pp. 72–73), §6.11 (pp. 80–81), §6.12.1 (p. 81), §9 to §12 (pp. 104–149), §20.1 Table 20-2 (pp. 270–272), §22.6 (p. 299), §22.8 (p. 305), §27.5 (p. 410), §33.9 Table 33-3 (p. 468), §33.11 to §33.12 Table 33-6 (pp. 469–471), §33.14 (p. 472), §35.2 to §35.4 (pp. 481–483), §35.12 (p. 490), §35.14 Table 35-15 (pp. 496–497), §36.1 to §36.4 (pp. 512–513), §42.1 Table 42-1 (pp. 532–533), §42.2 Table 42-3 (p. 535), §43 introduction (p. 550), §43.1 Table 43-1 (pp. 551–557), §43.14 Table 43-13 (p. 580), §43.2 (p. 558), §43.15 (pp. 581–582), §43.17 Table 43-15 (p. 584), Appendix C3 (pp. 600–606), Appendix G Table G-1 (pp. 619–621) | Primary for every IPMI fact: network functions, command numbers, request and response layouts, completion codes, the SDR Repository and reservation rules, the Full Sensor Record layout, sensor type, event/reading type and unit codes, the conversion formula, the ACPI `IPI0001` device, and the session-less system interface. |
| S2 | Microsoft Learn, [*Microsoft_IPMI class*](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ipmiprv/microsoft-ipmi) and [*RequestResponse method of the Microsoft_IPMI class*](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ipmiprv/microsoft-ipmi-requestresponse), archived pages, `ms.date` 2018-05-31, source commit `5a8ffea54220a2bd9f13c92c2a1df00b7f805a9e`; also [*RequestResponseEx*](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ipmiprv/microsoft-ipmi-requestresponseex) and [*SMS_Attention*](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ipmiprv/microsoft-ipmi-sms-attention) | Primary. Namespace `Root\wmi`, provider `WMIProv`, properties `BMCAddress` (uint8) and `InstanceName`, methods `RequestResponse`, `RequestResponseEx` and `SMS_Attention`, implemented by `IpmiDrv.sys`. `RequestResponse` parameters: in `Command`, `Lun`, `NetworkFunction`, `RequestData`, `RequestDataSize`, `ResponderAddress`; out `CompletionCode`, `ResponseData`, `ResponseDataSize`. The completion code is also the first byte of `ResponseData` and is counted in `ResponseDataSize`. Minimum client Windows Vista (method page) or Windows 8 (class page); minimum server Windows Server 2008. |
| S3 | Microsoft Learn, [*IPMI Provider*](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ipmiprv/ipmi-provider), archived page, `ms.date` 2018-05-31 | Primary. The IPMI driver is a WDM kernel-mode driver that communicates with the BMC using KCS and appears as "Microsoft SMBIOS Generic IPMI Compliant Device". The IPMI provider (`IPMIPrv`) is installed by default, obtains its data through the driver, and implements a subset of the IPMI CIM mapping. |
| S4 | Microsoft Learn, [*NumericSensor class*](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ipmiprv/numericsensor), archived page | Primary. Namespace `Root\Hardware`, provider `IPMIPrv`. `CurrentReading` is `sint32` in units of `BaseUnits` × 10^`UnitModifier`. The class also carries methods `Reset`, `RestoreDefaultThresholds`, `RequestStateChange`, `EnableDevice`, `OnlineDevice` and `QuiesceDevice`. Used only for the Deferred disposition. |
| S5 | Microsoft Learn archive, Steve Lee, [*In-Band Management of IPMI with PowerShell*](https://learn.microsoft.com/en-us/archive/blogs/wmi/in-band-management-of-ipmi-with-powershell), WMI team blog, 2012-03-22 | Microsoft-authored. The driver is a pass-through that takes IPMI requests and returns IPMI responses. The caller must be in the Administrators group to call `RequestResponse` successfully. LUN is usually 0 and `ResponderAddress` usually 32 (`20h`, the BMC). |
| S6 | Microsoft Learn, [*Required Items in WMI Classes*](https://learn.microsoft.com/en-us/windows-hardware/drivers/kernel/required-items-in-wmi-classes) | Primary. Every WDM WMI class instance carries `InstanceName` (key) and `Active` (read-only boolean), both supplied by WMI. |
| S7 | Spec-author host observation, 2026-11-29: Windows 11 Pro 10.0.26300, files `C:\Windows\INF\ipmidrv.inf` (SHA-256 `b91caea0812b1cf828c46e90aeccb4102e41c9168d9d7fb16e510c2fbb3072c2`, `DriverVer` 06/21/2006, 10.0.26100.9818) and `C:\Windows\INF\acpipmi.inf` (SHA-256 `e02389ca1dc1621887959c6c98786bd1d74e6cdd473b522e3af39022f517dbe5`, 10.0.26100.1150) | Observation of in-box Microsoft INF files on a client edition without a BMC. `ipmidrv.inf` installs service `IPMIDRV` (kernel driver, demand start) for hardware ID `ACPI\IPI0001` only, device name "Microsoft Generic IPMI Compliant Device". `acpipmi.inf` binds the ACPI power meter `ACPI\ACPI000D`. No BMC was present, so the driver was not loaded and nothing was read. |
| S8 | Linux kernel documentation [`Documentation/driver-api/ipmi.rst` at v7.3](https://github.com/torvalds/linux/blob/v7.3/Documentation/driver-api/ipmi.rst), Git blob `062c38d93fbb5a258f40b7245652008f4578da39`, file SHA-256 `50316d499bb1ee64ecc56a869e7167057ce960b2c3855da6b5f8dcd5cb5acc3c`: Basic Design, Addressing, Messages, Messaging, The SI Driver | Primary for the public driver interface: `ipmi_devintf` gives userland an ioctl interface, and each open file is a separate IPMI user of the message handler. A request is sent with `IPMICTL_SEND_COMMAND`. Readiness is awaited with `poll()`/`select()`, and the response is taken with `IPMICTL_RECEIVE_MSG` or `IPMICTL_RECEIVE_MSG_TRUNC`, never `read()`. The caller supplies the data buffer and its length. A message that does not fit fails with `EMSGSIZE` and stays queued, unless the truncating ioctl is used. The system-interface address type, with channel equal to the BMC channel, talks directly to the BMC. The response's first data byte is the completion code. Commands on the IPMB time out after the IPMI-specified 5 seconds with a generated timeout response. The local BMC's default address is `0x20`. |
| S9 | Linux UAPI headers [`include/uapi/linux/ipmi.h` at v7.3](https://github.com/torvalds/linux/blob/v7.3/include/uapi/linux/ipmi.h) (Git blob `966c3070959b932572d863d5ede6d3433ea85928`, SHA-256 `6b2049e867f6dba455eb11b8b59a7dfefa0ff30a2adbee6f9cc54cbfefeabceb`) and [`include/uapi/linux/ipmi_msgdefs.h` at v7.3](https://github.com/torvalds/linux/blob/v7.3/include/uapi/linux/ipmi_msgdefs.h) (Git blob `0934af3b8037a38fd2118a74bd3b98b31b588633`, SHA-256 `aa12b4d7c43241eff4d7eb62665dbdace8258ac00ce98a494e32ff2aaa28c7ae`), license `GPL-2.0+ WITH Linux-syscall-note` | **lead-only (copyleft)** for the numeric ABI restated in Linux transport: ioctl type `'i'` and numbers, structure field order and types, system-interface address type `0x0C`, BMC channel `0x0F`, response receive type `1`, maximum message length 272. Also for the header's statements that `read()` and `write()` do not work, that each caller receives only responses to its own commands, that asynchronous events can reach every open user, and the error values of the send and receive ioctls. |
| S10 | Linux [`drivers/char/ipmi/ipmi_devintf.c` at v7.3](https://github.com/torvalds/linux/blob/v7.3/drivers/char/ipmi/ipmi_devintf.c), Git blob `38d0f1bc10b0e6d46aea3bec2445a7c13879dfe2`, SHA-256 `41729e531a56bd83a14240fc7d47cbb344972b3b0a62bacde27e665215394423` | **lead-only (copyleft)**: device class `ipmi`, one node `ipmi<N>` per registered interface number `N`, `poll` support, and no capability check in the driver itself, so the node's file mode alone governs access. |
| S11 | Linux kernel documentation [`Documentation/hwmon/ibmaem.rst`](https://github.com/torvalds/linux/blob/v7.3/Documentation/hwmon/ibmaem.rst) (SHA-256 `6641e26df6029db5de74bc2b8df8c2a9b5781a30f71dcfe0adf456a2faa6c3a3`) and [`Documentation/hwmon/acpi_power_meter.rst`](https://github.com/torvalds/linux/blob/v7.3/Documentation/hwmon/acpi_power_meter.rst) (SHA-256 `24340eb4053739253d3c448bbe83acbcdabcab4f3991a285fd34174d8a0002e1`) at v7.3; [`drivers/hwmon/Kconfig` at v7.3](https://github.com/torvalds/linux/blob/v7.3/drivers/hwmon/Kconfig) entries for IBM AEM and PowerExecutive (SHA-256 `266062eb50098cabef60cf7f2f3cb30d889724b975c799a580ea86d699d63b03`) | Documentation is primary for the public ABI; Kconfig help text is **lead-only (copyleft)**. Used only for dispositions: `ibmaem` reads energy and power meters (and temperatures on the v2 interface) on IBM System x servers through the BMC; `ibmpex` reads IBM PowerExecutive temperature and power sensors on listed IBM System x servers; `power_meter` reads ACPI 4.0 power meters and can expose a power cap. |
| S12 | Hewlett Packard Enterprise, [*HPE iLO IPMI User Guide*](https://support.hpe.com/hpesc/public/api/document/c04530505), part number 30-585D730D-002, October 2021, Edition 1, SHA-256 `f6f8b67cc1ba42450fdd82d0ee737db0a1d57e03b28c136daaa118c1d98cae6a`: p. 8 Table 2, pp. 11–12 (Reading the SDR repository), pp. 13–14 (iLO security modes, interoperability modes) | Primary vendor BMC documentation. iLO temperature sensors are type `01h` with reading type `01h` (threshold). iLO fan sensors are type `04h` with a discrete reading type: `0Ah` (Availability) in Legacy mode and `07h` (Severity) in the default HPE Open mode. In High security, FIPS and CNSA modes, IPMI over the system interface is limited to user-level commands plus a few operator-level commands, and High security adds encryption and authentication on the host interface. Applications detect SDR changes from the Get SDR Repository Info addition timestamp. |
| S13 | Intel, [*Intel Server Board S1200SP Family Technical Product Specification*](https://www.intel.com/content/dam/support/us/en/documents/server-products/S1200SP_TPS_AllSKU.PDF), Revision 2.0, March 2020, SHA-256 `5045bff6edd4c0d3ec04e146eb816add9cf38a567ce2b1390b5ed7e129abd54c`: §6.10.2 (printed pp. 47–48), Appendix B (printed p. 111) | Primary vendor BMC documentation. The BMC exposes "thermal margin" sensors: the margin from the throttling threshold, not an absolute temperature. Some sensors are virtual and computed by the BMC. `P1 Therm Margin` is sensor `74h`, sensor type Temperature (`01h`), event/reading type Threshold (`01h`). |
| S14 | LibreHardwareMonitor at commit [`2808d80564b6`](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/tree/2808d80564b6761e1b01f42fabfd966ed695fcbc/LibreHardwareMonitorLib/Hardware/Motherboard/Lpc), `Hardware/Motherboard/Lpc/Ipmi.cs` (SHA-256 `82ce9f9f37cbf9d74fbd39326c0f4430bdf19b82e05939cc9aa33ae5fae152a4`) and `Lpc/LpcIO.cs` (SHA-256 `87c1239fc98e34400208b373c26d3b5349ed058ab36dc86219827793f46308b5`), MPL-2.0 | **lead-only (copyleft)**, used for the Target inventory and for the `ResponseData` type: on Windows it enables its IPMI path whenever an active `Microsoft_IPMI` instance exists. It reads Full Sensor Records with whole-record Get SDR requests at offset 0 without a reservation, and reads temperature, voltage and fan sensor types. Its requests go to responder `20h`, LUN 0. On boards whose SMBIOS manufacturer is Supermicro it also sends Supermicro OEM requests on NetFn `30h` (command `45h` fan mode get and set, command `70h` sub-function `66h` fan level get and set). It receives `ResponseData` as a byte array. |
| S15 | [Super I/O access specification](superio-access.md) revision 5, section Sampling-path rediscovery cadence, rule R2 | Repository specification. The backoff formula reused, as product policy, for BMC discovery retries. |
| S16 | [ADR 0029](../../adr/0029-windows-sensor-service.md) and [`docs/architecture/backend.md`](../../architecture/backend.md) (Windows sensor service) | Repository architecture: privileged Windows sensor sampling runs in the MSI-installed LocalSystem sensor service, and the app never elevates itself automatically. |

## Target inventory

Each row is keyed by the mechanism through which a reading would be
obtained. Vendor, board and BMC firmware names are evidence; they never
select a path.

| Target | Mechanism | Disposition in this revision |
| --- | --- | --- |
| T1. Windows hosts with an IPMI BMC bound to the in-box Microsoft IPMI driver | `Root\wmi` `Microsoft_IPMI.RequestResponse` (S2, S3), called from the LocalSystem sensor service (S16) | **This specification, Experimental.** Temperature and fan RPM only, under the record filters below. |
| T2. Linux hosts with an IPMI BMC bound to the kernel IPMI driver | `ipmi_devintf` node `/dev/ipmi<N>` (S8, S9, S10) | **This specification, Experimental.** The same protocol and filters as T1. Unavailable when the process cannot open the node. |
| T3. Windows IPMI Provider CIM classes (`Root\Hardware` `NumericSensor`, `Sensor`) | `IPMIPrv` decodes the SDRs itself (S3, S4) | Deferred. Follow-up (not yet filed): Microsoft does not document which IPMI commands the provider sends for an instance query, whether it reserves the repository, or how it caches, so the OS-owned read path cannot be audited as the specs README requires. `CurrentReading` is an integer scaled by `UnitModifier`, and the class carries state-changing methods that this project would never call. |
| T4. Direct KCS, SMIC, BT or SSIF access (PawnIO port I/O, SMBus) on Windows | Raw system-interface registers (S1 §9 to §12) | Unsupported. The OS driver owns the interface, and a second host-side owner would interleave KCS transfers with it. No PawnIO module exposes these ports for this purpose. |
| T5. Linux direct port access bypassing `ipmi_si` / `ipmi_ssif` | Same as T4 | Unsupported, for the same reason. |
| T6. Linux IPMI-backed hwmon drivers `ibmaem` and `ibmpex` | IBM OEM energy/power interfaces through the BMC (S11) | Deferred. Follow-up (not yet filed): IBM System x only, power-oriented, with OEM semantics this revision does not specify. A bound hwmon endpoint would follow the bound-endpoint pattern of other Linux specifications. |
| T7. ACPI power meter (`ACPI000D`; Linux `power_meter`, Windows `acpipmi`) | ACPI 4.0 power meter, not IPMI (S7, S11) | Out of scope for #2525: power, not temperature or fan, and not a BMC SDR path. |
| T8. LibreHardwareMonitor comparison path | Windows `Microsoft_IPMI` raw requests (S14) | Temperatures and fans: covered by T1 under stricter filters (record validation, unit, linearization and owner checks). Voltages: Deferred to a follow-up, together with current and power types (see T12). Supermicro OEM fan-mode and fan-level commands: never sent. They include writes, and an OEM read of fan duty is outside the temperature/fan scope. |
| T9. HPE iLO fan sensors | Discrete fan sensors (reading type `0Ah` or `07h`) with no RPM reading (S12) | Unsupported: no RPM is defined. iLO temperature sensors are threshold sensors and fall under T1/T2. |
| T10. Compact (`02h`), event-only (`03h`), OEM (`C0h`) and other non-sensor records | No analog reading in the record type (S1 §43.1, §43.2) | Not read beyond the header. Compact and event-only records cannot carry conversion factors. OEM records are vendor-defined. |
| T11. Linearized (`01h`–`0Bh`), non-linear (`70h`) and OEM non-linear (`71h`–`7Fh`) sensors | Linearization function or Get Sensor Reading Factors (S1 §36) | Deferred. Follow-up (not yet filed). Not published in this revision. |
| T12. Voltage, current, power and other sensor types; percent or rate-qualified fan units | Same records, other units | Deferred. Follow-up (not yet filed). Core has no BMC voltage/current/power presentation contract yet. |
| T13. Sensors owned by a satellite controller, by system software, on another channel or on a LUN other than 0 | IPMB bridging or another addressing path (S1 §5.4, §43.1) | Deferred. Follow-up (not yet filed). Bridging through the BMC uses Send Message, which this revision never sends. |
| T14. Device SDRs (Get Device SDR, NetFn `04h` command `21h`) of a controller without an SDR Repository | Different command set (S1 §35.2–§35.4) | Deferred. Follow-up (not yet filed). |
| T15. Remote BMC access (IPMI over LAN/RMCP+, Redfish, WS-Management), credentials, network discovery | Network | Out of scope for #2525. |
| T16. SEL reading or clearing, threshold or hysteresis changes, sensor re-arm, fan or OEM control, chassis, power, watchdog, reset and firmware commands | Mutation or state change | Excluded. Never sent on any platform (Safety notes). |
| T17. macOS | No IPMI system-interface driver | Unsupported. |
| T18. Desktop boards without a BMC | No `IPI0001` device, no IPMI driver binding | Nothing to read. Covered by existing Super I/O and ACPI paths. |

## Detection

| Condition | Gate kind | Source |
| --- | --- | --- |
| Windows: at least one instance of `Microsoft_IPMI` in `Root\wmi` with `Active` = TRUE. Each active instance is one BMC interface. | mechanism: the OS IPMI driver is bound and exposes its request interface | S2, S3, S6, S7 |
| Windows: the instance's `BMCAddress` equals `20h` | mechanism: the responder address must equal the owner address recorded in the SDRs read below | S1 §5.4, S2, S5 |
| Windows: the caller runs in the LocalSystem sensor service (or another process with Administrators membership) | safety: avoids an access failure in the normal app and never triggers elevation | S5, S16 |
| Linux: a character device `/dev/ipmi<N>` exists and the process can open it | mechanism: `ipmi_devintf` is bound to interface `N` | S8, S10 |
| Linux: the process is a 64-bit (LP64) Linux process | mechanism: selects the structure layouts and ioctl codes in Linux transport | S9 |
| Get Device ID completes with `00h` and its Additional Device Support byte has bit 1 (SDR Repository Device) and bit 0 (Sensor Device) set | mechanism: the BMC implements the SDR Repository and sensor commands used here | S1 Table 20-2 |
| Get Device ID Firmware Revision 1 bit 7 = 0 | safety: bit 7 = 1 means a firmware update, an SDR Repository update or self-initialization is in progress | S1 Table 20-2 |
| Get Device ID IPMI Version is `51h` (IPMI 1.5) or `02h` (IPMI 2.0) | mechanism: the command and record formats of S1 | S1 Table 20-2 |
| Get SDR Repository Info completes with `00h` and its SDR Version byte is `51h` | mechanism: the SDR command set and record formats of S1 | S1 §33.9, §33.12 note |
| Per record: header and key-field filters in Record selection | mechanism | S1 §43.1 |
| Per reading: Get Sensor Reading flags and the admitted range in Read procedure | mechanism (validity bits) and plausibility | S1 Table 35-15, ADR 0011 |

The Get Device ID Manufacturer ID, Product ID, Device ID and firmware
revision, the board and BMC vendor, the `InstanceName` string and the
interface number `N` are evidence identities or session enumeration
details. They never gate a read.

| Scope | Status | Default enablement |
| --- | --- | --- |
| Windows, `Microsoft_IPMI.RequestResponse` from the LocalSystem sensor service: temperature and fan RPM from selected Full Sensor Records | Transport primary (S2, S3, S5, S6); in-box INF observed on a client edition without a BMC (S7). Protocol, record layout and conversion primary (S1). Lead-only S14: `ResponseData` arrives as a byte array (the documented MOF shows a scalar; see Open question 3). No BMC observation and no maintainer-accepted hardware dump. Best-evidenced members: none observed. HPE iLO temperatures (S12) and Intel S1200SP threshold temperature sensors (S13) are documented to use this record and reading type. | Experimental |
| Linux, `ipmi_devintf` from the application process: the same readings | Protocol primary (S1). Driver interface primary (S8). Lead-only S9: ioctl codes, structure layouts, address-type, channel and receive-type constants, maximum message length. Lead-only S10: node naming and the absence of a driver-side capability check. No BMC observation and no maintainer-accepted hardware dump. | Experimental. Unavailable when the node cannot be opened. |
| Windows IPMI Provider CIM classes | See T3 | Deferred. Not used. |
| Everything else in the Target inventory | See the inventory | Unsupported, Deferred or out of scope as listed |

## IPMI commands (facts)

The complete set of commands this specification sends. All are sent to
the BMC (responder address `20h`) at LUN 0. Table G-1
assigns each command a minimum privilege level; privilege levels apply
to sessions, and the system interface is session-less (S1 §6.12.1,
Table G-1).

| Command | NetFn (request) | Cmd | Request data | Response data (byte 1 is the completion code) | Source |
| --- | --- | --- | --- | --- | --- |
| Get Device ID | `06h` App | `01h` | none | 2 Device ID; 3 Device Revision (bit 7 = provides Device SDRs); 4 Firmware Revision 1 (bit 7 = device busy: firmware, SDR Repository update or self-initialization in progress; bits 6:0 major); 5 Firmware Revision 2 (BCD minor); 6 IPMI Version (`51h` = 1.5, `02h` = 2.0); 7 Additional Device Support (bit 1 SDR Repository Device, bit 0 Sensor Device); 8–10 Manufacturer ID, LS byte first; 11–12 Product ID, LS byte first; 13–16 optional auxiliary firmware revision | S1 §20.1 Table 20-2, Table G-1 |
| Get SDR Repository Info | `0Ah` Storage | `20h` | none | 2 SDR Version (`51h`); 3–4 record count, LS byte first; 5–6 free space; 7–10 most recent addition timestamp, LS byte first; 11–14 most recent erase timestamp, LS byte first; 15 operation support | S1 §33.9 Table 33-3, Table G-1 |
| Get SDR | `0Ah` Storage | `23h` | 1–2 Reservation ID, LS byte first (always `0000h` here); 3–4 Record ID, LS byte first; 5 offset into record (always `00h` here); 6 bytes to read | 2–3 Record ID of the next record, LS byte first; 4 onward the requested record bytes | S1 §33.12 Table 33-6, Table G-1 |
| Get Sensor Reading | `04h` Sensor/Event | `2Dh` | 1 sensor number | 2 reading byte; 3 flags (bit 7 = 0: all event messages disabled; bit 6 = 0: sensor scanning disabled; bit 5 = 1: reading/state unavailable); 4 threshold comparison status (bits 5–0: at or above upper non-recoverable, critical, non-critical; at or below lower non-recoverable, critical, non-critical); 5 optional | S1 §35.14 Table 35-15, Table G-1 |

Get SDR rules used here (S1 §33.11.1, §33.12):

- A reservation is required only for a Get SDR with a non-zero offset
  into the record. A request at offset `00h` uses Reservation ID
  `0000h`. Reading from offset `00h` is the documented way to extract a
  record's header and key fields.
- Record ID `0000h` returns the first record. The response always
  carries the next Record ID, and `FFFFh` in that field means the last
  record has been returned. Record IDs need not be sequential or
  consecutive (S1 §33.12, S12 pp. 11–12).
- Bytes to read `FFh` means the entire record. Most system interfaces
  cannot return it, and the BMC then answers `CAh` (or `FFh`). This
  revision never requests `FFh`.
- Record IDs can be reassigned. A reader must verify that the record it
  received carries the key fields it expects (S1 §33.11.1, §43
  introduction).

Completion codes used for classification (S1 Table 5-2; the same values
are listed in S2): `00h` normal; `C0h` node busy; `C1h` invalid command;
`C3h` timeout while processing; `C5h` reservation canceled or invalid;
`C7h` request data length invalid; `C9h` parameter out of range; `CAh`
cannot return number of requested data bytes; `CBh` requested sensor,
data or record not present; `CCh` invalid data field; `CDh` command
illegal for this sensor or record type; `CEh` response could not be
provided; `D0h` SDR Repository in update mode; `D1h` firmware update
mode; `D2h` BMC initialization in progress; `D3h` destination
unavailable; `D4h` insufficient privilege or firmware-firewall
restriction; `D5h` not supported in present state; `D6h` sub-function
disabled; `FFh` unspecified. A responder may truncate the response after
a non-zero completion code (S1 §5.3.1). Generic software treats
device-specific codes `01h`–`7Eh` as `FFh` (S1 §5.3.2).

## Full Sensor Record layout (facts)

Byte numbers are 1-based positions in the record as returned by Get SDR,
starting with the record header. Only fields this specification reads
are listed. Source: S1 §43.1 Table 43-1 (pp. 551–557).

| Byte | Field | Bits and meaning |
| --- | --- | --- |
| 1–2 | Record ID | LS byte first |
| 3 | SDR Version | `51h` for S1 |
| 4 | Record Type | `01h` = Full Sensor Record |
| 5 | Record Length | Number of record bytes after this byte |
| 6 | Sensor Owner ID | Bits 7:1 the 7-bit slave address or software ID; bit 0: 0 = IPMB slave address, 1 = system software ID. The BMC's 8-bit form is `20h` (S1 Table 43-1 note 2, §5.4). |
| 7 | Sensor Owner LUN | Bits 7:4 channel number; bits 3:2 reserved; bits 1:0 owner LUN |
| 8 | Sensor Number | `FFh` reserved |
| 9 | Entity ID | Physical entity monitored (S1 Table 43-13); not interpreted in this revision |
| 10 | Entity Instance | Bit 7 logical container; bits 6:0 instance; not interpreted in this revision |
| 13 | Sensor Type | `01h` Temperature, `04h` Fan (S1 Table 42-3) |
| 14 | Event/Reading Type Code | `01h` = threshold-based (S1 Table 42-1) |
| 21 | Sensor Units 1 | Bits 7:6 analog data format: `00b` unsigned, `01b` 1's complement, `10b` 2's complement, `11b` no analog reading. Bits 5:3 rate unit (`000b` none). Bits 2:1 modifier unit (`00b` none). Bit 0 percentage. |
| 22 | Sensor Units 2, base unit | S1 Table 43-15: 1 = degrees C, 2 = degrees F, 3 = degrees K, 18 = RPM |
| 23 | Sensor Units 3, modifier unit | Unit code, `00h` if unused |
| 24 | Linearization | Bits 6:0: `00h` linear; `01h`–`0Bh` the named linearization functions; `70h` non-linear; `71h`–`7Fh` OEM non-linear |
| 25 | M, LS 8 bits | M is a 10-bit 2's complement value |
| 26 | M, MS 2 bits and tolerance | Bits 7:6 the two most significant bits of M; bits 5:0 tolerance |
| 27 | B, LS 8 bits | B is a 10-bit 2's complement value |
| 28 | B, MS 2 bits and accuracy | Bits 7:6 the two most significant bits of B; bits 5:0 accuracy LS bits |
| 30 | R and B exponents | Bits 7:4 R (result) exponent, 4-bit 2's complement; bits 3:0 B exponent, 4-bit 2's complement |
| 48 | ID String Type/Length | Bits 7:6 type (`00b` Unicode, `01b` BCD plus, `10b` 6-bit packed ASCII, `11b` 8-bit ASCII + Latin 1); bit 5 reserved; bits 4:0 length (`00000b` none, `11111b` reserved). For type `11b`, a length of 1 is reserved. (S1 §43.15) |
| 49 onward | ID String | At most 16 bytes. Bytes after the string in a fixed-length record are unspecified. |

- The Full record is defined as a 64-byte record (S1 §43.1). Bytes 6 to
  48 are 43 bytes, so a record whose ID string has `n` bytes has a
  Record Length of at least `43 + n`.
- Only the Full record type provides analog readings; the Compact record
  does not (S1 §43.1 capability table).
- Generic system management software does not access sensors that have no
  SDR (S1 §43 introduction).

Conversion (S1 §36.3): `y = L[(M × x + B × 10^K1) × 10^K2]` in the
record's units, where `x` is the raw reading, `K1` is the B exponent,
`K2` is the R exponent and `L` is the linearization function. `L` is the
identity for linear sensors (`00h`). Accuracy, tolerance and resolution
are not applied to the reading (S1 §36.4).

## Windows transport

Facts (S2, S3, S5, S6, S7):

- The in-box driver `IpmiDrv.sys` is a WDM kernel-mode driver that talks
  to the BMC over KCS. Its user-mode interface is the WMI class
  `Microsoft_IPMI` in namespace `Root\wmi`, provider `WMIProv`.
- The driver package in the observed Windows 11 driver store binds only
  hardware ID `ACPI\IPI0001` (S7). `IPI0001` is the ACPI `_HID` or
  `_CID` of an IPMI system interface device (S1 Appendix C3).
- `RequestResponse` is a dynamic method invoked on an instance. Inputs:
  `NetworkFunction` (request NetFn), `Command`, `Lun`, `ResponderAddress`,
  `RequestData` (byte array) and `RequestDataSize` (its length). Outputs:
  `CompletionCode`, `ResponseData` and `ResponseDataSize`.
  `ResponseData` starts with the completion code, and `ResponseDataSize`
  counts it.
- Calling `RequestResponse` requires membership in the Administrators
  group (S5).
- Microsoft documents no request timeout, no cancellation, and no
  serialization guarantee between callers for this method (Open
  questions 1 and 2).

Rules:

1. Run every call in the LocalSystem sensor service, on a worker that is
   not a UI or event-loop thread. The normal app process never calls
   the method and never elevates for it. When the service is absent,
   the readings are Unavailable.
2. Keep at most one outstanding `RequestResponse` call per BMC instance
   in the process. Wait at most 5 seconds for a call (product policy).
   If the wait expires, the request has failed with a transport timeout.
   Start no new call on that instance until the outstanding call has
   returned, so calls never pile up behind a stalled driver.
3. Use `RequestResponse` only. Never call `RequestResponseEx`,
   `SMS_Attention`, or any method of the IPMI Provider classes.
4. A response is well-formed only when the method succeeds, `ResponseData`
   is a byte array whose length equals `ResponseDataSize`, the size is at
   least 1, and its first byte equals `CompletionCode`. Anything else is a
   transport failure for that request.
5. A WMI access-denied error is a permission failure: the reading is
   Unavailable with that detail, and no elevation is requested.

## Linux transport

Facts from the public documentation (S8):

- Each open file of the IPMI device is one IPMI user. The application
  sends a request with `IPMICTL_SEND_COMMAND`, waits for readability with
  `poll()`, and takes the response with `IPMICTL_RECEIVE_MSG_TRUNC`. The
  caller supplies the receive buffer and its length.
- A system-interface address with channel equal to the BMC channel
  addresses the local BMC directly.
- The first byte of a response's data is the completion code.

ABI facts for 64-bit Linux, **lead-only (copyleft)** S9 (structure
layouts restated as byte offsets; natural C alignment):

| Item | Value |
| --- | --- |
| Node | `/dev/ipmi<N>`, one per IPMI interface number `N` (device class `ipmi`), lead-only S10 |
| Send ioctl | `IPMICTL_SEND_COMMAND`, request code `0x8028690D` (read direction, type `'i'` = `0x69`, number 13, argument size 40 bytes) |
| Receive ioctl | `IPMICTL_RECEIVE_MSG_TRUNC`, request code `0xC030690B` (read/write direction, type `0x69`, number 11, argument size 48 bytes). A response longer than the buffer is truncated instead of left queued. |
| System-interface address (8 bytes) | offset 0: address type, 32-bit signed, value `0x0C`; offset 4: channel, 16-bit signed, value `0x0F` (BMC channel); offset 6: LUN, 8-bit unsigned; offset 7: padding |
| Message descriptor (16 bytes) | offset 0: NetFn, 8-bit; offset 1: command, 8-bit; offset 2: data length, 16-bit unsigned; offset 4: padding; offset 8: pointer to the data buffer |
| Send argument (40 bytes) | offset 0: pointer to the address; offset 8: address length, 32-bit unsigned (8); offset 12: padding; offset 16: message ID, 64-bit signed; offset 24: message descriptor |
| Receive argument (48 bytes) | offset 0: receive type, 32-bit signed; offset 4: padding; offset 8: pointer to an address buffer; offset 16: address buffer length, 32-bit unsigned (in: buffer size; out: actual length); offset 20: padding; offset 24: message ID, 64-bit signed; offset 32: message descriptor (in: data buffer and its size; out: actual data length) |
| Receive type of a response to a command | `1` |
| Maximum message length | 272 bytes; use a 272-byte receive data buffer and a 32-byte address buffer |
| Send errors | `EFAULT`, `EINVAL` (address invalid or command not allowed), `EMSGSIZE`, `ENOMEM` |
| Receive errors | `EAGAIN` (queue empty), `EFAULT`, `EINVAL` |
| Delivery | A user receives responses only to its own commands. Asynchronous events can also reach open users. |

Rules:

1. Open each `/dev/ipmi<N>` with read and write access. `EACCES`,
   `EPERM` or `ENOENT` makes that interface Unavailable with a permission
   or absence detail. Never change the node's mode or ownership, install
   a udev rule, load or unload a module, or request elevation.
2. Use only the two ioctls above. Never issue any other IPMI ioctl: no
   event registration, command registration, address, LUN, timing or
   maintenance-mode ioctl.
3. Give every request a message ID that is unique within the open file.
   After sending, `poll()` for readability for at most 5 seconds in total
   (product policy), then receive. Discard any received message whose
   receive type is not `1` or whose message ID differs, and keep waiting
   within the same 5-second bound. Discard at most 64 messages per
   request (product policy); then the request has failed. Expiry of the bound is a
   transport timeout. A late response that arrives afterwards is
   discarded by its message ID.
4. A response is well-formed only when the received data length is at
   least 1 and the response NetFn equals the request NetFn plus 1 with the
   same command (requests use even and responses odd NetFn values, S1
   Table 5-1). The first data byte is the completion code.
5. Keep at most one outstanding request per open file. Run the exchange
   off any UI or event-loop thread.

## Record selection

A Full Sensor Record is published only when every condition holds.
Records that fail are skipped silently. They are not errors, and the
skip reason is kept for diagnostics.

1. Header: SDR Version `51h`; Record Type `01h`; Record Length `L` with
   `43 + n ≤ L ≤ 59`, where `n` is the ID string length from byte 48.
   A Record ID that equals the one requested, except for the first
   request of an enumeration (Record ID `0000h`).
2. Owner: byte 6 = `20h` (the BMC, IPMB address form); byte 7 bits 7:4 =
   0 (channel 0) and bits 1:0 = `00b` (LUN 0); byte 8 ≠ `FFh`.
3. Event/Reading Type Code (byte 14) = `01h`.
4. Sensor Units 1 (byte 21): bits 7:6 ≠ `11b`; bits 5:3 = `000b`; bits
   2:1 = `00b`; bit 0 = 0.
5. Linearization (byte 24) bits 6:0 = `00h`.
6. Sensor Type and unit:

   | Sensor Type (byte 13) | Base unit (byte 22) | Published as |
   | --- | --- | --- |
   | `01h` Temperature | 1 (°C), 2 (°F) or 3 (K) | BMC temperature, °C |
   | `04h` Fan | 18 (RPM) | BMC fan speed, RPM |

   Any other combination is not published.
7. M (10-bit, bytes 25–26) ≠ 0 (product policy: a zero multiplier makes
   every raw value convert to the same constant).
8. ID string type `11b` with a length of 0 or at least 2. Another type,
   or the reserved length 1, leaves the channel without a BMC label but
   does not skip it. A length of 17 to 30, or the reserved value 31,
   skips the record.

These rules use only S1 fields. They identify the threshold-based, linear,
BMC-owned temperature and fan sensors that S1 defines with a numeric
reading. They do not depend on any vendor, board or firmware identity
(ADR 0031).

## Read procedure and decode

### Discovery (once, then cached)

1. Open the transport (Windows instance or Linux node). Apply the
   Detection gates for the transport.
2. Send Get Device ID. Require completion code `00h` and at least 12
   response bytes. Apply the Get Device ID gates. Bit 7 of Firmware
   Revision 1 set, a transport timeout or failure, or completion code
   `C0h`, `D1h` or `D2h` ends discovery as *rediscovery eligible*. Any other failure of a gate ends
   it as *Unsupported on this interface*.
3. Send Get SDR Repository Info. Require `00h`, at least 15 response
   bytes and SDR Version `51h`. Keep the record count and the two
   timestamps. `D0h`, `C0h`, `D2h` or a transport timeout or failure ends discovery
   as *rediscovery eligible*. Another failure ends it as *Unsupported on
   this interface*.
4. Enumerate. Start with Record ID `0000h`. For each record:
   1. Header read: Get SDR with Reservation ID `0000h`, the Record ID,
      offset `00h` and bytes to read 5. Require `00h` and exactly 8
      response bytes: completion code, next Record ID (2 bytes) and the
      5-byte header.
   2. If the header's Record Type is not `01h`, skip the record. If it is
      `01h` but its SDR Version is not `51h` or its Record Length is below
      43 or above 59, skip it as *malformed record* without a body read.
   3. Otherwise, body read: Get SDR with the Record ID from header
      bytes 1–2 (the first record's own ID when `0000h` was requested), offset
      `00h` and bytes to read `5 + L`, where `L` is the header's Record
      Length (a whole-record read from offset 0; at most 64 bytes).
      Require `00h`, exactly `3 + 5 + L` response bytes, a next Record ID
      equal to the header read's, and header bytes 1–5 identical to the
      header read. Completion code `CAh`, `FFh`, `C7h`, `C8h` or `C9h`
      means this interface cannot return the record whole. Skip the
      record with reason *needs incremental read*; this revision does
      not reserve the repository (Open question 4).
   4. Apply Record selection. Keep the selected records' decode fields,
      keyed by (owner ID, owner LUN byte, sensor number).
   5. Continue with the next Record ID. Stop at next Record ID `FFFFh`.
      Also stop, and mark the enumeration incomplete, when a Record ID
      repeats, or after as many records as the repository record count
      (and at least one).
   6. Failures are classed by where they occur. The next Record ID is
      known only from a successful header read (step 4.1): a responder may
      truncate every byte after a non-zero completion code (S1 §5.3.1),
      and Record IDs are not sequential (S1 §33.12), so no later record
      can be located without it.
      - Header read fails. `CBh` on the first request (Record ID
        `0000h`) when Get SDR Repository Info reported a record count
        of 0 means an empty repository: cache an empty set. Any other
        `CBh` (the record named by the previous next Record ID is not
        present, so the repository changed during the read; S1 §33.11),
        any other non-zero completion code, a transport timeout or
        failure, or a malformed header response ends discovery as
        *rediscovery eligible*. Nothing from the partial enumeration is
        cached or published.
      - Body read fails after a successful header read. The next Record
        ID is already known. `CAh`, `FFh`, `C7h`, `C8h` or `C9h` skips
        the record as in step 4.3, and enumeration continues with that
        next Record ID. `CBh`, any other non-zero completion code, a
        transport timeout or failure, or a malformed body response ends
        discovery as *rediscovery eligible* without publishing anything.
5. Send Get SDR Repository Info again. If the record count or either
   timestamp changed during enumeration, discard the result and end
   discovery as *rediscovery eligible* (the repository changed under the
   read; S1 §33.11.2, S12 p. 12).
6. Two selected records with the same key: drop both and keep the
   reason (duplicate key).
7. Cache the selected records and the repository's record count and
   timestamps. An enumeration that selects nothing caches an empty set:
   the interface has no supported readings, and no Get Sensor Reading
   is sent.

Discovery cadence (product policy, modelled on S15 rule R2): after an
attempt that ends *rediscovery eligible*, the next attempt may start
only after `D(n) = min(2 s × 2^(n−1), 30 s)`, where `n` counts consecutive
eligible failures on that interface. An *Unsupported on this interface*
result starts no further attempt in the session. A discovery never runs
inside a sampling pass.

### Sampling pass

1. Run a pass only while BMC readings are requested (visible, or an
   explicit background consumer). Start a pass at most once every 2
   seconds per interface (product policy). Passes never overlap. S1
   allows a BMC to return a stored, periodically updated reading
   (§35.14), so faster polling adds traffic without new data.
2. For each cached record, in record order, send Get Sensor Reading
   with the record's sensor number to responder `20h`, LUN 0.
3. Per response:
   - Completion code other than `00h`, or fewer than 3 response bytes:
     that channel is Unavailable for the pass. `CBh` or `CDh` also marks
     the cache stale (step 5).
   - Flags byte bit 6 = 0 (scanning disabled) or bit 5 = 1 (reading or
     state unavailable): Unavailable. S1 sets bit 5 when the entity is
     not present or the BMC has no valid reading.
   - Otherwise decode the reading byte as below.
4. Timeout limit (product policy): a transport timeout makes that channel
   and every remaining channel of the pass Unavailable, and ends the
   pass. The pass costs at most one 5-second wait. After 3 consecutive
   passes that end this way, drop the cache and treat the interface as
   needing discovery (rediscovery eligible).
5. Staleness (product policy): at most once every 60 seconds, and after a
   `CBh` or `CDh`, send Get SDR Repository Info between passes. A change
   in record count or either timestamp drops the cache and starts
   discovery under the cadence above. A failed check leaves the cache in
   place.
6. A failed channel never shows zero and never reuses an earlier value.

### Decode

1. Raw value `x` from the reading byte `r` per Sensor Units 1 bits 7:6:
   `00b`: `x = r`. `01b` (1's complement): `x = r` if `r < 80h`, else
   `x = r − 255` (so `FFh` is −0, that is 0). `10b` (2's complement):
   `x = r` if `r < 80h`, else `x = r − 256`.
2. `M` = byte 25 + 256 × (byte 26 bits 7:6), minus 1024 if the result is
   at least 512. `B` the same from bytes 27 and 28. `K2` = byte 30 bits
   7:4 and `K1` = byte 30 bits 3:0, each minus 16 if at least 8.
3. `y = (M × x + B × 10^K1) × 10^K2`, in the record's base unit.
4. Temperature: convert to °C (°C: `y`; °F: `(y − 32) × 5 / 9`; K:
   `y − 273.15`). Admit −55 to 125 °C inclusive, the existing motherboard
   temperature policy. A value outside the range is omitted for the pass.
5. Fan: `y` in RPM. A non-finite value, a value below 0 or a value above
   65535 is an Invalid Fan Reading. 0 is an Inactive Fan Reading, and a
   positive value is an Active Fan Reading.
6. The threshold comparison bits (response byte 4) are not presented in
   this revision.

### Attribution and presentation

- Each Windows instance or Linux interface number is one BMC device
  entity, and its identity is session-local. Readings of different BMCs
  are never merged. The Get Device ID manufacturer and product IDs may be
  shown as descriptive metadata only.
- Each published channel is keyed by (owner ID, owner LUN byte, sensor
  number), the combination S1 §5.4 states is unique in the system, plus
  its role (temperature or fan). Its label is the decoded type `11b` ID
  string, with trailing spaces and NUL bytes removed and Latin-1 decoded.
  A channel with no usable label still publishes; the App decides its
  display.
- BMC temperatures are what the BMC reports for its own sensor, under the
  BMC's label. Some BMC temperature sensors report a margin below a
  throttling threshold rather than an absolute temperature, and some are
  virtual values computed by the BMC (S13). Nothing in the record fields
  distinguishes them. Never feed a BMC temperature into the CPU package,
  core or motherboard Super I/O readings, and never pick one as a
  headline temperature.
- BMC fan readings stay attributed to the BMC entity and are never merged
  with Super I/O fan channels of the same board.

## Safety notes

- **Commands.** Only Get Device ID, Get SDR Repository Info, Get SDR
  (Reservation ID `0000h`, offset `00h`) and Get Sensor Reading are ever
  sent. None changes BMC configuration, the SDR Repository, the SEL,
  thresholds, event status, fan control or power state (S1 §20.1,
  §33.9, §33.12, §35.14).
- **No reservation.** Reserve SDR Repository is never sent. A new
  reservation cancels the reservation held by any other requester
  (S1 §33.11.2), and requesters can cancel each other repeatedly
  (S1 §33.11). A canceled reservation discards another tool's partial
  SDR add (S1 §33.14). Offset-0 reads need no reservation, so this
  revision causes no reservation cancellation.
- **Commands that look like reads but change state** and are never sent:
  Get Message (`06h`/`33h`, dequeues the receive message queue, S1
  §22.6), Read Event Message Buffer (`06h`/`35h`, clears the buffer-full
  flag, S1 §22.8), Re-arm Sensor Events (`04h`/`2Ah`, regenerates event
  status, S1 §35.12) and Reset Watchdog Timer (`06h`/`22h`, restarts the
  watchdog, S1 §27.5).
- **Never sent, any platform:** every Set, Add, Delete, Clear, Enter,
  Exit, Run, Reset and Re-arm command; Clear SEL (`0Ah`/`47h`); Delete
  SDR, Clear SDR Repository, SDR update mode, Run Initialization Agent;
  Set Sensor Threshold, Hysteresis, Event Enable, Type, and Reading and
  Event Status; Send Message (`06h`/`34h`) and Master Write-Read; every
  Chassis (`00h`), Bridge, Firmware, Transport, Group Extension (`2Ch`)
  and OEM (`2Eh`–`3Fh`) request, including the Supermicro `30h` commands
  recorded in S14; Cold and Warm Reset; watchdog commands; and Get Sensor
  Reading Factors, Get Sensor Thresholds and SEL reads (read-only, but
  not needed in this revision).
- **Privilege.** The system interface is session-less and needs no
  credentials (S1 §6.12.1). Windows requires Administrators membership
  for the method (S5), satisfied by the LocalSystem sensor service.
  Linux access is whatever the node's mode grants. Some BMC security
  modes restrict the host interface: HPE iLO limits it to user-level
  commands in High security, FIPS and CNSA modes and adds authentication
  in High security (S12). A restricted interface either answers the four
  commands here or fails them (for example with `D4h`); both outcomes are
  handled, and no credential is ever supplied.
- **Contention.** On Linux, the kernel message handler multiplexes users
  and routes each response to its sender (S8, S9), so `ipmitool` or
  other `/dev/ipmi<N>` users coexist. On Windows, the IPMI provider,
  Windows hardware-event forwarding and other tools also use the driver,
  and Microsoft does not state how the driver orders concurrent callers
  (Open question 2). A vendor agent that bypasses the OS driver and
  drives the KCS registers directly can collide with any driver user,
  this one included. That hazard exists without this application, and
  the application does not widen it beyond adding requests. The BMC
  answers each request on its own, so no multi-request transaction can be
  split, except the header-then-body pair of discovery. Discovery step 5
  detects a repository change around it.
- **Load.** Discovery sends 2 + 1 + up to two Get SDR per record + 1
  requests, once per successful discovery. A pass sends one request per
  cached channel, at most once every 2 seconds, and stops at the first
  timeout.
- No PawnIO module and no ISA or PCI mutex is involved. Do not touch
  KCS, BT, SMIC or SSIF registers directly on any platform.

## Synthetic fixture acceptance

All fixtures are synthetic; no BMC dump exists. Byte strings are
response data after the completion code unless stated. "Record" means
the Full Sensor Record bytes 1 onward.

| Case | Expected result |
| --- | --- |
| Temperature record: owner `20h`, LUN byte `00h`, sensor `30h`, type `01h`, reading type `01h`, units 1 `00h`, base unit 1, linearization `00h`, M bytes `01h`/`00h`, B bytes `00h`/`00h`, exponents `00h`, ID type/length `C8h` "CPU Temp"; Get Sensor Reading completion `00h`, data `2D C0 C0` | 45.0 °C on channel (`20h`, `00h`, `30h`), label "CPU Temp". |
| Same record, units 1 `80h` (2's complement), reading byte `F6h` | −10.0 °C. |
| Same record, units 1 `40h` (1's complement), reading byte `F5h` | −10.0 °C. |
| Same record, units 1 `40h`, reading byte `FFh` | 0.0 °C (1's complement −0). |
| M = 5 (`05h`/`00h`), exponent byte `F0h` (R = −1, B exp = 0), reading `8Ch` | 70.0 °C. |
| M = 1, B = −40 (bytes 27/28 `D8h`/`C0h`), exponents `00h`, reading `64h` | 60.0 °C. |
| M = −1 (bytes 25/26 `FFh`/`C0h`), B = 100 (`64h`/`00h`), reading `14h` | 80.0 °C. |
| Base unit 2 (°F), M = 1, reading `8Ch` | 60.0 °C. |
| Base unit 3 (K), M = 2, reading `96h` | 26.85 °C. |
| Converted temperature 130 °C or −60 °C | Omitted for the pass. |
| Fan record: type `04h`, base unit 18, M = 50 (`32h`/`00h`), reading `30h` | 2400 RPM, Active. |
| Fan reading `00h` with flags `C0h` | 0 RPM, Inactive. |
| Fan with B = −100 and reading `01h`, M = 50 | −50 RPM: Invalid Fan Reading. |
| Flags byte `80h` (scanning disabled) | Unavailable; not 0. |
| Flags byte `E0h` (reading unavailable) | Unavailable; not 0. |
| Get Sensor Reading completion `CBh` | Channel Unavailable; staleness check requested. |
| Get Sensor Reading completion `00h` with only the reading byte `2Dh` (2 response bytes in total) | Channel Unavailable. |
| Transport timeout on the third of five channels | Channels 3–5 Unavailable; 1–2 preserved; pass ends; one 5-second wait only. |
| Three consecutive passes end with a timeout | Cache dropped; discovery after `D(1)` = 2 s. |
| Record type `02h` (Compact), `03h` (event-only) or `C0h` (OEM) | Header read only; no body read; not published. |
| Linearization `07h` (1/x) or `70h` | Not published. |
| Units 1 `C0h` (no analog reading), `08h` (rate per µs) or `01h` (percentage) | Not published. |
| Owner `2Ch` (satellite), `41h` (software ID), LUN byte `01h` or `10h` (channel 1) | Not published. |
| Sensor type `02h` (Voltage) with base unit 4 | Not published. |
| Fan record with base unit 0 or the percentage bit set | Not published. |
| Temperature sensor type with base unit 18 | Not published. |
| M = 0 | Not published. |
| Reading type `0Ah` fan (HPE iLO Legacy) or `07h` (HPE Open) | Not published. |
| Record Length 40 with ID length 0 (below 43) | Skipped, *malformed record*. |
| Record Length 60 | Skipped, *malformed record*. |
| ID type/length `C8h` and Record Length 51 | Published with label from 8 bytes. |
| ID type/length `88h` (6-bit packed ASCII, length 8) or `48h` (BCD plus) | Published without BMC label. |
| ID type/length `C1h` (type `11b`, reserved length 1) and Record Length 44 | Published without BMC label. |
| ID type/length `D1h` (length 17) | Skipped. |
| Body read returns `CAh` | Record skipped, *needs incremental read*; no Reserve SDR Repository sent. |
| Body read returns fewer bytes than `3 + 5 + L` | Discovery ends *rediscovery eligible*; nothing published. |
| Body header differs from the header read | Discovery ends *rediscovery eligible*. |
| Next Record ID repeats `0005h` | Enumeration stops, marked incomplete; records found so far kept. |
| Header read for Record ID `0012h` (the previous next Record ID) returns completion `CBh` with no further bytes | Discovery ends *rediscovery eligible*; no record of this enumeration is cached or published; retried after `D(1)` = 2 s. |
| First header read (Record ID `0000h`) returns `CBh` and Get SDR Repository Info reported record count 0 | Empty cache; no Get Sensor Reading sent; no error. |
| First header read (Record ID `0000h`) returns `CBh` and the reported record count is 12 | Discovery ends *rediscovery eligible*. |
| Header read succeeds with next Record ID `0013h`, body read returns `CAh` | Record skipped, *needs incremental read*; enumeration continues at `0013h`. |
| Header read succeeds, body read returns `CBh` | Discovery ends *rediscovery eligible*; nothing published. |
| Repository Info record count or timestamp differs after enumeration | Result discarded; *rediscovery eligible*. |
| Two selected records with key (`20h`, `00h`, `30h`) | Both dropped. |
| Get Device ID Additional Device Support `01h` (no SDR Repository) | *Unsupported on this interface*; no Get SDR sent. |
| Get Device ID Firmware Revision 1 `81h` | *rediscovery eligible*; retried after 2 s, then 4 s, 8 s, 16 s, 30 s. |
| Get Device ID IPMI Version `10h` | *Unsupported on this interface*. |
| Repository Info SDR Version `01h` | *Unsupported on this interface*. |
| Repository Info completion `D0h` | *rediscovery eligible*. |
| Enumeration selects no record | Empty cache; no Get Sensor Reading sent; no error. |
| Windows: no `Microsoft_IPMI` instance, or only instances with `Active` FALSE | Unsupported on this machine; no method call. |
| Windows: `BMCAddress` `22h` | Unsupported on this interface. |
| Windows: access denied | Unavailable with permission detail; no elevation. |
| Windows: `ResponseDataSize` 3 but `ResponseData` holds 2 bytes, or `ResponseData[0]` ≠ `CompletionCode` | Transport failure for that request. |
| Windows: a call does not return within 5 s | Transport timeout; no new call on that instance until it returns. |
| Windows: sensor service absent | Unavailable; the normal app sends nothing. |
| Linux: `open` fails with `EACCES` | Unavailable with permission detail; no mode change or elevation. |
| Linux: a receive returns type `2` (event), then the response | Event discarded; response used. |
| Linux: a response with another message ID | Discarded; waiting continues within the bound. |
| Linux: response NetFn `0Bh` to a Get Sensor Reading request | Malformed; channel Unavailable. |
| Linux: `/dev/ipmi0` and `/dev/ipmi1` both qualify | Two independent BMC entities; never merged. |

## Open questions

1. Non-blocking for #2525 Experimental scopes: rule 2 of Windows
   transport bounds the application's wait and prevents call pile-up
   whatever the driver does.
   Microsoft documents no request timeout, retry or cancellation
   behavior for `RequestResponse`, and no Windows event for a stalled
   BMC beyond the driver's event log entries. Whether a stalled call
   holds the WMI provider host, and for how long, is unknown.
2. Non-blocking for #2525 Experimental scopes: every command here is a
   single self-contained Get request, so interleaving with other callers
   cannot change BMC state, and discovery step 5 catches a repository
   change.
   Whether `IpmiDrv.sys` serializes requests from several callers
   (this service, `IPMIPrv`, Windows hardware-event forwarding, vendor
   agents) and routes every response to its own caller is not
   documented. A controlled test with two concurrent callers on real
   hardware would settle it.
3. Non-blocking for #2525 Windows scope: rule 4 of Windows transport
   treats any other shape as a transport failure.
   The documented MOF shows `ResponseData` as a scalar `uint8`, while the
   description, `ResponseDataSize` and S14 imply a byte array.
4. Non-blocking for #2525 Experimental scopes: a record the interface
   cannot return whole is skipped and reported, never guessed.
   Some system interfaces cannot return a 64-byte record in one response,
   and S1 then tells software to use reservation-based incremental reads.
   Enabling them needs a decision on Reserve SDR Repository contention
   with other requesters (Safety notes), a chunk size and a retry bound.
   A BMC observation showing `CAh` on whole-record reads would show the
   need.
5. Non-blocking for #2525 Experimental scopes: presentation keeps BMC
   temperatures under the BMC's own label and out of every headline or
   CPU reading.
   Whether a temperature-type sensor reports an absolute temperature or
   a margin (S13) cannot be read from the record. A margin passes the
   admitted range and is shown as the BMC labels it.
6. Non-blocking for #2525 Experimental scopes: the scope is live-only,
   so session-local identity suffices.
   No identity stable across reboots is established for a BMC or a
   sensor. Get Device GUID (`06h`/`08h`) and SDR key fields could provide
   one; Record IDs can change.
7. Non-blocking for #2525 Linux scope: the reading is Unavailable with a
   permission detail, and the application changes nothing.
   Distributions commonly create `/dev/ipmi<N>` accessible only to root
   (the driver sets no mode, S10). Whether HardwareVisualizer should
   document a user-installed udev rule or group is a product decision,
   not a hardware fact.
8. Non-blocking for #2525 Experimental scopes: presentation is an App
   and frontend modelling decision; channel roles, units and
   attribution above are complete.
   Core has no BMC device category. A BMC reading needs a per-BMC device
   entity with session-local identity and labelled temperature and fan
   channels that stay apart from CPU, Super I/O and PSU readings. The
   identity contract is decided in the implementation design review
   (verify-identity-contracts).
9. Non-blocking for #2525 Experimental scopes: cadence values are product
   policy and can change without a hardware fact.
   No source states how often a BMC refreshes its stored readings. The
   2-second pass interval, the 5-second wait and the 60-second staleness
   check are chosen, not documented.
10. Non-blocking for #2525 Windows scope: the driver binds only through
    `ACPI\IPI0001` in the observed package, and a host without that
    device has no instance, which is handled as Unsupported.
    Whether a board that describes its BMC only in SMBIOS Type 38 or the
    SPMI table (S1 §6.11) gets the Windows driver is unverified. S3
    names the device "Microsoft SMBIOS Generic IPMI Compliant Device"
    while the observed INF (S7) matches only the ACPI ID.

## Revision history

| Revision | Date | Change |
| --- | --- | --- |
| 1 | 2026-11-29 | Initial version. Propose Implementation-ready for two Experimental scopes: Windows `Microsoft_IPMI.RequestResponse` from the LocalSystem sensor service and Linux `ipmi_devintf`. Both read BMC temperatures and fan RPM from threshold-based, linear, BMC-owned Full Sensor Records, using only Get Device ID, Get SDR Repository Info, Get SDR at offset 0 without a reservation, and Get Sensor Reading. Record the command layouts, record fields, conversion, validity flags, timeouts, cadence, contention, privilege and never-sent commands, synthetic fixtures, and a disposition for every #2525 target. Review correction before merge: a failed header Get SDR leaves no next Record ID (the response may end at the completion code, S1 §5.3.1), so a header-level `CBh` ends discovery as rediscovery eligible, except on the first request of an empty repository; only a body-read failure after a successful header read can skip a record. |
