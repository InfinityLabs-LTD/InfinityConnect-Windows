// Установщик Infinity Connect. Отключаем консольное окно в релизе.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    infinity_setup_lib::run()
}
