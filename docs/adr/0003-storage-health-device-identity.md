# Storage Health Device Identity

Status: proposed

Storage Health Snapshots need to attach records to the same physical storage device over time, even when OS device paths change. During review of [#1483](https://github.com/shm11C3/HardwareVisualizer/pull/1483), the original unkeyed identifier hash was called out because structured hardware serial spaces can be brute-forced cheaply. We therefore identify storage devices with a locally keyed, versioned HMAC-derived identifier and store a keyed serial hash when a serial is available, instead of storing raw serial numbers or relying only on transient device paths.

The purpose of the identity is limited to tracking the health trend of the same physical storage device across dates on the local machine. It is not intended to identify users, support cross-device synchronization, provide an externally shareable device identity, or remain globally stable outside the local app installation.

This is a local-app privacy hardening measure, not protection against full machine compromise: if both the database and the local HMAC key are exposed, the protection is weakened. It still avoids direct persistence of hardware serial numbers and makes a copied database or isolated snapshot less useful for recovering serials. If a serial is unavailable, the fallback identity may be less stable, but it still stays within the same privacy boundary.

This decision is intentionally provisional because Storage Health has not shipped yet and the identity scheme may change before release.
