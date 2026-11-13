; NSIS installer hooks for Agent Panel (D-033).
; Verified against Tauri 2 docs: bundle.windows.nsis.installerHooks.

!macro NSIS_HOOK_PREUNINSTALL
  ; Runs BEFORE files are removed, so the sidecar CLI still exists.
  ; Removes all AgentPanel.* entries from the user's Windows Terminal
  ; settings.json so no dangling "Agent settings" profile remains.
  DetailPrint "Removing Agent Panel entries from Windows Terminal settings.json..."
  ExecWait '"$INSTDIR\agent-panel-cli.exe" --strip'
!macroend
