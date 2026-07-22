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
  ; Гарантированно останавливаем ядра (xray/sing-box/hysteria) перед удалением,
  ; чтобы файлы не были заняты и деинсталляция прошла чисто.
  nsExec::Exec 'taskkill /F /IM sing-box.exe /T'
  nsExec::Exec 'taskkill /F /IM xray.exe /T'
  nsExec::Exec 'taskkill /F /IM hysteria.exe /T'
!macroend
