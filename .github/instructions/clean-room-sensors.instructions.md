# Clean-room rules for PawnIO sensor work (HardwareVisualizer)

These instructions enforce the clean-room (Chinese wall) process of
issue #1635 for native CPU / Super I/O sensor monitoring via PawnIO.
The canonical process description lives in
[`docs/specs/sensors/README.md`](../../docs/specs/sensors/README.md);
this file is the AI-facing enforcement summary and the committed
**prohibited-source list**.

Scope: any work on sensor specs under `docs/specs/sensors/**` and any
Rust implementation of CPU / Super I/O sensor access (MSR, SMN,
LPC/ISA port I/O, PawnIO client code).

## Roles

There are two strictly separated roles. A single session/agent must
act in exactly one role. When writing or reviewing Rust sensor code,
the **implementer** rules apply by default.

| Role | Purpose |
| --- | --- |
| Spec author ("dirty room") | Produces fact-only spec documents under `docs/specs/sensors/**` from primary sources |
| Implementer ("clean room") | Writes Rust strictly from those spec documents plus this repository |

## Prohibited sources (implementer role)

The implementer (and reviewers of implementation PRs) must NOT read,
fetch, clone, search for, quote, or otherwise consult:

- LibreHardwareMonitor / LibreHardwareMonitorLib (MPL-2.0)
- OpenHardwareMonitor (MPL-2.0)
- Linux kernel sources — in particular `drivers/hwmon/**`
  (`k10temp`, `coretemp`, `nct6775`, `it87`, …) and `arch/x86`
  MSR/SMN helpers (GPL-2.0)
- lm-sensors / `sensors-detect` (GPL-2.0 / LGPL-2.1)
- Any decompiled or disassembled monitoring tool (HWiNFO, AIDA64,
  CAM, Open Hardware Monitor forks, …)
- Forks, mirrors, vendored copies, patches, blog posts, gists, Q&A
  answers, or AI summaries that reproduce code or code structure from
  any of the above

This applies to every channel: web search, web fetch, `git clone`,
`curl`/`wget`, package contents, local checkouts, screenshots, and
content pasted into the conversation by anyone other than the
maintainer explicitly taking spec-author responsibility.

## Allowed inputs (implementer role)

- `docs/specs/sensors/**` at pinned revisions whose status is **not**
  `Draft — not implementation-ready`
- This repository (code, docs, issues, PRs)
- General language/platform documentation that is not a sensor
  monitoring implementation: Rust std/crate docs, Microsoft Windows
  API documentation, `PawnIOLib.h` from an installed PawnIO release
  (upstream-published API of the driver this project calls)

If required information is missing from the specs, **stop and hand
the question to the spec-author role** (file it as an Open question /
spec revision request). Never fill spec gaps by consulting other
sensor implementations.

## Tool restrictions (implementer sessions)

- Do not use web search / web fetch tools at all.
- Do not use shell commands to fetch or clone anything from the
  prohibited-source list (no `git clone`, `curl`, `wget` of those
  projects). Dependency management (`cargo`/`npm` against their
  default registries) is allowed.
- Prefer running implementation work under the dedicated agent
  definition `.claude/agents/sensor-clean-room-implementer.md`, whose
  toolset omits web access.

## Spec-author role (summary)

Full rules: `docs/specs/sensors/README.md`. In short: vendor
datasheets / public hardware specifications / independently collected
dumps are the primary sources; MPL/GPL/LGPL implementations are
**non-normative leads only** and may never be the sole basis of a
normative fact; no code excerpts, structure, or identifiers from
copyrighted implementations may enter the spec documents; every fact
carries provenance. Use
`.claude/agents/sensor-spec-author.md` for this role.

## License policy

- All new sensor code in this repository is MIT, produced clean-room
  from the spec documents.
- Translating or porting MPL/GPL/LGPL implementation code is
  prohibited (carrying ported files under file-level MPL-2.0 was
  considered and rejected in #1635).
- PawnIO is GPL-2.0 with an exception for independent programs
  communicating through its device IO control interface; the modules
  are LGPL-2.1-or-later. Calling them via IOCTLs keeps this
  repository MIT. Redistributing module blobs with an installer
  requires third-party-notice compliance (see
  `docs/specs/sensors/pawnio-interface.md`).

## Implementation PR requirements

No PR may be opened or reviewed as clean-room implementation work
unless all of the following hold (the "implementation gate" of
`docs/specs/sensors/README.md`):

1. Every consulted spec document is implementation-ready: it carries
   `Status: Implementation-ready (rev N)` at the pinned revision and
   has no unresolved `TODO(provenance)` markers. The flip from draft
   follows the status-transition checklist in
   `docs/specs/sensors/README.md`.
2. The PR uses the clean-room PR template
   (`.github/PULL_REQUEST_TEMPLATE/clean-room-sensor-implementation.md`,
   append `?template=clean-room-sensor-implementation.md&expand=1` to the
   compare URL) and completes:
   - the spec **revision pinning** statement,
   - the implementer **provenance attestation**,
   - the reviewer **attestation** (reviewers copy the checklist into
     their approval review comment).

## Contamination handling

If a prohibited source is viewed by accident in an implementer
session (mis-click, search result, pasted content):

1. Stop implementation work in that session immediately.
2. Disclose the exposure in the PR or issue (what was seen, when).
3. The contaminated session/contributor must not write or review the
   affected implementation code; restart that work cleanly.
