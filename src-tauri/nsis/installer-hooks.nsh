; NSIS installer hooks for ymux.
;
; Adds the install directory to the *user* PATH on install and removes it on
; uninstall, so ymux, ymon, ydir, ycode, y and ygit are callable from any
; terminal — mirroring what the WiX/MSI `path-env.wxs` fragment does for the
; MSI build. The MSI is the CI/default target; this exists so the NSIS bundle
; (used locally when WiX/light.exe fails) has feature parity.
;
; Implementation note: Tauri's bundled NSIS does not ship the EnVar plugin, so
; we drive PATH edits through PowerShell via the built-in nsExec plugin. Using
; [Environment]::SetEnvironmentVariable avoids the NSIS_MAX_STRLEN truncation
; trap of editing the registry value directly, dedupes entries, and broadcasts
; WM_SETTINGCHANGE so already-open shells eventually pick up the change.

!macro NSIS_HOOK_POSTINSTALL
  InitPluginsDir
  FileOpen $9 "$PLUGINSDIR\ymux-path-add.ps1" w
  FileWrite $9 "$$d = $\"$INSTDIR$\"$\r$\n"
  FileWrite $9 "$$p = [Environment]::GetEnvironmentVariable('PATH','User')$\r$\n"
  FileWrite $9 "if ([string]::IsNullOrEmpty($$p)) { $$p = '' }$\r$\n"
  FileWrite $9 "if (($$p -split ';') -notcontains $$d) { [Environment]::SetEnvironmentVariable('PATH', ($$p.TrimEnd(';') + ';' + $$d).Trim(';'), 'User') }$\r$\n"
  FileClose $9
  nsExec::ExecToLog 'powershell -NoProfile -ExecutionPolicy Bypass -File "$PLUGINSDIR\ymux-path-add.ps1"'
  Pop $0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  InitPluginsDir
  FileOpen $9 "$PLUGINSDIR\ymux-path-del.ps1" w
  FileWrite $9 "$$d = $\"$INSTDIR$\"$\r$\n"
  FileWrite $9 "$$p = [Environment]::GetEnvironmentVariable('PATH','User')$\r$\n"
  FileWrite $9 "if ($$p) { $$new = (($$p -split ';') | Where-Object { $$_ -ne $$d -and $$_ -ne '' }) -join ';'; [Environment]::SetEnvironmentVariable('PATH', $$new, 'User') }$\r$\n"
  FileClose $9
  nsExec::ExecToLog 'powershell -NoProfile -ExecutionPolicy Bypass -File "$PLUGINSDIR\ymux-path-del.ps1"'
  Pop $0
!macroend
