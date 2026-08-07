// Windows release builds use the GUI subsystem. Development builds retain a
// console so native diagnostics remain visible.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    mangodisk_lib::run();
}
