; Wex Launcher NSIS Installer Hooks
; Specification: wex-installer-polish
; - Prompt user on uninstall whether to delete %APPDATA%\Wex (default: NO)
; - Silent uninstall (/S) preserves user data by default
; - WEX_UNINSTALL_DELETE_DATA=1 environment variable enables data deletion in automated testing

!macro NSIS_HOOK_PREUNINSTALL
  ; Check if automated deletion is requested via environment variable
  ReadEnvStr $0 WEX_UNINSTALL_DELETE_DATA
  StrCmp $0 "1" do_cleanup

  ; In silent mode without WEX_UNINSTALL_DELETE_DATA=1, skip prompt and preserve all user data
  IfSilent skip_data_cleanup

  ; Prompt user with NO as the default button (MB_DEFBUTTON2)
  ; IDNO jumps to skip_data_cleanup
  MessageBox MB_YESNO|MB_DEFBUTTON2|MB_ICONQUESTION "Удалить пользовательские данные и настройки Wex (%APPDATA%\Wex)?" IDNO skip_data_cleanup

do_cleanup:
  ; If user explicitly clicked YES or WEX_UNINSTALL_DELETE_DATA=1:
  ReadEnvStr $1 WEX_DATA_DIR
  StrCmp $1 "" default_cleanup
  IfFileExists "$1\*.*" 0 skip_data_cleanup
  RMDir /r "$1"
  Goto skip_data_cleanup

default_cleanup:
  IfFileExists "$APPDATA\Wex\*.*" 0 skip_data_cleanup
  RMDir /r "$APPDATA\Wex"

skip_data_cleanup:
!macroend
