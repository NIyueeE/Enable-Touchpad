// src-tauri/src/commands.rs
use tauri::{command, State, Manager, WindowUrl, WindowBuilder};
use crate::core::state::{SharedState, AppSettings};

#[command]
pub fn get_settings(state: State<SharedState>) -> Result<AppSettings, String> {
    Ok(state.settings.read().clone())
}

#[command]
pub fn save_settings(settings: AppSettings, state: State<SharedState>) -> Result<(), String> {
    let mut state_settings = state.settings.write();
    *state_settings = settings;
    
    state.save_settings()?;
    Ok(())
}

#[command]
pub fn check_permissions(state: State<SharedState>) -> Result<bool, String> {
    Ok(*state.permission_granted.read())
}

#[command]
pub fn request_permissions(app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        
        Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    #[cfg(target_os = "windows")]
    {
        // Windows doesn't require special permissions
    }
    
    #[cfg(target_os = "linux")]
    {
        // Linux permissions are managed through polkit
        // May need to guide user to configure udev rules
    }
    
    Ok(())
}

pub fn open_settings_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_window("settings") {
        window.show().ok();
        window.set_focus().ok();
    } else {
        tauri::WindowBuilder::new(
            app,
            "settings",
            WindowUrl::App("settings.html".into())
        )
        .title("Touchpad Control Settings")
        .inner_size(800.0, 600.0)
        .resizable(true)
        .build()
        .ok();
    }
}