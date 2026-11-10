; Installer hooks: the watcher is closed before its files change, and started again after an install.

Var VestigiumWatcherWasRunning

; Asks the watcher's hidden window to close, waits up to 10 s for it to save and exit, then ends it.
; If the window is still there after that, the install or uninstall stops before any file changes.
; The message follows the installer language ($LANGUAGE 1046 is Portuguese, Brazil; else English).
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
      StrCpy $1 0
      ${Do}
        Sleep 200
        FindWindow $0 "VestigiumAgentSession"
        IntOp $1 $1 + 1
        ${If} $1 >= 10
          ${ExitDo}
        ${EndIf}
      ${LoopUntil} $0 = 0
    ${EndIf}
    ${If} $0 <> 0
      ${If} $LANGUAGE = 1046
        MessageBox MB_OK|MB_ICONSTOP "O monitor do Vestigium ainda está em execução e não pôde ser encerrado, então nada foi alterado.$\r$\n$\r$\nEscolha Sair no ícone do Vestigium na bandeja do sistema e execute este instalador de novo." /SD IDOK
      ${Else}
        MessageBox MB_OK|MB_ICONSTOP "The Vestigium watcher is still running and could not be stopped, so nothing was changed.$\r$\n$\r$\nChoose Exit on the Vestigium icon in the system tray, then run this installer again." /SD IDOK
      ${EndIf}
      Abort
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
