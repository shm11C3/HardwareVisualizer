; Installer hooks for Tauri's NSIS template (bundle.windows.nsis.installerHooks).
;
; External Component Setup (ADR 0024, #2118): after the files are installed,
; offer PawnIO setup and run the application's command-line setup mode.
;
; - Interactive install: one Yes/No question, Yes by default.
; - /S, /P (passive, used by the updater), and /UPDATE: no question and no setup.
; - /EXTERNAL_COMPONENT_PAWNIO=1 runs the setup without asking; =0 skips it.
;
; The default installMode is currentUser, so the installer itself is not
; elevated; "runas" shows the one UAC prompt the setup needs. A declined
; prompt or a failed setup never fails the installation.
;
; This file is included before the language files, so strings are literal
; English like the rest of the English-only installer.

!macro NSIS_HOOK_POSTINSTALL
  Push $R0

  ClearErrors
  ${GetOptions} $CMDLINE "/EXTERNAL_COMPONENT_PAWNIO=" $R0
  ${If} ${Errors}
    StrCpy $R0 ""
    ${IfNot} ${Silent}
    ${AndIf} $PassiveMode <> 1
    ${AndIf} $UpdateMode <> 1
      MessageBox MB_YESNO|MB_ICONQUESTION|MB_DEFBUTTON1 \
        "Set up PawnIO for CPU temperature and power and motherboard sensors?$\r$\n$\r$\n\
HardwareVisualizer downloads the PawnIO driver and its sensor modules from their official releases, \
verifies them against pinned checksums, and installs only what is missing. \
Windows asks for administrator permission.$\r$\n$\r$\n\
PawnIO stays installed when you uninstall HardwareVisualizer. \
You can also set it up later from Settings > Advanced." \
        IDNO +2
      StrCpy $R0 "1"
    ${EndIf}
  ${EndIf}

  ${If} $R0 == "1"
    DetailPrint "Setting up PawnIO"
    ExecShellWait "runas" "$INSTDIR\${MAINBINARYNAME}.exe" "--external-component-setup pawnio" SW_HIDE
  ${EndIf}

  Pop $R0
!macroend
