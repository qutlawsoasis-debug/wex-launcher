; Wex Launcher NSIS Installer Hooks
; Specification: wex-installer-polish
; - Prompt user on uninstall whether to delete %APPDATA%\Wex (default: NO)
; - Silent uninstall (/S) preserves user data
; - Safe isolated cleanup

!macro NSIS_HOOK_PREUNINSTALL
  ; In silent mode, skip prompt and preserve all user data
  IfSilent skip_data_cleanup

  ; Prompt user with NO as the default button (MB_DEFBUTTON2)
  ; IDNO jumps to skip_data_cleanup
  MessageBox MB_YESNO|MB_DEFBUTTON2|MB_ICONQUESTION "Удалить пользовательские данные и настройки Wex (%APPDATA%\Wex)?" IDNO skip_data_cleanup

  ; If user explicitly clicked YES:
  IfFileExists "$APPDATA\Wex\*.*" 0 skip_data_cleanup
  RMDir /r "$APPDATA\Wex"

skip_data_cleanup:
!macroend
