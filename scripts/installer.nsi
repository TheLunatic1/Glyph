Unicode True
SetCompressor /SOLID lzma

!ifndef APP_VERSION
  !define APP_VERSION "3.0.0"
!endif

!include "MUI2.nsh"
!include "FileFunc.nsh"

!insertmacro GetParameters
!insertmacro GetOptions

Name "Glyph"
OutFile "..\dist\Glyph-Setup-${APP_VERSION}.exe"
RequestExecutionLevel user
InstallDir "$LOCALAPPDATA\Programs\Glyph"
InstallDirRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Glyph" "InstallLocation"

ShowInstDetails nevershow
ShowUnInstDetails nevershow

Var /GLOBAL CmdParams
Var /GLOBAL IsUpdated
Var /GLOBAL ForceRun

!define MUI_ABORTWARNING
!define MUI_ICON "..\assets\logo.ico"
!define MUI_UNICON "..\assets\logo.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\Glyph.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch Glyph"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Function .onInit
  StrCpy $IsUpdated "0"
  StrCpy $ForceRun "0"
  ${GetParameters} $CmdParams

  ClearErrors
  ${GetOptions} $CmdParams "--updated" $R0
  IfErrors not_updated 0
    StrCpy $IsUpdated "1"
    SetSilent silent
  not_updated:

  ClearErrors
  ${GetOptions} $CmdParams "--force-run" $R0
  IfErrors not_forcerun 0
    StrCpy $ForceRun "1"
  not_forcerun:
FunctionEnd

Section "Install Glyph" SecMain
  SetOutPath "$INSTDIR"

  ; Give exiting app process time to flush and exit during self-update or Electron upgrade
  IfSilent 0 skip_kill
    Sleep 1500
    nsExec::ExecToStack 'taskkill /F /IM Glyph.exe'
    Pop $0
    Pop $1
    Sleep 500
  skip_kill:

  ; Clean up legacy Electron v2 runtime binaries if upgrading over v2.7.6
  Delete "$INSTDIR\*.dll"
  Delete "$INSTDIR\*.pak"
  Delete "$INSTDIR\*.dat"
  Delete "$INSTDIR\*.bin"
  Delete "$INSTDIR\vk_swiftshader_icd.json"
  Delete "$INSTDIR\LICENSE.electron.txt"
  Delete "$INSTDIR\LICENSES.chromium.html"
  RMDir /r "$INSTDIR\locales"
  RMDir /r "$INSTDIR\resources"
  RMDir /r "$INSTDIR\swiftshader"

  ; Preserve any running glyph-mcp.exe handle while swapping binaries on disk
  Delete "$INSTDIR\glyph-mcp.exe.old*"
  IfFileExists "$INSTDIR\glyph-mcp.exe" 0 no_prev_mcp
    ClearErrors
    Rename "$INSTDIR\glyph-mcp.exe" "$INSTDIR\glyph-mcp.exe.old.$HWNDPARENT"
    IfErrors 0 no_prev_mcp
      Rename "$INSTDIR\glyph-mcp.exe" "$INSTDIR\glyph-mcp.exe.old2.$HWNDPARENT"
  no_prev_mcp:

  File "/oname=Glyph.exe" "..\dist\stage\Glyph.exe"
  File "/oname=glyph-mcp.exe" "..\dist\stage\glyph-mcp.exe"
  File "/oname=logo.png" "..\assets\logo.png"
  File "/oname=logo.ico" "..\assets\logo.ico"

  CreateDirectory "$INSTDIR\resources"
  File "/oname=resources\mcp.js" "..\scripts\legacy-mcp-shim.cjs"

  WriteUninstaller "$INSTDIR\Uninstall Glyph.exe"

  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Glyph" "DisplayName" "Glyph"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Glyph" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Glyph" "Publisher" "TheLunatic1 (Salman Toha)"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Glyph" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Glyph" "DisplayIcon" "$INSTDIR\Glyph.exe,0"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Glyph" "UninstallString" '"$INSTDIR\Uninstall Glyph.exe"'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Glyph" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Glyph" "NoRepair" 1

  CreateDirectory "$SMPROGRAMS\Glyph"
  CreateShortcut "$SMPROGRAMS\Glyph\Glyph.lnk" "$INSTDIR\Glyph.exe" "" "$INSTDIR\Glyph.exe" 0
  CreateShortcut "$SMPROGRAMS\Glyph\Uninstall Glyph.lnk" "$INSTDIR\Uninstall Glyph.exe" "" "$INSTDIR\Uninstall Glyph.exe" 0
  CreateShortcut "$DESKTOP\Glyph.lnk" "$INSTDIR\Glyph.exe" "" "$INSTDIR\Glyph.exe" 0
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, i 0, i 0)'
SectionEnd

Function .onInstSuccess
  IfSilent LaunchApp
  StrCmp $IsUpdated "1" LaunchApp
  StrCmp $ForceRun "1" LaunchApp DoneLaunch
  LaunchApp:
    Exec '"$INSTDIR\Glyph.exe"'
  DoneLaunch:
FunctionEnd

Section "Uninstall"
  nsExec::ExecToStack 'taskkill /F /IM Glyph.exe'
  Pop $0
  Pop $1
  Sleep 400

  Delete "$INSTDIR\Glyph.exe"
  Delete "$INSTDIR\glyph-mcp.exe"
  Delete "$INSTDIR\glyph-mcp.exe.old*"
  Delete "$INSTDIR\logo.png"
  Delete "$INSTDIR\logo.ico"
  Delete "$INSTDIR\resources\mcp.js"
  RMDir "$INSTDIR\resources"
  Delete "$INSTDIR\Uninstall Glyph.exe"
  RMDir "$INSTDIR"

  Delete "$DESKTOP\Glyph.lnk"
  Delete "$SMPROGRAMS\Glyph\Glyph.lnk"
  Delete "$SMPROGRAMS\Glyph\Uninstall Glyph.lnk"
  RMDir "$SMPROGRAMS\Glyph"

  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Glyph"
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, i 0, i 0)'
SectionEnd
