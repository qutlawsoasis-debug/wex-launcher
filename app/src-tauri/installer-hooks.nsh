; Wex Launcher NSIS Installer Hooks
; Specification: wex-installer-polish & wex-deeplink
; - Prompt user on uninstall whether to delete %APPDATA%\Wex (default: NO)
; - Silent uninstall (/S) preserves user data
; - Safe isolated cleanup
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
