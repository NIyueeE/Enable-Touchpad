// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;
mod tray;
mod commands;
mod osd;

use tauri::{Manager};
use core::state::{AppState, SharedState, TouchpadState};
use core::input_controller::PlatformTouchpadController;
use core::hotkey_manager::{HotkeyManager, HotkeyEvent};
use core::mouse_emulator::MouseEmulator;
use osd::OSDManager;
use tray::setup_tray;
use log::{info, error, warn};
use crossbeam::channel::{unbounded, Receiver};
use std::sync::Arc;

fn main() {
    if let Err(e) = simple_logger::init_with_level(log::Level::Info) {
        eprintln!("Failed to initialize logger: {}", e);
        std::process::exit(1);
    }
    info!("Starting Touchpad Control");

    if let Err(e) = tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Initialize shared state
            let state = Arc::new(AppState::new(&app_handle));
            app.manage(state.clone());

            // Initialize services
            let touchpad_controller = match PlatformTouchpadController::new() {
                Ok(controller) => controller,
                Err(e) => {
                    error!("Failed to initialize touchpad controller: {:?}", e);
                    return Err(Box::new(e) as Box<dyn std::error::Error>);
                }
            };
            
            let mouse_emulator = MouseEmulator::new();
            let osd_manager = OSDManager::new(app_handle.clone());
            
            // Create event channel
            let (hotkey_tx, hotkey_rx) = unbounded();
            
            // Start hotkey manager
            let hotkey_manager = HotkeyManager::new(
                state.clone(),
                touchpad_controller.clone(),
                mouse_emulator.clone(),
                hotkey_tx
            );
            hotkey_manager.start();

            // Start OSD event listener
            start_osd_listener(
                state.clone(),
                osd_manager.clone(),
                hotkey_rx
            );

            // Hide main window (tray-only app)
            if let Some(window) = app.get_webview_window("main") {
                if let Err(e) = window.hide() {
                    warn!("Failed to hide main window: {}", e);
                }
            }
            
            // Setup system tray
            setup_tray(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::check_permissions,
            commands::request_permissions
        ])
        .run(tauri::generate_context!())
    {
        error!("Error while running tauri application: {}", e);
        std::process::exit(1);
    }
}

fn start_osd_listener(
    state: SharedState,
    osd_manager: Arc<OSDManager>,
    rx: Receiver<HotkeyEvent>
) {
    std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            match event {
                HotkeyEvent::TouchpadEnabled => {
                    osd_manager.show(true);
                }
                HotkeyEvent::TouchpadDisabled => {
                    osd_manager.show(false);
                }
                HotkeyEvent::PermissionNeeded => {
                    // Show persistent notification
                    osd_manager.show_permission_warning();
                }
            }
        }
    });
}