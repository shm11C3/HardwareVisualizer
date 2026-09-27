# Elevated Startup Mode

Status: accepted

Some Windows sensor providers, including PawnIO-backed CPU package temperature collection, require the process that opens the provider to have administrator privileges. We decided to add Elevated Startup Mode as an App-owned preference that restarts the whole Tauri process with Windows elevation, instead of introducing a separate elevated Core helper or Windows service in this slice. `hardviz-core` is linked into the app process, so elevating only Core is not possible without adding a new process boundary, IPC contract, installer work, and service/helper lifecycle.

## Reopening an elevated resident process

On Windows, a normal Start menu or taskbar launch must be able to reopen an
elevated process hidden by Close to Tray without another UAC prompt. The
single-instance plugin's `WM_COPYDATA` notification can be blocked by Windows
User Interface Privilege Isolation (UIPI) when the new process has medium
integrity and the resident process has high integrity. The plugin exits the new
process without checking whether the notification arrived.

The App therefore claims a named, session-local Open event before settings,
database, or Tauri startup work. Its DACL limits access to the current user,
and its medium integrity label lets that user's ordinary launcher signal an
elevated resident. The event carries only an Open request, not command-line
arguments or paths. A later launch signals the existing event and exits before
opening the database. Once Tauri setup finishes, the resident listens for the
event and uses the same window restore path as the tray's Open action.

The original plugin remains for older binary compatibility. A newly installed
binary must detect an older resident before database startup. It may use the
older notification when delivery can be confirmed; when UIPI blocks that
notification, it must stop without opening a second database owner and explain
that the older resident needs to exit from the tray before the updated binary
can take over. A running older binary cannot receive an event protocol it was
not built to listen for.

Elevating every new launcher was rejected because it would show UAC for a
routine reopen. Allowing lower-integrity `WM_COPYDATA` into the plugin window
was rejected because that handler parses the sender's payload without bounding
it by the message length. A separate elevated helper or service would add a
process boundary for a single Open signal.
