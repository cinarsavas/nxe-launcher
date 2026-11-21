; NXE Launcher installer (NSIS 3)
; Build: makensis /DVERSION=0.12.0 /DSRC=<folder with nxe-launcher.exe> /DOUT=<setup.exe path> installer/nxe-launcher.nsi
;
; Installs for the current user only (no admin prompt) into %LOCALAPPDATA%\Programs\NXE Launcher,
; adds a Start menu entry (so Windows search finds it), an optional desktop shortcut, and an
; entry in Settings > Apps so it can be uninstalled like any other app.

Unicode true
!include "MUI2.nsh"
!include "FileFunc.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!ifndef SRC
  !define SRC "..\target\x86_64-pc-windows-gnu\release"
!endif
!ifndef OUT
  !define OUT "NXE-Launcher-Setup.exe"
!endif

!define APP "NXE Launcher"
!define EXE "nxe-launcher.exe"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\NXELauncher"

Name "${APP}"
OutFile "${OUT}"
InstallDir "$LOCALAPPDATA\Programs\${APP}"
InstallDirRegKey HKCU "${UNINST_KEY}" "InstallLocation"
RequestExecutionLevel user
SetCompressor /SOLID lzma
BrandingText "${APP} ${VERSION}"

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${APP}"
VIAddVersionKey "FileDescription" "${APP} Setup"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "MIT License"

!define MUI_ICON "..\assets\icon.ico"
!define MUI_UNICON "..\assets\icon.ico"
!define MUI_ABORTWARNING
!define MUI_WELCOMEPAGE_TITLE "Install ${APP} ${VERSION}"
!define MUI_WELCOMEPAGE_TEXT "A console-style dashboard for your emulators and PC ports.$\r$\n$\r$\nIt installs just for you (no administrator rights needed) and shows up in the Start menu and Windows search.$\r$\n$\r$\nYour name, gamerpic and play time are kept in Documents\NXE Launcher, so they are safe when you update or reinstall."
!define MUI_FINISHPAGE_RUN "$INSTDIR\${EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "Start ${APP} now"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Function .onInit
  ; Close a running copy so its exe can be replaced.
  nsExec::Exec 'taskkill /IM ${EXE} /F'
FunctionEnd

Section "${APP} (required)" SecMain
  SectionIn RO
  SetOutPath "$INSTDIR"
  File "${SRC}\${EXE}"
  File "/oname=README.md" "..\README.md"
  File "/oname=LICENSE.txt" "..\LICENSE"
  ; Folders you fill yourself.
  CreateDirectory "$INSTDIR\theme\covers"
  CreateDirectory "$INSTDIR\theme\sounds"
  CreateDirectory "$INSTDIR\theme\gamerpics"
  CreateDirectory "$INSTDIR\theme\music"

  ; Start menu entry: this is what makes Windows search list it as an app.
  CreateShortcut "$SMPROGRAMS\${APP}.lnk" "$INSTDIR\${EXE}" "" "$INSTDIR\${EXE}" 0

  ; "Run" dialog / search by exe name.
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\App Paths\${EXE}" "" "$INSTDIR\${EXE}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\App Paths\${EXE}" "Path" "$INSTDIR"

  ; Settings > Apps entry.
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "${APP}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "Publisher" "cinarsavas"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\${EXE}"
  WriteRegStr HKCU "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "URLInfoAbout" "https://cinarsavas.github.io/nxe-launcher/"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKCU "${UNINST_KEY}" "EstimatedSize" "$0"
SectionEnd

Section "Desktop shortcut" SecDesktop
  CreateShortcut "$DESKTOP\${APP}.lnk" "$INSTDIR\${EXE}" "" "$INSTDIR\${EXE}" 0
SectionEnd

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SecMain} "The launcher, plus a Start menu entry so Windows search finds it."
  !insertmacro MUI_DESCRIPTION_TEXT ${SecDesktop} "A shortcut on your desktop."
!insertmacro MUI_FUNCTION_DESCRIPTION_END

Section "Uninstall"
  nsExec::Exec 'taskkill /IM ${EXE} /F'
  Delete "$SMPROGRAMS\${APP}.lnk"
  Delete "$DESKTOP\${APP}.lnk"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\App Paths\${EXE}"
  DeleteRegKey HKCU "${UNINST_KEY}"
  Delete "$INSTDIR\${EXE}"
  Delete "$INSTDIR\nxe-launcher.old.exe"
  Delete "$INSTDIR\nxe-launcher.new.exe"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\LICENSE.txt"
  Delete "$INSTDIR\Uninstall.exe"
  ; Settings, covers, downloaded cores and ports: ask first (silent uninstall keeps them).
  IfSilent keep
  MessageBox MB_YESNO|MB_ICONQUESTION "Also delete your NXE Launcher settings, covers, downloaded cores and ports in$\r$\n$INSTDIR ?$\r$\n$\r$\nYour profile (name, gamerpic, play time) in Documents\NXE Launcher is kept either way." IDNO keep
    RMDir /r "$INSTDIR"
    Goto done
  keep:
    RMDir "$INSTDIR"
  done:
SectionEnd
