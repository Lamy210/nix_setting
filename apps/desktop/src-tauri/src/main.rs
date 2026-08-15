#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // macOS で Finder/Spotlight 起動時の PATH 欠損を補正する
    fix_path_env::fix().ok();
    schneeforge_desktop_lib::run()
}
