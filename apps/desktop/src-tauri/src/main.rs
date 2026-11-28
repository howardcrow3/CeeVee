#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("--export-bindings") => {
            ceevee_desktop::export_bindings().expect("could not export bindings");
        }
        Some("--check-bindings") => match ceevee_desktop::bindings_are_current() {
            Ok(true) => println!("bindings.ts is current"),
            Ok(false) => {
                eprintln!("bindings.ts is stale: run `cargo run -- --export-bindings` in apps/desktop/src-tauri");
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("could not check bindings: {e}");
                std::process::exit(1);
            }
        },
        _ => ceevee_desktop::run(),
    }
}
