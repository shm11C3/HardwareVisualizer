# Windows Sensor Service for Unelevated Resident Monitoring

Status: accepted

Tracking issue: [#2508](https://github.com/shm11C3/HardwareVisualizer/issues/2508).

Implementation status: service, authenticated IPC, app integration and optional
MSI setup are implemented in this change. Windows hardware/logon qualification
and signed release-artifact verification remain release gates. This amends the
service deferral in ADRs 0007 and 0024; existing explicit elevated-startup
preferences and older-resident activation compatibility remain supported.

## Context

PawnIO requires a privileged caller even after its driver is installed. Its
device ACL admits SYSTEM and Administrators; a normal user cannot open it.
Core remains linked into the Tauri process. Previously its PawnIO providers
ran inside that process, so Elevated Startup Mode elevated the entire resident
application, including its WebView, for sensor access.

The [2026-08-13 user report](https://blog.mkarakiapps.com/p/20260813-hardviz-no-uac-startup/)
describes startup UAC interrupting browsing on HardwareVisualizer 1.9.2 and
uses a highest-privilege scheduled task to avoid the prompt. This is historical
UX evidence, not a current-version reproduction. It establishes a need beyond
the earlier setup slice: ordinary startup should preserve sensor capability
without elevating the desktop application.

DP-03 requires explicit, optional component setup; DP-04 bounds background
work; DP-07 keeps collection independent of Tauri. A service boundary now earns
its lifecycle and security cost by serving that startup requirement.

## Decision

1. **Separate privileged collection from the resident application.** Add a
   dedicated, Tauri-independent Windows service executable for existing
   PawnIO-backed CPU package temperature/thermal status, package power, and
   motherboard temperature/fan readings. Reuse Core's providers and pinned
   clean-room specifications. Normal collection, ACPI fallback, UI, settings,
   EventBus and history remain in the unelevated application. The service
   never opens an application archive database or user profile.
2. **Use LocalSystem with a deliberately small interface.** SYSTEM is admitted
   by the existing PawnIO ACL and requires no stored user credentials. Do not
   broaden that ACL. LocalSystem remains a powerful trust boundary: load only
   protected, verified service/runtime artifacts, constrain the token where
   compatible, and expose typed read-only sensor operations rather than raw
   IOCTLs, register addresses, module loading or general execution. Runtime
   service requests cannot install components or perform downloads.
3. **Use authenticated local IPC.** A versioned named pipe admits authenticated
   interactive users of this machine, rejects network/anonymous clients, and
   does not grant clients pipe-instance creation or service-control rights.
   The app authenticates the server against the SCM-owned service process.
   These users may read the same machine-level sensor facts; the interface
   exposes no other user's processes, settings or history. Bound all messages,
   clients, demand, waiting and retries.
4. **Keep optionality and freshness explicit.** A single service sampling
   owner shares readings across sessions, follows leased demand and preserves
   provider minimum intervals and ecosystem mutexes. No active demand means
   no sensor polling. Service loss, incompatibility or PawnIO failure preserves
   unrelated monitoring and existing fallbacks. Never silently elevate or
   present stale readings as current; service restarts invalidate acquisition
   generations and power-delta baselines.
5. **Make installation a separate, explicit choice.** The service is an
   optional, machine-wide feature of the protected MSI installation. Initial
   install, repair and maintenance use an explicitly elevated installer path.
   Existing PawnIO installation does not authorize adding a service on upgrade.
   Previously selected service features can be maintained on upgrade. Routine
   startup, reconnect and tray activation request no elevation. NSIS/portable
   clients can use a compatible installed service but do not install one from
   their user-writable executable; otherwise partial monitoring remains.
6. **Preserve installation and user ownership.** The owning MSI upgrades,
   rolls back and removes the app-owned service; full uninstall keeps shared
   PawnIO and modules, as ADRs 0024/0026 require. Service installation does not
   change `elevatedStartupMode`. Offer existing users an explicit switch to
   normal startup after service readiness is verified, without changing their
   autostart/Close to Tray preferences or user-created scheduled tasks.
   Missing-service guidance follows an attempted visible sensor collection and
   insufficient fallback, including elevated clients upgrading from direct
   PawnIO collection. It opens the existing service section in Settings.
   Automatic sensor setup dialogs are limited to one per launch, during the
   initial screen's first ten seconds after settings/navigation become ready.
   This bounded evaluation lets the first sensor attempt settle; later failures,
   screen changes and window restoration never reopen sensor setup guidance.
   A queued dialog expires with this window; an already visible dialog remains
   until the user acts. Ongoing setup/status checks remain available in Settings.
   Once the service supplies readings, a dismissible notice makes the explicit
   normal-startup switch discoverable to elevated-startup users. Neither notice
   installs anything or changes the saved startup preference automatically.

## Alternatives Considered

- **Keep whole-app elevation or create a highest-privilege scheduled task.**
  The latter hides recurring UAC but leaves the desktop application elevated.
- **Launch an elevated helper at every app start.** This separates the WebView
  but retains the startup consent interruption. A persistent helper needs
  lifecycle and authentication work already provided by Windows services.
- **Grant ordinary users direct PawnIO access.** This widens low-level
  hardware capability beyond the narrow reading API needed by the product.
- **Use only ACPI thermal zones.** This remains a useful fallback, but cannot
  promise equivalent CPU package and motherboard coverage.
- **Move all Core work and persistence into a service.** This needlessly adds
  cross-user data ownership, database and collection-policy changes.

## Consequences

- Add a service executable, IPC boundary and installer/update lifecycle. This
  explicitly revisits the service deferral in ADR 0007 and ADR 0024's
  alternatives; their unrelated activation and external-component policies
  remain applicable.
- Default/new installations can remain unelevated. Existing elevated-startup
  users need a deliberate migration; external tasks or compatibility settings
  can still elevate a launch and must not be described as automatically fixed.
- Other providers may still lack privileged data. This proposal does not move
  SMART, all WMI operations or arbitrary hardware access into the service and
  does not promise full feature parity for every privileged provider.
- Security review and real Windows installation/token/sensor evidence are
  release requirements. Unit tests cannot prove that no UAC appears at logon.
- Ownership, transport decisions, migration and validation are developed in
  the [Design Doc](../design/windows-sensor-service.md). Current ownership is
  documented in [backend architecture](../architecture/backend.md); Windows
  logon qualification and signed-artifact verification remain release gates.
