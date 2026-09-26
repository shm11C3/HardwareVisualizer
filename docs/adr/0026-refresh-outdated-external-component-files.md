# Refresh Outdated External Component Files

Status: accepted

Tracking issue: [#2284](https://github.com/shm11C3/HardwareVisualizer/issues/2284).

This amends decisions 1 and 4 of
[ADR 0024](0024-external-component-setup.md), decided on 2026-09-26.
Implementation status is kept in the design document linked under
Consequences.

## Context

ADR 0024 makes External Component Setup fill gaps only: a module file that
already exists is never replaced, and setup reports the component complete as
soon as every file exists. The pinned PawnIO.Modules release moves with the
sensor specification, so each pin bump leaves every machine that already has
module files on the older blobs. That includes the files an earlier setup
placed and the files users placed by hand following External Component
Guidance, which is how most stable-channel users got them. Those machines do
not receive upstream module fixes, and the blobs they load no longer match the
tag the specification was verified against.

## Decision

1. **Setup can tell a current file from an outdated one.** Core pins the
   SHA-256 of each module file in the pinned release and in every earlier
   upstream release that shipped it. A present file is *current* when it
   matches the pinned release, *outdated* when it matches an earlier release
   and not the pinned one, and *unrecognized* otherwise. A file that is newer
   than the pinned release is unrecognized by construction, because no later
   release is listed.
2. **Outdated files are replaced; nothing else is.** A refresh downloads and
   verifies the pinned modules archive as setup already does, then replaces
   only files that are outdated when they are replaced. Current and
   unrecognized files are never touched, so a newer blob placed by the user or
   another tool is not rolled back. Replacement stages the new file next to the
   old one and swaps it in atomically; a file that cannot be replaced (for
   example because it is in use) keeps its old contents and is reported. This
   replaces the "never replacing an existing file" part of ADR 0024 decision 4.
   Setup still never removes a file and never uninstalls the runtime.
3. **An update of HardwareVisualizer triggers the refresh.** When the MSI
   upgrades an existing installation, a commit custom action runs the refresh
   from the already elevated install, at every UI level, including the in-app
   updater (`/passive`) and `msiexec /qn` package-manager upgrades. Windows
   Installer runs commit actions only after the installation script has
   succeeded, so an update that rolls back never leaves refreshed files behind
   the previous version, provided no commit action that can fail runs after
   the refresh: a failing commit action starts a rollback that does not undo
   the refreshed files. The refresh itself ignores its exit code, and the
   package must keep every other commit action unable to fail or scheduled
   before it. It runs only under Program Files, like the setup action. A
   refresh maintains a
   component that is already on the machine and adds nothing new, so it does
   not need the per-install consent that decision 1 of ADR 0024 requires for
   setup. It never installs the runtime and never adds a missing module file.
   Its failure never fails the update.
4. **Settings offers the same refresh.** Outdated files make the component
   incomplete, so Settings shows that an update is available, and the existing
   setup action both fills missing files and replaces outdated ones. This
   covers NSIS users, whose updater does not run the refresh for the same
   reason the NSIS installer offers no setup, and MSI updates whose refresh
   failed.

## Alternatives Considered

- **Replace only files this application placed before.** Rejected. Only one
  pre-release shipped setup, so this would leave almost every stable-channel
  user, who placed the files by hand, on the old blobs.
- **Replace every file that differs from the pinned release.** Rejected. It
  would roll back a newer blob that the user or another tool placed, which
  contradicts the rule that setup does not undo what it did not put there.
- **Record a manifest of placed files and compare it on update.** Rejected as
  the primary mechanism. Existing installations have no manifest, so it cannot
  fix the machines this decision is about.
- **Detect outdated files on the first launch after an update and prompt.**
  Rejected as the primary mechanism. The update already runs elevated, so a
  second UAC prompt adds friction without adding consent. Settings remains the
  path when the update did not refresh.

## Consequences

- Each modules pin bump also records the per-file hashes of the new release,
  while the hashes of earlier releases stay in the catalog.
- An MSI update may download the pinned modules archive. It downloads nothing
  when no file is outdated, and a failed download leaves the update and the
  existing files unchanged.
- The PawnIO runtime is out of scope: updating it runs the upstream driver
  installer, which carries more risk than replacing module files. A later
  decision can extend the refresh to it when a runtime pin bump has to reach
  existing installations.
- Implementation slices and installer mechanics are recorded in
  [`docs/design/external-component-setup.md`](../design/external-component-setup.md).
