//! The app menu. On macOS Tauri's default menu has "Close Window" on Cmd+W, which would close the
//! window where the editor's Close tab (Mod+W, CE-32.2) is meant; the app sets [`MENU`] instead:
//! the app submenu with Quit, and Edit, whose items keep the webview's text editing keys. It has
//! no item on Cmd+W. Windows and Linux get no menu, as Tauri gives them none.
//!
//! [`MENU`] is plain data, so every OS tests it; only [`install`] differs per OS. `main` installs
//! it on the real runtime. Building it needs the macOS main thread, which no test has (Tauri's
//! mock builder turns the default menu off for that reason): a manual start on macOS checks the
//! built menu.

/// A predefined menu item of Tauri's (`tauri::menu::PredefinedMenuItem`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    /// About the app.
    About,
    /// Hide the app (Cmd+H).
    Hide,
    /// Hide the other apps (Cmd+Alt+H).
    HideOthers,
    /// Show every app.
    ShowAll,
    /// Quit the app (Cmd+Q).
    Quit,
    /// Undo (Cmd+Z).
    Undo,
    /// Redo (Cmd+Shift+Z).
    Redo,
    /// Cut (Cmd+X).
    Cut,
    /// Copy (Cmd+C).
    Copy,
    /// Paste (Cmd+V).
    Paste,
    /// Select all (Cmd+A).
    SelectAll,
    /// A line between items.
    Separator,
}

/// A submenu: its title and its items in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Submenu {
    /// The title; macOS shows the app's name for the first submenu whatever it says.
    pub title: &'static str,
    /// The items, top down.
    pub items: &'static [Item],
}

/// The app menu on macOS.
pub const MENU: &[Submenu] = &[
    Submenu {
        title: "Theos",
        items: &[
            Item::About,
            Item::Separator,
            Item::Hide,
            Item::HideOthers,
            Item::ShowAll,
            Item::Separator,
            Item::Quit,
        ],
    },
    Submenu {
        title: "Edit",
        items: &[
            Item::Undo,
            Item::Redo,
            Item::Separator,
            Item::Cut,
            Item::Copy,
            Item::Paste,
            Item::SelectAll,
        ],
    },
];

/// Sets [`MENU`] as the app menu, in place of Tauri's default one.
#[cfg(target_os = "macos")]
#[must_use]
pub fn install<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.menu(mac::build)
}

/// Windows and Linux: no menu, as Tauri gives them none.
#[cfg(not(target_os = "macos"))]
#[must_use]
pub fn install<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
}

#[cfg(target_os = "macos")]
mod mac {
    use tauri::menu::{Menu, PredefinedMenuItem, Submenu};
    use tauri::{AppHandle, Runtime};

    use super::{Item, MENU};

    /// [`MENU`] as Tauri's menu.
    pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
        let menu = Menu::new(app)?;
        for entry in MENU {
            let submenu = Submenu::new(app, entry.title, true)?;
            for item in entry.items {
                submenu.append(&predefined(app, *item)?)?;
            }
            menu.append(&submenu)?;
        }
        Ok(menu)
    }

    fn predefined<R: Runtime>(
        app: &AppHandle<R>,
        item: Item,
    ) -> tauri::Result<PredefinedMenuItem<R>> {
        match item {
            Item::About => PredefinedMenuItem::about(app, None, None),
            Item::Hide => PredefinedMenuItem::hide(app, None),
            Item::HideOthers => PredefinedMenuItem::hide_others(app, None),
            Item::ShowAll => PredefinedMenuItem::show_all(app, None),
            Item::Quit => PredefinedMenuItem::quit(app, None),
            Item::Undo => PredefinedMenuItem::undo(app, None),
            Item::Redo => PredefinedMenuItem::redo(app, None),
            Item::Cut => PredefinedMenuItem::cut(app, None),
            Item::Copy => PredefinedMenuItem::copy(app, None),
            Item::Paste => PredefinedMenuItem::paste(app, None),
            Item::SelectAll => PredefinedMenuItem::select_all(app, None),
            Item::Separator => PredefinedMenuItem::separator(app),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // AC: CE-32.2
    #[test]
    fn the_menu_quits_and_edits_text_and_has_no_close_window() {
        let menu: Vec<(&str, &[Item])> = MENU.iter().map(|s| (s.title, s.items)).collect();
        assert_eq!(
            menu,
            [
                (
                    "Theos",
                    &[
                        Item::About,
                        Item::Separator,
                        Item::Hide,
                        Item::HideOthers,
                        Item::ShowAll,
                        Item::Separator,
                        Item::Quit,
                    ][..]
                ),
                (
                    "Edit",
                    &[
                        Item::Undo,
                        Item::Redo,
                        Item::Separator,
                        Item::Cut,
                        Item::Copy,
                        Item::Paste,
                        Item::SelectAll,
                    ][..]
                ),
            ],
            "no Window submenu, so nothing sits on Cmd+W but the editor's Close tab"
        );
    }

    #[test]
    fn installing_the_menu_keeps_the_builder() {
        // The menu is built only when the app is; on macOS that needs the main thread.
        let builder = install(tauri::test::mock_builder());
        drop(builder);
    }
}
