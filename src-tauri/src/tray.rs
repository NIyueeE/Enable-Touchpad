use tauri::{
    AppHandle, CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem,
};
use crate::commands;
use crate::core::state::SharedState;
use crate::core::input_controller::PlatformTouchpadController;
use log::error;

pub fn create_tray() -> SystemTray {
    let quit = CustomMenuItem::new("quit".to_string(), "退出");
    let settings = CustomMenuItem::new("settings".to_string(), "设置");
    let enable = CustomMenuItem::new("enable".to_string(), "启用触摸板");
    let disable = CustomMenuItem::new("disable".to_string(), "禁用触摸板");
    
    let tray_menu = SystemTrayMenu::new()
        .add_item(enable)
        .add_item(disable)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(settings)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);
    
    SystemTray::new().with_menu(tray_menu)
}

pub fn handle_tray_event(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "quit" => {
                app.exit(0);
            }
            "settings" => {
                commands::open_settings_window(app);
            }
            "enable" => {
                // Enable touchpad
                if let Some(state) = app.try_state::<SharedState>() {
                    // We would need access to the touchpad controller here
                    // This is a simplified implementation - in a full implementation,
                    // we would need to store the controller in the app state
                    match state.touchpad_state.try_write() {
                        Some(mut touchpad_state) => *touchpad_state = crate::core::state::TouchpadState::Enabled,
                        None => error!("Failed to acquire touchpad_state write lock")
                    }
                }
            }
            "disable" => {
                // Disable touchpad
                if let Some(state) = app.try_state::<SharedState>() {
                    // We would need access to the touchpad controller here
                    // This is a simplified implementation - in a full implementation,
                    // we would need to store the controller in the app state
                    match state.touchpad_state.try_write() {
                        Some(mut touchpad_state) => *touchpad_state = crate::core::state::TouchpadState::Disabled,
                        None => error!("Failed to acquire touchpad_state write lock")
                    }
                }
            }
            _ => {}
        },
        _ => {}
    }
}