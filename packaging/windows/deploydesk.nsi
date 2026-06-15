Unicode true
ManifestDPIAware true
RequestExecutionLevel user

!include "MUI2.nsh"

!ifndef ROOT_DIR
!define ROOT_DIR "..\.."
!endif

!ifndef PRODUCT_VERSION
!define PRODUCT_VERSION "0.6.0"
!endif

!ifndef PRODUCT_VERSION4
!define PRODUCT_VERSION4 "0.6.0.0"
!endif

!ifndef APP_EXE
!define APP_EXE "${ROOT_DIR}\target\release\deploydesk.exe"
!endif

!ifndef OUT_DIR
!define OUT_DIR "${ROOT_DIR}\dist"
!endif

!define PRODUCT_NAME "DeployDesk"
!define PRODUCT_PUBLISHER "DeployDesk contributors"
!define PRODUCT_WEB_SITE "https://github.com"
!define INSTALL_REG_KEY "Software\DeployDesk"
!define UNINSTALL_REG_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\DeployDesk"

Name "${PRODUCT_NAME}"
OutFile "${OUT_DIR}\DeployDesk-Setup-${PRODUCT_VERSION}.exe"
InstallDir "$LOCALAPPDATA\Programs\DeployDesk"
InstallDirRegKey HKCU "${INSTALL_REG_KEY}" "InstallDir"

VIProductVersion "${PRODUCT_VERSION4}"
VIAddVersionKey "ProductName" "${PRODUCT_NAME}"
VIAddVersionKey "CompanyName" "${PRODUCT_PUBLISHER}"
VIAddVersionKey "FileDescription" "${PRODUCT_NAME} Installer"
VIAddVersionKey "FileVersion" "${PRODUCT_VERSION}"
VIAddVersionKey "ProductVersion" "${PRODUCT_VERSION}"
VIAddVersionKey "LegalCopyright" "MIT License"

!define MUI_ABORTWARNING
!define MUI_ICON "${ROOT_DIR}\src\assets\app-icon.ico"
!define MUI_UNICON "${ROOT_DIR}\src\assets\app-icon.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\deploydesk.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch DeployDesk"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${ROOT_DIR}\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Section "DeployDesk" SecMain
  SetOutPath "$INSTDIR"

  File "/oname=deploydesk.exe" "${APP_EXE}"
  File "${ROOT_DIR}\LICENSE"

  CreateDirectory "$SMPROGRAMS\DeployDesk"
  CreateShortcut "$SMPROGRAMS\DeployDesk\DeployDesk.lnk" "$INSTDIR\deploydesk.exe" "" "$INSTDIR\deploydesk.exe" 0
  CreateShortcut "$DESKTOP\DeployDesk.lnk" "$INSTDIR\deploydesk.exe" "" "$INSTDIR\deploydesk.exe" 0

  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "${INSTALL_REG_KEY}" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_REG_KEY}" "DisplayName" "${PRODUCT_NAME}"
  WriteRegStr HKCU "${UNINSTALL_REG_KEY}" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegStr HKCU "${UNINSTALL_REG_KEY}" "Publisher" "${PRODUCT_PUBLISHER}"
  WriteRegStr HKCU "${UNINSTALL_REG_KEY}" "DisplayIcon" "$INSTDIR\deploydesk.exe"
  WriteRegStr HKCU "${UNINSTALL_REG_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_REG_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "${UNINSTALL_REG_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKCU "${UNINSTALL_REG_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_REG_KEY}" "NoRepair" 1
  WriteRegDWORD HKCU "${UNINSTALL_REG_KEY}" "EstimatedSize" 19000
SectionEnd

Section "Uninstall"
  Delete "$DESKTOP\DeployDesk.lnk"
  Delete "$SMPROGRAMS\DeployDesk\DeployDesk.lnk"
  RMDir "$SMPROGRAMS\DeployDesk"

  Delete "$INSTDIR\deploydesk.exe"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  DeleteRegKey HKCU "${UNINSTALL_REG_KEY}"
  DeleteRegKey HKCU "${INSTALL_REG_KEY}"
SectionEnd
