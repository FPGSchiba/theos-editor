//! The TypeScript bindings of the IPC commands are generated from Rust by tauri-specta and
//! checked in; a changed Rust type with an old file fails here (spec §5.5).

mod generated;

use generated::is_stale;
use specta_typescript::Typescript;
use tauri::test::MockRuntime;

/// The checked-in bindings, relative to the repository root.
const BINDINGS: &str = "editor-ui/src/lib/ipc/bindings.ts";

/// The bindings as tauri-specta exports them now.
fn export() -> Result<String, Box<dyn std::error::Error>> {
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("bindings.ts");
    theos_editor::specta_builder::<MockRuntime>().export(Typescript::default(), &path)?;
    Ok(std::fs::read_to_string(&path)?)
}

// AC: CE-32.4
#[test]
fn bindings_are_current() -> Result<(), Box<dyn std::error::Error>> {
    let bindings = export()?;
    generated::check(BINDINGS, &bindings, "bindings")?;
    Ok(())
}

// AC: CE-32.4
#[test]
fn a_changed_byte_is_stale() {
    let current = b"export type IpcError = { kind: IpcErrorKind; message: string }\n";
    let mut changed = current.to_vec();
    changed[12] = b'x';
    assert!(is_stale(Some(&changed), current));
    assert!(is_stale(None, current));
    assert!(!is_stale(Some(current), current));
}
