!include "MUI2.nsh"
!include "FileFunc.nsh"

Name "MySystem"
OutFile "nsis-output.exe"
InstallDir "$PROGRAMFILES64\MySystem"

!define MUI_ABORTWARNING

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY      ; 这一页让用户选择安装目录
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_LANGUAGE "SimpChinese"

Section "Install"
    SetOutPath "$INSTDIR"
    File /r "..\..\dist\*.*"          ; 前端构建产物
    File "..\..\target\release\mysystem.exe"  ; Rust 可执行文件
    
    WriteUninstaller "$INSTDIR\uninstall.exe"
    
    CreateShortCut "$DESKTOP\MySystem.lnk" "$INSTDIR\mysystem.exe"
    CreateDirectory "$SMPROGRAMS\MySystem"
    CreateShortCut "$SMPROGRAMS\MySystem\MySystem.lnk" "$INSTDIR\mysystem.exe"
    CreateShortCut "$SMPROGRAMS\MySystem\卸载.lnk" "$INSTDIR\uninstall.exe"
SectionEnd

Section "Uninstall"
    Delete "$INSTDIR\mysystem.exe"
    Delete "$INSTDIR\uninstall.exe"
    RMDir /r "$INSTDIR"
    Delete "$DESKTOP\MySystem.lnk"
    RMDir /r "$SMPROGRAMS\MySystem"
SectionEnd