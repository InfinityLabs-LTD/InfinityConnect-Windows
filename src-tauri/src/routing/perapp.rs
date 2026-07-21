//! Split-tunnel по приложениям через WFP (Windows Filtering Platform).
//!
//! **Статус: задел (Фаза 6, первая итерация).** Полноценный per-app split-tunnel
//! на Windows требует WFP-провайдера/сублейера и callout-драйвера либо фильтров
//! по пути процесса (`FWPM_CONDITION_ALE_APP_ID`) — это сотни строк FFI к
//! `fwpuclnt.dll` и, для надёжного разделения трафика, компонент режима ядра.
//! Нативного аналога Android `addAllowed/DisallowedApplication` на уровне TUN нет.
//!
//! По ТЗ на первой итерации допустима заглушка: настройки app_mode/apps
//! сохраняются и доступны в UI, но фильтр пока не устанавливается. Реальная
//! WFP-реализация — Фаза 7 (полировка) вместе с kill-switch (тоже WFP).

use crate::routing::{AppRoutingMode, RoutingSettings};

/// Применяет split-tunnel по приложениям (пока — no-op с логом намерения).
/// Возвращает true, если фильтр реально установлен (сейчас всегда false).
pub fn apply_per_app(settings: &RoutingSettings) -> bool {
    if settings.app_mode == AppRoutingMode::Off || settings.apps.is_empty() {
        return false;
    }
    // TODO(Фаза 7): WFP-фильтры по FWPM_CONDITION_ALE_APP_ID для путей из
    // settings.apps; направление в/мимо TUN по app_mode (Allow/Disallow).
    eprintln!(
        "[routing] per-app split-tunnel ({:?}, {} прил.) — WFP-фильтр будет на Фазе 7",
        settings.app_mode,
        settings.apps.len()
    );
    false
}

/// Снимает установленные WFP-фильтры (no-op пока фильтров нет).
pub fn clear_per_app() {
    // TODO(Фаза 7): удалить наши WFP-фильтры/сублейер.
}
