// src-tauri/src/core/state.rs
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use rdev::Key;
use std::path::PathBuf;
use tauri::AppHandle;
use std::sync::Arc;
use once_cell::sync::Lazy;
use std::collections::HashSet;
use log::error;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum TouchpadState {
    Disabled,
    Enabled,
    Enabling,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppSettings {
    pub long_press_hotkey: Key,
    pub long_press_duration_ms: u64,
    pub mouse_move_speed: u32,
    pub mouse_emulation_keys: MouseKeybindings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            long_press_hotkey: Key::F12,
            long_press_duration_ms: 300,
            mouse_move_speed: 10,
            mouse_emulation_keys: MouseKeybindings::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MouseKeybindings {
    pub up: Key,
    pub down: Key,
    pub left: Key,
    pub right: Key,
    pub left_click: Key,
    pub right_click: Key,
    pub middle_click: Key,
}

impl Default for MouseKeybindings {
    fn default() -> Self {
        Self {
            up: Key::KeyK,
            down: Key::KeyJ,
            left: Key::KeyH,
            right: Key::KeyL,
            left_click: Key::Space,
            right_click: Key::KeyR,
            middle_click: Key::KeyM,
        }
    }
}

pub struct AppState {
    pub settings: RwLock<AppSettings>,
    pub touchpad_state: RwLock<TouchpadState>,
    pub hotkey_pressed: RwLock<bool>,
    pub config_path: PathBuf,
    pub permission_granted: RwLock<bool>,
}

impl AppState {
    pub fn new(app_handle: &AppHandle) -> Self {
        let config_dir = match app_handle.path_resolver().app_config_dir() {
            Some(dir) => dir,
            None => {
                error!("Unable to get config dir, using default settings");
                std::env::temp_dir()
            }
        };
        
        std::fs::create_dir_all(&config_dir).ok();
        let config_path = config_dir.join("settings.toml");

        let settings = if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(content) => {
                    match toml::from_str(&content) {
                        Ok(settings) => settings,
                        Err(e) => {
                            error!("Failed to parse settings file: {}, using defaults", e);
                            AppSettings::default()
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to read settings file: {}, using defaults", e);
                    AppSettings::default()
                }
            }
        } else {
            AppSettings::default()
        };

        // Validate key bindings
        if let Err(e) = Self::validate_keybindings(&settings.mouse_emulation_keys) {
            error!("Invalid key bindings: {}", e);
            // Instead of panicking, we'll use default keybindings
            let default_bindings = MouseKeybindings::default();
            let mut settings_with_valid_bindings = settings;
            settings_with_valid_bindings.mouse_emulation_keys = default_bindings;
        }

        // Check permissions on startup
        let permission_granted = cfg!(target_os = "linux") || 
            Self::check_permissions().unwrap_or(false);

        Self {
            settings: RwLock::new(settings),
            touchpad_state: RwLock::new(TouchpadState::Disabled),
            hotkey_pressed: RwLock::new(false),
            config_path,
            permission_granted: RwLock::new(permission_granted),
        }
    }

    pub fn save_settings(&self) -> Result<(), String> {
        let settings = self.settings.read().clone();
        let toml_str = toml::to_string_pretty(&settings)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;
        std::fs::write(&self.config_path, toml_str)
            .map_err(|e| format!("Failed to write settings file: {}", e))?;
        Ok(())
    }

    fn validate_keybindings(bindings: &MouseKeybindings) -> Result<(), String> {
        let keys = [
            bindings.up, bindings.down, 
            bindings.left, bindings.right,
            bindings.left_click, bindings.right_click,
            bindings.middle_click
        ];
        
        let unique_keys: HashSet<_> = keys.iter().collect();
        if unique_keys.len() != keys.len() {
            return Err("按键绑定冲突".to_string());
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn check_permissions() -> Result<bool, String> {
        use objc::runtime::YES;
        use objc_foundation::NSString;
        use objc::{class, msg_send, sel, sel_impl};
        
        unsafe {
            let workspace: *mut objc::runtime::Object = msg_send![class!(NSWorkspace), sharedWorkspace];
            let options = 0;
            let running_apps: *mut objc::runtime::Object = msg_send![workspace, runningApplications];
            let count: usize = msg_send![running_apps, count];
            
            for i in 0..count {
                let app: *mut objc::runtime::Object = msg_send![running_apps, objectAtIndex: i as u64];
                let bundle_id: *mut NSString = msg_send![app, bundleIdentifier];
                
                if !bundle_id.is_null() {
                    let bundle_str: &str = (*bundle_id).as_str();
                    if bundle_str == "com.apple.TouchpadControl" {
                        let trusted: bool = msg_send![app, isTrusted];
                        return Ok(trusted);
                    }
                }
            }
        }
        Ok(false)
    }

    #[cfg(not(target_os = "macos"))]
    fn check_permissions() -> Result<bool, String> {
        // Windows and Linux don't require special permissions
        Ok(true)
    }
}

pub type SharedState = Arc<AppState>;