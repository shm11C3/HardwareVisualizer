# GPU Attribution on the Performance Screen

Status: accepted

Refines [ADR 0014](0014-performance-views-and-specification-sheet.md) and
[ADR 0015](0015-performance-and-system-specifications-destinations.md).

ADR 0014 moved every live GPU reading onto the Performance Screen and left the
System Specifications sheet as static facts. That split left the Instrument
Strip and the Compact strip showing GPU usage, temperature, VRAM, and fan
speed without saying which adapter produced them, and grouped navigation
carried no way to change adapters at all — the only selection control in the
app was the classic Hardware Dashboard GPU card. On a machine with a discrete
and an integrated GPU, a grouped-navigation user could neither tell which
adapter the numbers described nor reach the other one.

Attribution belongs on the surface that makes the claim. The Performance
Screen names the adapter behind its GPU readings and owns the selection; the
specifications sheet stays an inventory of every detected adapter with no
selected state, because it shows nothing that could be attributed.

## Selection beats availability

The effective GPU resolves in this order:

1. An explicit selection, while that adapter is still detected — whether or not
   it is currently reporting anything.
2. Otherwise the first adapter that reports usage, then temperature, then fan
   speed, then dedicated memory.

The first rule is the decision. The earlier resolver dropped a selection the
moment its adapter stopped appearing in the usage map, which silently replaced
the user's chosen adapter with another adapter's numbers under the same label.
Honoring the selection and reporting nothing is the honest answer; only a
selection pointing at an adapter that no longer exists is discarded, because
there is no longer anything to honor.

## The inventory is not a source of adapter identity

The `getHardwareInfo` inventory and the monitor stream key their GPUs in
different namespaces on every platform. Windows NVIDIA reports the raw NVAPI id
as `GraphicInfo.id` but samples as `nvapi:<id>`; macOS pairs
`0x<registry_id>` with `iokit:<name>`; Linux pairs `card<n>` with the PCI BDF.
The two id spaces are disjoint, so they cannot be joined, unioned, or looked up
across.

The adapter list is therefore built from the live side alone: every id the
stream reported, named by the `gpuName` each sample carries, plus any id that
only a value map knows about. Unioning with the inventory would render one
physical GPU as two adapters, one of which reports nothing and is then declared
silent — the exact misattribution this decision exists to prevent. The
inventory's surface is the System Specifications sheet, which lists every
detected adapter as a static fact and attributes no readings.

Where the two sides do have to meet, they meet on the name, which both sources
report — and only while that name picks out exactly one entry on each side.
There are two such places: the VRAM total that labels a live VRAM reading, and
the classic Hardware Dashboard, which holds inventory entries but shares
`selectedGpuIdAtom` with Performance. `findInventoryGpu` and `toLiveGpuId` are
that join, so the shared selection is written in the live namespace by every
surface and read back through the name by the one surface that needs the
inventory.

The classic card is reachable before the first sample, when no live id exists
yet, so a selection made there starts as an inventory id and is reconciled to
the live id as soon as the stream names that adapter. Committing an id that
cannot address readings is what would otherwise leave the card's highlight and
the graphs describing different adapters.

Where the join is ambiguous, the caller refuses rather than falls back: the
classic card claims no adapter identity for a selection it cannot resolve,
instead of labelling one adapter's readings with another's name. Pairing the
two sides by position would look plausible and is a guess — the inventory's
enumeration order and the stream's are different enumerations.

"Every detected adapter is represented" therefore means every adapter the
monitor stream reported. An adapter that names itself and reports no values is
still listed and still selectable; that is what the unavailable state is for.

The derived atoms the classic Usage screen, the classic dashboard, and the
Monitor graph read (`graphicUsageHistoryAtom`, `gpuUsageSourceAtom`,
`gpuDedicatedMemoryKbAtom`) resolve the selection through the same rule. They
previously fell back to the first adapter whenever the selected key was
missing, which would have let Monitor name one adapter in its toolbar and
graph another underneath it.

The subscription that answers these questions lives in the component that
renders the answer, never in a screen's parent: the underlying atoms are
rewritten on every sample, so subscribing higher up would rerender unrelated
panels once a second and break the ADR 0010 rendering-cost rule.

## All four live maps, or none of the conclusions

Usage, temperature, fan speed, and dedicated memory are populated
independently: an adapter can report a fan speed and no usage. Every question
of the form "did this adapter report anything" therefore has to consult all
four. Consulting a subset turns a partially reporting adapter into a silent
one, which is the same misattribution this decision exists to prevent, one
level down.

## Not measured yet is not unavailable

"This adapter is not reporting live readings" may only be stated once some
adapter has reported and this one still has not, in any of the four maps.
Empty maps at startup mean the first sample has not arrived, and claiming
unavailability there would turn a timing gap into a hardware conclusion. The
note is additive rather than a replacement: it appears alongside whatever the
adapter did report, never in place of it.

## Adapter labels

Controls carry a shortened label and keep the full platform name as the
accessible name and tooltip. Shortening removes only what cannot distinguish
one adapter from another: trademark marks, a leading vendor word, and a run of
leading words every detected adapter repeats. The last rule matters because the
shared part is exactly what a narrow card keeps while truncating the model
away — "GeForce RTX 4080" and "GeForce RTX 4060" would otherwise both render as
"GeForce RTX 40…".

Labels are then made unique, because two controls that read identically are
not a choice. Adapters the platform reports under one name — two identical
cards — get an ordinal; if labels still collide, every label falls back to the
full name. The accessible name follows the same rule, so the ordinal is
announced exactly when the raw name cannot tell the cards apart.

A duplicated name also disqualifies the name as a join key. The VRAM total,
the one place the inventory and the live side have to meet, is dropped rather
than guessed when the name is ambiguous on *either* side: two live adapters
sharing it, or two inventory entries sharing it while only one reports.
Showing a live reading against the wrong card's capacity is worse than showing
no denominator.

## Placement

The selector sits in the GPU instrument's header, where the readings it governs
are. It is a labelled group of toggle buttons, not a tablist: there is no
tabpanel, no roving tabindex, and no arrow-key contract, so announcing tabs
would promise a keyboard model the control does not implement.

A single-adapter machine gets the name alone, because naming is the whole job
when there is no choice to make. Monitor carries the same control in its
toolbar whenever the GPU is among the graph's display targets: it mounts only
the graph, so there is nowhere else for the adapter behind the GPU series to
be named. Compact names its adapter in the footer
rather than in the GPU row: the row's tracks are sized for the mini monitor's
small corner window and cannot hold a device name. Compact does not offer
selection; it follows the choice made in Panels.

## Consequences

### Positive

- Every GPU number on the Performance Screen states the device it came from.
- Every detected adapter is reachable without leaving grouped navigation.
- A user who selects a silent adapter is told it is silent instead of being
  shown another adapter's numbers.
- The selection is one piece of state (`selectedGpuIdAtom`), so Performance and
  Compact always agree on which adapter they describe.
- `useSelectedGpuPersistence` restores the stored id as-is instead of
  validating it against the inventory, which it could never match. A selection
  made for an adapter that is absent at the next launch is kept on disk rather
  than overwritten, so it applies again when the adapter returns.

### Negative

- The GPU instrument header is denser than the CPU and RAM headers, which have
  no device to name.
- Long adapter names still truncate inside a three-column strip; the full name
  is only available on hover or to assistive technology.
- Two identical cards lose their VRAM denominator, because the only join
  available cannot say which card is which.
- A GPU that the inventory lists but the monitor stream never reports does not
  appear on Performance at all. It is still on the System Specifications sheet,
  but Performance cannot name a device it has no reading from.
- The unavailable state only covers an adapter that has never reported. The
  live maps, including the name map, are append-only, so an adapter that
  reports and then goes silent keeps its last value on screen and stays in the
  selector for the rest of the session. Distinguishing a stale reading from a
  current one, and an unplugged adapter from a skipped sample, needs per-sample
  presence tracking in the event listener, which this decision does not add.

### Non-goals

- Changing GPU collection providers, archive semantics, or the Insights GPU
  view.
- Adding selection or live readings to the System Specifications sheet.
- Redesigning the classic Hardware Dashboard GPU card. Its selection is
  reconciled with the shared atom here, because making a second adapter
  selectable on Performance would otherwise let the classic card pair one
  adapter's name with another's readings, but nothing else about it changes.
