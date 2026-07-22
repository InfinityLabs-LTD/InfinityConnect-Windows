; Кастомные NSIS-хуки Infinity Connect, внедряемые в стандартный шаблон Tauri
; (bundle.windows.nsis.installerHooks). Только логика — UI страниц остаётся
; штатным MUI (фирменный вид задаётся сайдбаром/баннером/иконкой в конфиге).
;
; Официальный, версионно-стабильный механизм: не подменяет весь installer.nsi,
; поэтому не ломается при обновлении tauri-bundler.

!macro NSIS_HOOK_POSTINSTALL
  ; Разворачиваем окно приложения при первом запуске из установщика и
  ; помечаем, что установка прошла через фирменный установщик (для телеметрии
  ; версии в реестре — читается приложением при желании).
  WriteRegStr SHCTX "${MANUPRODUCTKEY}" "InstalledVia" "InfinityInstaller"
  WriteRegStr SHCTX "${MANUPRODUCTKEY}" "InstalledVersion" "${VERSION}"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Останавливаем ТОЛЬКО ядра из нашей папки установки ($INSTDIR), по пути, а не
  ; по имени процесса — иначе `taskkill /IM sing-box.exe` убил бы одноимённые
  ; ядра сторонних приложений (напр. Happ тоже использует sing-box.exe/xray.exe).
  nsExec::Exec 'wmic process where "ExecutablePath like \'$INSTDIR\\%\'" call terminate'
!macroend
