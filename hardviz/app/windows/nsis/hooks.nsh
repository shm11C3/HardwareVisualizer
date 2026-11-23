; Installer hooks for Tauri's NSIS template (bundle.windows.nsis.installerHooks).
;
; External component uninstall notice (ADR 0024, #2119): before anything is
; removed, run the installed executable's notice mode. It detects the
; external components that stay installed (PawnIO today) with the same Core
; logic as Settings, tells the user they are kept, and always exits 0.
;
; - Interactive uninstall only: /S, /P (passive), /UPDATE (the updater
;   replacing the old version), and an installer-spawned uninstall (a
;   newer *-setup.exe removing the old version) show nothing.
; - The uninstaller runs as the user, and uninstall.exe lives in the same
;   per-user folder as the executable, so this adds no elevation path.
; - The NSIS installer does not offer PawnIO setup (ADR 0024); users who set
;   it up from Settings still need to hear that it is kept.

Var InstallerSpawnedUninstall

!macro NSIS_HOOK_PREUNINSTALL
  ; Installer-spawned uninstall (#2248): PageLeaveReinstall in Tauri's
  ; installer.nsi runs the old uninstaller as '"<uninstall.exe>" _?=<dir>',
  ; with /P and /UPDATE (if applicable) inserted before it, so _?= is always
  ; the last token on the command line. A manual upgrade with a newer
  ; *-setup.exe would otherwise show the notice mid-install. Apps & features
  ; never passes _?=, so its presence identifies an installer-spawned
  ; uninstall. Uses a dedicated var (not $0/$1) so this can't clobber a
  ; register another macro in this hook relies on.
  ${GetOptions} $CMDLINE "_?=" $InstallerSpawnedUninstall
  ${IfNot} ${Errors}
    StrCpy $InstallerSpawnedUninstall 1
  ${Else}
    StrCpy $InstallerSpawnedUninstall 0
  ${EndIf}

  ${IfNot} ${Silent}
  ${AndIf} $PassiveMode <> 1
  ${AndIf} $UpdateMode <> 1
  ${AndIf} $InstallerSpawnedUninstall <> 1
    ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --external-component-notice uninstall'
  ${EndIf}
!macroend
