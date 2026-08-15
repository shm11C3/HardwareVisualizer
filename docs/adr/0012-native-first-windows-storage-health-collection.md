# Windows Storage Health Collection Source Priority

Status: accepted

## Context

Storage Health can be collected from multiple sources on Windows:

- DeviceIoControl
- MSStorageDriver_FailurePredict* WMI
- smartctl
- Storage Management CIM

Initial implementations relied on WMI with smartctl as a fallback.

In a real Windows NVMe environment, however:

- MSStorageDriver_FailurePredict* was unavailable for the NVMe devices.
- smartctl was not installed.
- Windows could still expose NVMe Health Information Log data through DeviceIoControl without elevation.

Relying on smartctl as the primary source would also make basic Storage Health
depend on installation of an external component.

## Decision

Prefer native Windows APIs for Storage Health when they can provide the required
signals.

Daily Storage Health collection uses:

1. DeviceIoControl
2. MSStorageDriver_FailurePredict* WMI
3. smartctl
4. Storage Management CIM

Live Storage Health uses only DeviceIoControl.

smartctl remains a fallback and capability-extension source rather than the
primary Windows provider.

## Rationale

- Basic Storage Health should work on a standard Windows installation.
- Native collection avoids spawning an external process.
- DeviceIoControl exposes NVMe health data without elevation on supported systems.
- smartctl remains valuable for devices or protocols not covered by the native path.
- Provider-specific differences are normalized behind the existing Storage Health model.

## Consequences

- Windows-specific provider code must be maintained.
- New smartctl capabilities should not automatically replace native collection.
- smartctl improvements should be adopted only when they fill a concrete coverage gap.
- Live collection may expose fewer devices than daily collection because it deliberately
  avoids expensive fallbacks.

## Rejected Alternative: Prefer smartctl

Using smartctl as the primary provider would simplify some protocol-specific parsing,
but would introduce an external runtime dependency for functionality that Windows can
provide natively.

This was rejected after observing an environment where smartctl was absent but native
NVMe health information was available.
