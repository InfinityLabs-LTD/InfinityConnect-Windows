// Прячем консольное окно в релизной сборке Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // wintun-адаптер и правка маршрутов требуют прав администратора. Если запущены
    // без них — перезапускаем себя с элевацией (UAC) и выходим. Риск №1 из ТЗ.
    //
    // ИСКЛЮЧЕНИЕ: запуск по deep-link (`infinityconnect://…`, напр. возврат со
    // входа через сайт). Тогда НЕ элевируемся — иначе каждый вход дёргал бы новый
    // UAC. Пропускаем к Tauri: single-instance передаст URL уже работающему
    // (элевированному) экземпляру и этот процесс тихо закроется. Если приложение
    // ещё не запущено — оно поднимется без прав, обработает вход и элевируется
    // при первом connect.
    #[cfg(windows)]
    {
        let is_deep_link = std::env::args().skip(1).any(|a| a.starts_with("infinityconnect://"));
        if !is_deep_link && !infinity_connect_lib::is_elevated() {
            if infinity_connect_lib::relaunch_elevated() {
                return; // элевированная копия запущена — текущий процесс завершаем
            }
            // Не удалось поднять права (пользователь отклонил UAC) — продолжаем без
            // них: логин/список серверов работают, connect выдаст понятную ошибку.
        }
    }

    infinity_connect_lib::run()
}
