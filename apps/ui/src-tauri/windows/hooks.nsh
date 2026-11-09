; Installer hooks: the watcher is closed before its files change, and started again after an install.

Var VestigiumWatcherWasRunning

; Asks the watcher's hidden window to close, waits up to 10 s for it to save and exit, then ends it.
!macro VestigiumStopWatcher
  StrCpy $VestigiumWatcherWasRunning 0
  FindWindow $0 "VestigiumAgentSession"
  ${If} $0 <> 0
    StrCpy $VestigiumWatcherWasRunning 1
    SendMessage $0 0x0010 0 0 /TIMEOUT=5000
    StrCpy $1 0
    ${Do}
      Sleep 200
      FindWindow $0 "VestigiumAgentSession"
      IntOp $1 $1 + 1
      ${If} $1 >= 50
        ${ExitDo}
      ${EndIf}
    ${LoopUntil} $0 = 0
    ${If} $0 <> 0
      nsExec::Exec '"$SYSDIR\taskkill.exe" /F /IM Vestigium.exe'
      Pop $0
      Sleep 500
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro VestigiumStopWatcher
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ${If} $VestigiumWatcherWasRunning = 1
    Exec '"$INSTDIR\Vestigium.exe"'
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro VestigiumStopWatcher
!macroend

; The history lives beside the program, so deleting the app data also deletes it.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    RMDir /r "$LOCALAPPDATA\Vestigium\data"
    RMDir "$LOCALAPPDATA\Vestigium"
  ${EndIf}
!macroend
