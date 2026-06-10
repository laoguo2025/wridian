!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    SetShellVarContext current
    RMDir /r "$APPDATA\Wridian"
    RMDir /r "$LOCALAPPDATA\Wridian"
    ExecWait '"$SYSDIR\cmdkey.exe" /delete:custom-api-key.ai.wridian.app'
    ExecWait '"$SYSDIR\cmdkey.exe" /delete:provider:official-openai.ai.wridian.app'
    ExecWait '"$SYSDIR\cmdkey.exe" /delete:provider:official-anthropic.ai.wridian.app'
    ExecWait '"$SYSDIR\cmdkey.exe" /delete:provider:official-gemini.ai.wridian.app'
    ExecWait '"$SYSDIR\cmdkey.exe" /delete:provider:deepseek.ai.wridian.app'
    ExecWait '"$SYSDIR\cmdkey.exe" /delete:provider:glm.ai.wridian.app'
    ExecWait '"$SYSDIR\cmdkey.exe" /delete:provider:kimi.ai.wridian.app'
    ExecWait '"$SYSDIR\cmdkey.exe" /delete:provider:minimax.ai.wridian.app'
    ExecWait '"$SYSDIR\cmdkey.exe" /delete:provider:volcengine-ark.ai.wridian.app'
    ExecWait '"$SYSDIR\cmdkey.exe" /delete:provider:qwen.ai.wridian.app'
    ExecWait '"$SYSDIR\cmdkey.exe" /delete:provider:mimo.ai.wridian.app'
    ExecWait '"$SYSDIR\cmdkey.exe" /delete:provider:bailian.ai.wridian.app'
    ExecWait '"$SYSDIR\cmdkey.exe" /delete:provider:custom-openai-compatible.ai.wridian.app'
    ExecWait '"$SYSDIR\cmdkey.exe" /delete:provider:custom-anthropic-compatible.ai.wridian.app'
  ${EndIf}
!macroend
