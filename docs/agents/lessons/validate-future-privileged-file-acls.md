---
id: LRN-20261105-validate-future-privileged-file-acls
status: promoted
cause_status: confirmed
scope: Windows privileged service installation and filesystem ACL validation
trigger: validating a directory before a privileged executable is placed there
failure_signature: an inherit-only write grant was skipped although a newly installed executable could inherit it
root_cause: effective access to an existing directory was treated as proof of access to a future child file
guardrail: validate future file inheritance at the target parent and effective access on existing ancestors; retain trusted creator-owner templates
canonical_refs: core/src/sensor_service/windows/security.rs, docs/design/windows-sensor-service.md
verification: Windows SDDL regression reproduces the skipped write grant; service boundary tests also accept standard protected ancestry
evidence: PR 2509 review and future_file_writes_are_checked_before_installation regression
revalidate_when: file placement, ACL inheritance, trusted creator policy, or installer/service ownership changes
---

# Validate Future Privileged File ACLs

An inherit-only ACE does not grant access to the directory itself, but can
grant write access to the executable installed below it. Installation-time
validation must check that future file before the service can start it.

Do not reject every inheritance template on every ancestor. Existing child
directories may block inheritance, and Windows uses CREATOR OWNER templates
for trusted creators. Check effective ancestry separately from the target
parent's file inheritance. The owner and effective-write restrictions must
still ensure that untrusted principals cannot create or replace files there.

Use native Windows descriptor parsing and a standard protected-directory
positive case alongside the rejection test; a mask-only unit test misses the
inheritance and platform-compatibility boundary.
