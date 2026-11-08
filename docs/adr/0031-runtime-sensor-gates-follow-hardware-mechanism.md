# Runtime Sensor Gates Follow the Hardware Mechanism

Status: accepted

Tracking issue: [#2510](https://github.com/shm11C3/HardwareVisualizer/issues/2510).

This record refines [ADR 0011](0011-experimental-sensor-enablement.md) and the
clean-room specification process in
[`docs/specs/sensors/README.md`](../specs/sensors/README.md). It decides which
conditions a sensor specification may place on the runtime path before a read
is attempted. It does not change what the implementer may read, the read-only
policy, the plausibility gates, or the lead-only source rule of
[ADR 0023](0023-copyleft-derived-facts-for-experimental-scopes.md).

## Context

ADR 0011 made Experimental the default for hardware that a safe access path
already recognizes, and named Intel package DTS as the model: the CPUID
capability bits are "existence gates, not verification allowlists". The
Nuvoton profile table follows the same idea with exact chip identifiers and
`0xFFF8`-masked family ranges that fall back to a conservative profile.

The coverage work under #2510 then added many scopes in a short period, and
almost every new scope was pinned to the identity of the observation that
supplied its evidence rather than to the hardware mechanism the evidence
describes:

- Arrow Lake core DTS admits five exact brand strings although all five share
  one signature, one hybrid mechanism and one register decode.
- Vermeer CCD temperatures admit three exact brand strings although every
  Vermeer B0 die has the same CCD registers; only the CCD count differs, and
  the register's own valid bit reports it.
- IT8728F fan readback admits one exact baseboard manufacturer, product and
  version although the fan counters are chip registers and an unpopulated
  header already decodes as inactive.
- The MSI NCT6687D Linux path admits one exact BIOS version per board although
  the kernel driver, not the firmware version, defines the sysfs ABI it reads.
- The ODROID-H4 DIMM path admits one board, one firmware version, one DIMM
  part number and one PCI address although the SPD5118 hwmon ABI and the
  Type 17 identity match are what make the reading attributable.

These identities do not change a register map, a decode or a hazard. They
record where the evidence came from. Using them as runtime gates has three
costs. Coverage work scales with SKUs and firmware builds instead of with
mechanisms. The exactness is inconsistent: the Intel core DTS specification
admits Alder Lake H/P and PS by shared signature "without a product-name
test", while the Arrow Lake specification requires a brand match for the same
hybrid mechanism. A firmware-version gate fails closed on the next BIOS update
with no hardware reason.

## Decision

Every runtime condition in a specification's Detection section is one of
three kinds, and the specification tags it:

- **Mechanism gate.** The condition selects between register maps, addresses,
  decodes or topologies that genuinely differ, using what the hardware or the
  OS reports about itself: CPUID vendor, family and model (the die),
  capability bits, the hybrid flag and per-core type, Super I/O chip
  identifier and revision registers, a register's own valid or presence bit,
  a hwmon chip kind. Mechanism gates are required for correctness.
- **Safety gate.** The condition avoids a known hazard: a configuration-mode
  exit that hangs a bus, an I/O-space lock, an energy-unit difference. Safety
  gates are required and stay as exact as the hazard they describe.
- **Evidence identity.** A brand string, baseboard strings, a BIOS or firmware
  version, a DIMM part number, a bus address. It records which configuration
  an observation came from. It is not a runtime condition. A baseboard
  identity is an evidence identity unless the specification states the
  mechanism it selects, such as an embedded controller's firmware-defined
  channel mapping that differs between boards; then it is a mechanism gate,
  tagged with the fact it selects.

The rules that follow:

1. A runtime gate is a mechanism gate or a safety gate. Evidence identities
   appear in the Status column of the scoped-enablement table and in the
   fixture that corroborates the row. They never block a read.
2. Scope rows are keyed by mechanism. The configurations that were observed
   are the Verified members of the row, or its Experimental members when the
   facts rest on lead-only sources under ADR 0023. Other configurations that
   share the mechanism are Experimental under ADR 0011, with its existing
   read-only, plausibility and failure-presentation rules.
3. A mechanism gate is never guessed. When the fact that distinguishes two
   configurations is unknown, for example the CCD register addresses of a
   different Zen die, the unknown configuration stays Unsupported until the
   specification states that fact. Widening happens by a stated mechanism,
   not by removing a condition and hoping.
4. A firmware version is never a runtime gate. Where a firmware build changes
   what the OS exposes, for example hwmon channel labels, gate on the exposed
   ABI or on the board identity that defines it, and record the version in
   Status.
5. Specifications written before this record keep their current gates until
   each is re-scoped in its own revision. The implementation follows the
   revision through the normal specification gate and clean-room roles.

## Alternatives

- **Keep observation-identity gates.** The safest claim per row, but coverage
  then grows one SKU or BIOS build at a time, the rule contradicts ADR 0011's
  default, and firmware gates break on updates that change nothing the reader
  depends on.
- **Attempt any recognized chip or CPU without a profile.** Guesses register
  maps and decodes. Rejected by ADR 0011's definition of Unsupported and by
  the read-only safety policy.
- **Leave exactness to each specification author.** This is the current
  state, and it produced two specifications that gate the same hybrid DTS
  mechanism differently within a day of each other.

## Consequences

- The specification template's Detection section gains a runtime-gate table
  with a Gate kind column, and the status-transition checklist requires every
  runtime condition to be a mechanism or safety gate.
- The following existing gates are candidates for re-scoping, each as one
  specification revision followed by one implementation PR, in this order:
  Arrow Lake brand match to signature, hybrid flag and core type; the Intel
  legacy core DTS model list to the core DTS capability bit and a plausible
  `MSR_TEMPERATURE_TARGET` value, with the listed models Verified; the
  Vermeer brand match to the Vermeer die and per-slot valid-bit discovery;
  the GA-B75M-D3H baseboard match to the IT8728F primary-slot chip gate; the
  MSI BIOS-version match to board identity and hwmon chip kind; the ODROID-H4
  board, firmware and part match to the one-to-one SPD5118 and Type 17
  identity match.
- The user-initiated Hardware Compatibility Report
  ([#2500](https://github.com/shm11C3/HardwareVisualizer/issues/2500)) is the
  path by which an Experimental member of a mechanism row becomes Verified.
- Hardware safety is unchanged: access stays read-only, mutex conventions,
  plausibility filters and safety gates keep their current exactness, and the
  clean-room role split and prohibited-source list still apply to every
  re-scoping.
