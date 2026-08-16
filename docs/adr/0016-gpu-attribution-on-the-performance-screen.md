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
2. Otherwise the first adapter that reports usage, then the first that reports
   a temperature.

The first rule is the decision. The earlier resolver dropped a selection the
moment its adapter stopped appearing in the usage map, which silently replaced
the user's chosen adapter with another adapter's numbers under the same label.
Honoring the selection and reporting nothing is the honest answer; only a
selection pointing at an adapter that no longer exists is discarded, because
there is no longer anything to honor.

An adapter list is therefore needed, not just the live maps. It is the union of
the adapters the one-shot hardware fetch detected and any id that appears in
any live map, so a reading is never rendered without an owner and a detected
adapter is never unreachable. An id the static fetch has not returned — because
it is slow, or because it failed — is named from the temperature or fan map,
which carry the platform's own name for each adapter; only an id no source
names at all is shown as itself.

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
"GeForce RTX 40…". If shortening would make two labels identical, every label
falls back to the full name.

## Placement

The selector sits in the GPU instrument's header, where the readings it governs
are. It is a labelled group of toggle buttons, not a tablist: there is no
tabpanel, no roving tabindex, and no arrow-key contract, so announcing tabs
would promise a keyboard model the control does not implement. A single-adapter machine gets the name alone, because naming is the whole
job when there is no choice to make. Compact names its adapter in the footer
rather than in the GPU row: the row's tracks are sized for the mini monitor's
small corner window and cannot hold a device name. Compact does not offer
selection; it follows the choice made in Panels.

## Consequences

### Positive

- Every GPU number on the Performance Screen states the device it came from.
- Every detected adapter is reachable without leaving grouped navigation.
- A user who selects a silent adapter is told it is silent instead of being
  shown another adapter's numbers.
- The selection is one piece of state (`selectedGpuIdAtom`, persisted by
  `useSelectedGpuPersistence`), so Performance, Compact, and the classic
  Hardware Dashboard agree and the choice survives restarts.

### Negative

- The GPU instrument header is denser than the CPU and RAM headers, which have
  no device to name.
- Long adapter names still truncate inside a three-column strip; the full name
  is only available on hover or to assistive technology.
- Compact fetches the static hardware facts itself, adding one one-shot IPC
  call to a view that previously needed none.
- The unavailable state only covers an adapter that has never reported. The
  live maps are append-only, so an adapter that reports and then goes silent
  keeps its last value on screen; distinguishing a stale reading from a current
  one needs per-sample presence tracking in the event listener, which this
  decision does not add.

### Non-goals

- Changing GPU collection providers, archive semantics, or the Insights GPU
  view.
- Adding selection or live readings to the System Specifications sheet.
- Changing the classic Hardware Dashboard GPU card.
