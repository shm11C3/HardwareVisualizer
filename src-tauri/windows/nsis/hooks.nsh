; Installer hooks for Tauri's NSIS template (bundle.windows.nsis.installerHooks).
;
; External component uninstall notice (ADR 0024, #2119): before anything is
; removed, run the installed executable's notice mode. It detects the
; external components that stay installed (PawnIO today) with the same Core
; logic as Settings, tells the user they are kept, and always exits 0.
;
; - Interactive uninstall only: /S, /P (passive), and /UPDATE (the updater
;   replacing the old version) show nothing.
; - The uninstaller runs as the user, and uninstall.exe lives in the same
;   per-user folder as the executable, so this adds no elevation path.
; - The NSIS installer does not offer PawnIO setup (ADR 0024); users who set
;   it up from Settings still need to hear that it is kept.

!macro NSIS_HOOK_PREUNINSTALL
  ${IfNot} ${Silent}
  ${AndIf} $PassiveMode <> 1
  ${AndIf} $UpdateMode <> 1
    ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --external-component-notice uninstall'
  ${EndIf}
!macroend
