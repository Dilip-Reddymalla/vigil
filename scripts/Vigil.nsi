; Vigil installer — NSIS script
; Build with makensis (NSIS 3.x).

!define APPNAME "Vigil"
!define VERSION "0.1.8"
!define EXE_SERVICE "vigil-service.exe"
!define EXE_DASH "vigil-dashboard.exe"

Name "${APPNAME}"
OutFile "Vigil-Setup-${VERSION}.exe"
InstallDir "$LOCALAPPDATA\Vigil"
RequestExecutionLevel user
Unicode true

!include "MUI2.nsh"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath "$INSTDIR"

  ; Binaries
  File "vigil-service.exe"
  File "vigil-dashboard.exe"

  ; Icons / shortcuts
  CreateDirectory "$INSTDIR\icons"
  ; (add icon.ico via File if placed beside this script)

  CreateDirectory "$SMPROGRAMS\${APPNAME}"
  CreateShortCut "$SMPROGRAMS\${APPNAME}\Vigil Dashboard.lnk" "$INSTDIR\${EXE_DASH}"
  CreateShortCut "$SMPROGRAMS\${APPNAME}\Uninstall Vigil.lnk" "$INSTDIR\Uninstall.exe"
  CreateShortCut "$DESKTOP\Vigil.lnk" "$INSTDIR\${EXE_DASH}"

  ; Register auto-start (no admin needed — uses Startup folder)
  ExecWait '"$INSTDIR\${EXE_SERVICE}" --install-autostart'
  ; Start the service immediately so it is running right after install
  ; (the Startup entry above only fires on the *next* logon).
  Exec '"$INSTDIR\${EXE_SERVICE}"'

  ; Uninstaller
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "DisplayName" "${APPNAME}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "DisplayIcon" "$INSTDIR\${EXE_DASH}"
SectionEnd

Section "Uninstall"
  ; Stop/running service, clear auto-start, then delete files
  nsExec::ExecToLog '"$INSTDIR\${EXE_SERVICE}" --remove-autostart'
  ExecWait 'taskkill /F /IM ${EXE_SERVICE}'

  Delete "$SMPROGRAMS\${APPNAME}\Vigil Dashboard.lnk"
  Delete "$SMPROGRAMS\${APPNAME}\Uninstall Vigil.lnk"
  RMDir "$SMPROGRAMS\${APPNAME}"
  Delete "$DESKTOP\Vigil.lnk"

  Delete "$INSTDIR\${EXE_SERVICE}"
  Delete "$INSTDIR\${EXE_DASH}"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir /r "$INSTDIR\icons"
  RMDir "$INSTDIR"

  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}"
SectionEnd
