// Прячем консольное окно в релизной сборке Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    infinity_connect_lib::run()
}
