; Tauri 默认创建开始菜单入口；这里确保普通图形安装也直接创建桌面快捷方式。
; 卸载时 Tauri 会校验快捷方式目标并安全移除它。
!macro NSIS_HOOK_POSTINSTALL
  Call CreateOrUpdateDesktopShortcut
!macroend
