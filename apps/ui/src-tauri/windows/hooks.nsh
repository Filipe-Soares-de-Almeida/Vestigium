; Installer hooks. The install is per machine and runs elevated, so everything that belongs to the signed-in
; user (the monitor, the autostart entry, the history, an earlier per-user install) is done in that user's
; own context, never with the installer's token.

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

; Runs cmd.exe with a command line as the signed-in user, not with the installer's elevated token.
; Nothing runs when no one is signed in. The line must stay under 1000 characters once $INSTDIR is expanded.
!macro VestigiumRunAsUser line
  nsis_tauri_utils::RunAsUser "$SYSDIR\cmd.exe" '/d /s /c "${line}" >nul 2>&1'
  Pop $2
!macroend

!define VESTIGIUM_RUN_KEY 'HKCU\Software\Microsoft\Windows\CurrentVersion\Run'
!define VESTIGIUM_OLD_DIR '%LOCALAPPDATA%\Vestigium'

; An earlier version installed per user, in %LOCALAPPDATA%\Vestigium, a folder that user can write. The
; elevated helper in it could be swapped, so it goes: its programs, its uninstall entry and its shortcuts.
; The history in data\ stays, and a start-at-logon entry that names the old monitor is pointed at the new one.

; Before the new files go in: ends a window of the old install that is still open, since it locks its files.
!macro VestigiumEndOldWindow
  !insertmacro VestigiumRunAsUser 'if exist "${VESTIGIUM_OLD_DIR}\Vestigium.Window.exe" if /i not "${VESTIGIUM_OLD_DIR}"=="$INSTDIR" taskkill /f /im Vestigium.Window.exe'
!macroend

; After the new files are in: removes the rest of the old install.
!macro VestigiumRemoveOldInstall
  !insertmacro VestigiumRunAsUser 'if exist "${VESTIGIUM_OLD_DIR}\Vestigium.Window.exe" if /i not "${VESTIGIUM_OLD_DIR}"=="$INSTDIR" (reg query "${VESTIGIUM_RUN_KEY}" /v Vestigium | find /i "${VESTIGIUM_OLD_DIR}\Vestigium.exe" && reg add "${VESTIGIUM_RUN_KEY}" /v Vestigium /t REG_SZ /d "\"$INSTDIR\Vestigium.exe\"" /f & cd /d "${VESTIGIUM_OLD_DIR}" && del /f /q Vestigium.Window.exe Vestigium.exe Vestigium.Helper.exe PresentMon-*-x64.exe uninstall.exe & cd /d "%LOCALAPPDATA%" & rmdir /s /q Vestigium\licenses & rmdir Vestigium & reg delete "HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Vestigium" /f & reg delete HKCU\Software\vestigium\Vestigium /f & del /f /q "%APPDATA%\Microsoft\Windows\Start Menu\Programs\Vestigium.lnk" "%USERPROFILE%\Desktop\Vestigium.lnk")'
!macroend

; An update installs no shortcuts, so one is made when the all-users Start menu has none yet.
!macro VestigiumEnsureStartMenuShortcut
  ${If} $NoShortcutMode <> 1
  ${AndIfNot} ${FileExists} "$SMPROGRAMS\${PRODUCTNAME}.lnk"
    StrCpy $R9 $UpdateMode
    StrCpy $UpdateMode 0
    Call CreateOrUpdateStartMenuShortcut
    StrCpy $UpdateMode $R9
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro VestigiumStopWatcher
  !insertmacro VestigiumEndOldWindow
!macroend

; The monitor starts again as the signed-in user, only if it was running before the install.
!macro NSIS_HOOK_POSTINSTALL
  !insertmacro VestigiumRemoveOldInstall
  !insertmacro VestigiumEnsureStartMenuShortcut
  ${If} $VestigiumWatcherWasRunning = 1
    nsis_tauri_utils::RunAsUser "$INSTDIR\Vestigium.exe" ""
    Pop $2
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro VestigiumStopWatcher
!macroend

; A plain uninstall removes the start-at-logon entry when it names this install; an update keeps it.
; With "delete the application data" ticked, the history and the window's files of the signed-in user go too.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    !insertmacro VestigiumRunAsUser 'reg query "${VESTIGIUM_RUN_KEY}" /v Vestigium | find /i "$INSTDIR\Vestigium.exe" && reg delete "${VESTIGIUM_RUN_KEY}" /v Vestigium /f'
  ${EndIf}
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    !insertmacro VestigiumRunAsUser 'rmdir /s /q "${VESTIGIUM_OLD_DIR}\data" & rmdir "${VESTIGIUM_OLD_DIR}" & rmdir /s /q "%LOCALAPPDATA%\app.vestigium" & rmdir /s /q "%APPDATA%\app.vestigium"'
  ${EndIf}
!macroend
