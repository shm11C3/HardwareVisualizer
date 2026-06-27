# Nuvoton 0xD802 source hunt - 2026-06-28

This is spec-author evidence for #1635 Phase 3. It records the source search
for the observed Nuvoton-class Super I/O chip ID `0xD802`. It is not a clean-room
implementation input by itself.

## Question

Can `0xD802` be pinned to an exact Nuvoton chip model strongly enough to flip
`docs/specs/sensors/superio-nuvoton-nct67xx.md` to Implementation-ready?

## Sources checked

| Source | Result | Use in spec |
| --- | --- | --- |
| Nuvoton official Super I/O Series product page: <https://www.nuvoton.com/products/cloud-computing/i-o/super-i-o-series/> | The static page exposes selection-guide JSON endpoints. The public Super I/O table returned `NCT5104D`, `NCT5124D`, `NCT5585D`, `NCT6106D`, `NCT6116D`, `NCT6126D`, `NCT6776D`, and `NCT6796D-E`; it did not return `NCT6799D` or `NCT6799D-R`. | Negative official-public evidence only. |
| Nuvoton selection-guide endpoint `selectionPage.json?currentFolder=/products/cloud-computing/i-o/super-i-o-series/` with `family=I/O`, `ProductSeries=Super I/O Series`, and `partNo=NCT6799D` / `NCT6799D-R` / `NCT6798D` / `NCT6797D` | Each queried part number returned `resultCount: 0`. | Negative official-public evidence only. |
| Direct Nuvoton product-page probes under `/products/cloud-computing/i-o/super-i-o-series/nct6799d/`, `/nct6799d-r/`, `/nct6798d/`, `/nct6797d/` | Each returned HTTP 404. | Negative official-public evidence only. |
| Direct Nuvoton resource-file URL probes for `NCT6799D`, `NCT6799D-R`, `NCT6798D`, and `NCT6797D` datasheet version patterns `V0_1` through `V2_4` | No public PDF candidate returned success. | Negative official-public evidence only. |
| GECID ASUS ROG CROSSHAIR X670E HERO board teardown: <https://ua.gecid.com/mboard/asus_rog_crosshair_x670e_hero/> | Independent board-review evidence shows an AM5/X670E board populated with a Nuvoton `NCT6799D-R` Super I/O part. It does not report the chip-id bytes. | Corroborates that NCT6799D-R is a real deployed AM5-era part; not enough to map `0xD802`. |
| HenryHu/bsdsensors issue #7: <https://github.com/HenryHu/bsdsensors/issues/7> | Public user report titled for NCT6799D-R includes an unknown Nuvoton chip report for `0xd802`. The reporter explicitly could not find a datasheet. | Non-normative corroborating lead only. |
| lm-sensors issue #462: <https://github.com/lm-sensors/lm-sensors/issues/462> | Public user report for ASUS PRIME B650M-A II includes `nct6799-isa-0290` sensor output and separately mentions `0xD802`. | Independent dump lead, but not strong enough alone for this repo's ready mapping. Do not use implementation snippets from the issue as normative material. |
| AIDA64 public text dumps attached to LibreHardwareMonitor issue #1720: <https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/issues/1720>; summarized in [`2026-06-28-nuvoton-0xd802-aida64-dump.md`](2026-06-28-nuvoton-0xd802-aida64-dump.md) | Independent dump evidence from an ASUS ROG Strix X670E-F Gaming WiFi board reports raw Super I/O device ID `D802h` and labels it `NCT6799D`; the same dump records normal HM base `0x0290` and bank 4 temperature/RPM bytes. | Strong enough independent dump evidence for the scoped `0xD802` -> `NCT6799D` mapping. It does not prove package suffix `-R`, OEM variant, or a public Nuvoton datasheet. |

## Conclusion

`0xD802` can be mapped to `NCT6799D` for a scoped clean-room
implementation using the AIDA64 independent dump evidence. The mapping is not
official-public Nuvoton evidence:

- no public Nuvoton datasheet or product-page source was found for
  NCT6799D/NCT6799D-R;
- no public official Nuvoton chip-id table was found that ties
  `CR20/CR21 = 0xD8/0x02` to the model;
- the independent AIDA64 dump ties raw device ID `D802h` to `NCT6799D` and
  corroborates normal HM base `0x0290` plus bank 4 temperature/RPM bytes;
- public board and user-report leads still make `NCT6799D-R` plausible, but
  the package suffix/revision remains unproven.

For Phase 3 ready, the supported identity should therefore be described as
`0xD802` / `NCT6799D`, with no claim about `-R`, OEM variant, or package
revision. A future public Nuvoton datasheet/table or same-board physical
chip-marking photo can narrow the package suffix later without changing the
raw chip-id support key.
