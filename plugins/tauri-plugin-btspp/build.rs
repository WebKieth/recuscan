const COMMANDS: &[&str] = &["list_paired"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
