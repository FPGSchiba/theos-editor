//! The Theos editor: [`theos_editor::run`] with Tauri's runtime and the app menu
//! ([`theos_editor::menu`]), the app's context and the command line, `theos-editor <path>`.

fn main() -> tauri::Result<()> {
    theos_editor::run(
        theos_editor::menu::install(tauri::Builder::default()),
        tauri::generate_context!(),
        std::env::args_os(),
    )
}
