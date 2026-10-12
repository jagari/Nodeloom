//! Nodeloom desktop shell. The UI is a plain web client of the master's
//! HTTP/WebSocket API, so there are no custom commands here.

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to run the Nodeloom desktop app");
}
