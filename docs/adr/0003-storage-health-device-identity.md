# Storage Health Device Identity

Status: accepted

Storage Health Records need to attach records to the same physical storage device over time, even when OS device paths change. During review of [#1483](https://github.com/shm11C3/HardwareVisualizer/pull/1483), the original unkeyed identifier hash was called out because structured hardware serial spaces can be brute-forced cheaply. We therefore identify storage devices with a locally keyed, versioned HMAC-derived identifier and store a keyed serial hash when a serial is available, instead of storing raw serial numbers or relying only on transient device paths.

The purpose of the identity is limited to tracking the health trend of the same physical storage device across dates on the local machine. It is not intended to identify users, support cross-device synchronization, provide an externally shareable device identity, or remain globally stable outside the local app installation.

This is a local-app privacy hardening measure, not protection against full machine compromise: if both the database and the local HMAC key are exposed, the protection is weakened. It still avoids direct persistence of hardware serial numbers and makes a copied database or isolated snapshot less useful for recovering serials. If a serial is unavailable, the fallback identity may be less stable, but it still stays within the same privacy boundary.

## Decision

Keep the current locally keyed HMAC-derived identity scheme for the first Storage Health release:

- Store a per-installation `storageHealthIdentity.hashKey` in `settings.json`.
- When a normalized serial number is available, derive the device ID from the storage protocol and serial number.
- Store a separate keyed serial hash when a serial number is available so future migrations can reason about serial-backed records without persisting the raw serial.
- When no serial number is available, derive a best-effort fallback identity from protocol or device type, model, capacity, and device path.
- Prefix derived identifiers with explicit versions so future schemes can be introduced without overloading old values.

This keeps the identity local to one app installation and preserves enough stability for daily Storage Health Records without turning hardware serials into persisted product data.

## Alternatives Considered

- Store raw serial numbers locally: rejected because it widens the persisted data surface for a value that is not needed by the product UI.
- Use unkeyed hashes: rejected because structured serial spaces can be brute-forced cheaply.
- Use opaque local UUID mappings: rejected for the first release because rediscovery and migration rules would add state without improving the privacy boundary.
- Use fallback-only protocol/model/capacity/device-path identities: rejected because this is less stable for normal serial-backed devices and can split history when paths change.
- Introduce a hybrid migration now: rejected because Storage Health has not shipped and the current implementation already matches the accepted release behavior.

## Consequences

Existing development data that was written with the same local hash key remains readable. If the key is reset, lost, or moved separately from the database, later records enter a new local identity namespace and are not automatically linked back to earlier records.

Serial-backed device identities should remain stable across OS device path changes. Serial-less fallback identities are explicitly best-effort: they may split records when the OS path changes and may be less precise for identical devices with the same model and capacity.
