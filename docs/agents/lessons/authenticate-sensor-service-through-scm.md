---
id: LRN-20261104-authenticate-sensor-service-through-scm
status: promoted
cause_status: confirmed
scope: Windows sensor-service client authentication
trigger: authenticating a privileged service from a standard-user client
failure_signature: querying the SYSTEM service process failed with Win32 error 5 from a non-administrator token
root_cause: process and token query access was assumed to follow from SCM query access
guardrail: authenticate the pipe server against SCM status and dedicated LocalSystem registration without opening its process or token
canonical_refs: core/src/sensor_service/windows/security.rs, docs/design/windows-sensor-service.md
verification: SCM queries succeed from the same non-administrator token; boundary tests reject shared-process and non-System registrations
evidence: issue 2508 implementation; read-only Spooler process and SCM query comparison on Windows
revalidate_when: the service account, process sharing policy or client authentication mechanism changes
---

# Authenticate the Sensor Service Through SCM

An unelevated client must authenticate the privileged sensor service without
depending on rights to open its process or token. During implementation,
`OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` on the existing SYSTEM Spooler
process returned access denied, while `QueryServiceConfig` and
`QueryServiceStatusEx` succeeded from the same non-administrator token.

Use the OS-reported pipe server PID, the running SCM service PID and its
dedicated LocalSystem registration. Recheck the PID and running state after
querying the registration and authenticate again on every reconnect. Keep
service creation and reconfiguration administrative; a greeting or a PID
claimed inside the application protocol is not proof of identity.
