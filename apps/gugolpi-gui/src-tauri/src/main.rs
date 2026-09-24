//! Binario de la GUI: sin consola en Windows en release.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    gugolpi_gui_lib::run();
}
