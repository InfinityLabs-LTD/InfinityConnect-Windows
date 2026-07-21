// Прячем консольное окно в релизной сборке Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // wintun-адаптер и правка маршрутов требуют прав администратора. Если запущены
    // без них — перезапускаем себя с элевацией (UAC) и выходим. Риск №1 из ТЗ.
    #[cfg(windows)]
    if !infinity_connect_lib::is_elevated() {
        if infinity_connect_lib::relaunch_elevated() {
            return; // элевированная копия запущена — текущий процесс завершаем
        }
        // Не удалось поднять права (пользователь отклонил UAC) — продолжаем без
        // них: логин/список серверов работают, connect выдаст понятную ошибку.
    }

    infinity_connect_lib::run()
}
