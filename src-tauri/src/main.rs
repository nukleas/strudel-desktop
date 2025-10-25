// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audioexport;
mod chatbridge;
mod loggerbridge;
mod midibridge;
mod oscbridge;
mod tools;
use std::sync::Arc;

use loggerbridge::Logger;
use tauri::Manager;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
fn main() {
    let (async_input_transmitter_midi, async_input_receiver_midi) = mpsc::channel(1);
    let (async_output_transmitter_midi, async_output_receiver_midi) = mpsc::channel(1);
    let (async_input_transmitter_osc, async_input_receiver_osc) = mpsc::channel(1);
    let (async_output_transmitter_osc, async_output_receiver_osc) = mpsc::channel(1);

    // Initialize chat state
    let chat_state = chatbridge::init();

    let mut builder = tauri::Builder::default()
        .manage(midibridge::AsyncInputTransmit {
            inner: Mutex::new(async_input_transmitter_midi),
        })
        .manage(oscbridge::AsyncInputTransmit {
            inner: Mutex::new(async_input_transmitter_osc),
        })
        .manage(chat_state);

    // Add DevTools plugin for debugging JavaScript features (debug builds only)
    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(tauri_plugin_devtools::init());
    }

    builder
        .invoke_handler(tauri::generate_handler![
            midibridge::sendmidi,
            oscbridge::sendosc,
            chatbridge::send_chat_message,
            chatbridge::set_chat_config,
            chatbridge::get_chat_config,
            chatbridge::validate_strudel_code,
            chatbridge::load_strudel_docs,
            chatbridge::set_code_context,
            chatbridge::clear_code_context,
            chatbridge::get_chat_history,
            chatbridge::clear_chat_history,
            audioexport::export_pattern_audio
        ])
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            // Auto-open DevTools in debug builds for JavaScript debugging
            #[cfg(debug_assertions)]
            {
                window.open_devtools();
            }

            let logger = Logger {
                window: Arc::new(window),
            };
            midibridge::init(
                logger.clone(),
                async_input_receiver_midi,
                async_output_receiver_midi,
                async_output_transmitter_midi,
            );
            oscbridge::init(
                logger,
                async_input_receiver_osc,
                async_output_receiver_osc,
                async_output_transmitter_osc,
            );
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
