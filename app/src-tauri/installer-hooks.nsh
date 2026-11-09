; Wex Launcher NSIS Installer Hooks
; Specification: wex-installer-polish, wex-install-upgrade & wex-deeplink
; - Prompt user on uninstall whether to delete %APPDATA%\Wex (default: NO)
; - Silent uninstall (/S) preserves user data by default
; - WEX_UNINSTALL_DELETE_DATA=1 environment variable enables data deletion in automated testing
; - URI scheme registration: wex:// and aura://

!macro NSIS_HOOK_POSTINSTALL
  ; Register wex:// URI scheme protocol handler
  WriteRegStr HKCU "Software\Classes\wex" "" "URL:Wex Protocol"
  WriteRegStr HKCU "Software\Classes\wex" "URL Protocol" ""
  WriteRegStr HKCU "Software\Classes\wex\DefaultIcon" "" "$INSTDIR\Wex.exe,0"
  WriteRegStr HKCU "Software\Classes\wex\shell\open\command" "" '"$INSTDIR\Wex.exe" "%1"'

  ; Register legacy aura:// URI scheme protocol handler
  WriteRegStr HKCU "Software\Classes\aura" "" "URL:Wex Protocol"
  WriteRegStr HKCU "Software\Classes\aura" "URL Protocol" ""
  WriteRegStr HKCU "Software\Classes\aura\DefaultIcon" "" "$INSTDIR\Wex.exe,0"
  WriteRegStr HKCU "Software\Classes\aura\shell\open\command" "" '"$INSTDIR\Wex.exe" "%1"'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Clean up URI scheme associations
  DeleteRegKey HKCU "Software\Classes\wex"
  DeleteRegKey HKCU "Software\Classes\aura"

  ; Determine target data directory to prompt / delete
  ; Default target is %APPDATA%\Wex
  StrCpy $2 "$APPDATA\Wex"

  ; Check if custom data directory is specified via WEX_DATA_DIR
  ReadEnvStr $1 WEX_DATA_DIR
  StrCmp $1 "" target_selected

  ; Require Wex ownership marker in custom directory (config.json or instances folder)
  ; to prevent wiping root of disk, user profile, or shared directories
  IfFileExists "$1\config.json" use_custom_target 0
  IfFileExists "$1\instances\*.*" use_custom_target 0
  ; Not a verified Wex directory, ignore custom path and stick to default
  Goto target_selected

use_custom_target:
  StrCpy $2 "$1"

target_selected:
  ; Ensure target directory actually exists before prompting
  IfFileExists "$2\*.*" 0 skip_data_cleanup

  ; Check if automated deletion is requested via environment variable
  ReadEnvStr $0 WEX_UNINSTALL_DELETE_DATA
  StrCmp $0 "1" do_cleanup

  ; In silent mode without WEX_UNINSTALL_DELETE_DATA=1, skip prompt and preserve all user data
  IfSilent skip_data_cleanup

  ; Prompt user with the ACTUAL directory path shown in the message box (default: NO)
  ; IDNO jumps to skip_data_cleanup
  MessageBox MB_YESNO|MB_DEFBUTTON2|MB_ICONQUESTION "Удалить пользовательские данные и настройки Wex ($2)?" IDNO skip_data_cleanup

do_cleanup:
  ; Guard: if target is custom, re-verify ownership marker before recursive deletion
  StrCmp $2 "$APPDATA\Wex" perform_cleanup
  IfFileExists "$2\config.json" perform_cleanup 0
  IfFileExists "$2\instances\*.*" perform_cleanup 0
  Goto skip_data_cleanup

perform_cleanup:
  RMDir /r "$2"

skip_data_cleanup:
!macroend
