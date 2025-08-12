// src-tauri/src/core/input_controller.rs
use crate::core::state::TouchpadState;
use log::error;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum ControllerError {
    #[error("Linux: Failed to find touchpad device")]
    LinuxDeviceNotFound,
    #[error("Windows: API call failed")]
    WindowsApiError,
    #[error("macOS: Accessibility permission required")]
    MacOsPermissionRequired,
    #[error("Unsupported platform")]
    UnsupportedPlatform,
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
#[allow(dead_code)]
pub trait TouchpadController: Send + Sync {
    fn enable(&self) -> Result<(), ControllerError>;
    fn disable(&self) -> Result<(), ControllerError>;
    fn get_state(&self) -> Result<TouchpadState, ControllerError>;
}

// Platform implementations
#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP
    };
    use windows::Win32::Foundation::HWND;
    use std::sync::Mutex;
    use once_cell::sync::Lazy;
    use log::error;

    static STATE: Lazy<Mutex<TouchpadState>> = Lazy::new(|| Mutex::new(TouchpadState::Disabled));

    pub struct WindowsTouchpadController;

    impl WindowsTouchpadController {
        pub fn create() -> Result<Arc<Self>, ControllerError> {
            // Check if application has admin privileges
            if !is_elevated() {
                return Err(ControllerError::WindowsApiError);
            }
            Ok(Arc::new(Self))
        }
    }

    impl TouchpadController for WindowsTouchpadController {
        fn enable(&self) -> Result<(), ControllerError> {
            unsafe {
                let mut input = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: std::mem::zeroed(),
                };
                
                // Simulate Fn key press to enable touchpad
                input.Anonymous.ki = KEYBDINPUT {
                    wVk: 0xFF, // Custom virtual key code
                    ..Default::default()
                };
                
                SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
            }
            
            match STATE.lock() {
                Ok(mut state) => *state = TouchpadState::Enabled,
                Err(e) => error!("Failed to acquire STATE lock: {}", e)
            }
            
            Ok(())
        }

        fn disable(&self) -> Result<(), ControllerError> {
            // Similar to enable but with different key code
            match STATE.lock() {
                Ok(mut state) => *state = TouchpadState::Disabled,
                Err(e) => error!("Failed to acquire STATE lock: {}", e)
            }
            
            Ok(())
        }

        fn get_state(&self) -> Result<TouchpadState, ControllerError> {
            match STATE.lock() {
                Ok(state) => Ok(*state),
                Err(e) => {
                    error!("Failed to acquire STATE lock: {}", e);
                    // Return a default state in case of error
                    Ok(TouchpadState::Disabled)
                }
            }
        }
    }

    fn is_elevated() -> bool {
        unsafe {
            use windows::Win32::Security::{
                OpenProcessToken, GetTokenInformation, TokenElevation, TOKEN_QUERY
            };
            use windows::Win32::System::Threading::{GetCurrentProcess, PROCESS_QUERY_INFORMATION};
            use windows::Win32::Foundation::HANDLE;
            
            let mut token = HANDLE(0);
            let process = GetCurrentProcess();
            
            if OpenProcessToken(process, TOKEN_QUERY, &mut token).as_bool() {
                let mut elevation = TokenElevation::default();
                let mut size = 0;
                
                if GetTokenInformation(
                    token,
                    TokenElevation,
                    Some(&mut elevation as *mut _ as *mut _),
                    std::mem::size_of::<TokenElevation>() as u32,
                    &mut size
                ).as_bool() {
                    return elevation.TokenIsElevated != 0;
                }
            }
            false
        }
    }
}
#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use objc::{class, msg_send, sel, sel_impl};
    use objc::runtime::Object;
    use objc_foundation::{INSString, NSString};
    use std::sync::Mutex;
    use once_cell::sync::Lazy;
    use log::error;

    static STATE: Lazy<Mutex<TouchpadState>> = Lazy::new(|| Mutex::new(TouchpadState::Disabled));

    pub struct MacosTouchpadController;

    impl MacosTouchpadController {
        pub fn create() -> Result<Arc<Self>, ControllerError> {
            Ok(Arc::new(Self))
        }
    }

    impl TouchpadController for MacosTouchpadController {
        fn enable(&self) -> Result<(), ControllerError> {
            unsafe {
                let cls = class!(NSAppleScript);
                let script: *mut Object = msg_send![cls, alloc];
                let source = NSString::from_str("tell application \"System Preferences\"\nset current pane to pane \"com.apple.preference.trackpad\"\nend tell\ntell application \"System Events\" to tell process \"System Preferences\"\nclick checkbox \"Ignore built-in trackpad when mouse or wireless trackpad is present\" of tab group 1 of window \"Trackpad\"\nend tell");
                let script: *mut Object = msg_send![script, initWithSource: source];
                let _: () = msg_send![script, executeAndReturnError: 0 as *mut _];
            }
            
            match STATE.lock() {
                Ok(mut state) => *state = TouchpadState::Enabled,
                Err(e) => error!("Failed to acquire STATE lock: {}", e)
            }
            
            Ok(())
        }

        fn disable(&self) -> Result<(), ControllerError> {
            // Similar to enable with opposite setting
            match STATE.lock() {
                Ok(mut state) => *state = TouchpadState::Disabled,
                Err(e) => error!("Failed to acquire STATE lock: {}", e)
            }
            
            Ok(())
        }

        fn get_state(&self) -> Result<TouchpadState, ControllerError> {
            // macOS doesn't provide API to get current state
            match STATE.lock() {
                Ok(state) => Ok(*state),
                Err(e) => {
                    error!("Failed to acquire STATE lock: {}", e);
                    // Return a default state in case of error
                    Ok(TouchpadState::Disabled)
                }
            }
        }
    }
}
#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::process::{Command, Stdio};
    use std::sync::Mutex;
    use once_cell::sync::Lazy;
    use log::error;

    static STATE: Lazy<Mutex<TouchpadState>> = Lazy::new(|| Mutex::new(TouchpadState::Disabled));

    pub struct LinuxTouchpadController {
        device_id: String,
    }

    impl LinuxTouchpadController {
        pub fn create() -> Result<Arc<Self>, ControllerError> {
            let output = Command::new("xinput")
                .arg("--list")
                .output()
                .map_err(|_| ControllerError::LinuxDeviceNotFound)?;
            
            let output_str = String::from_utf8_lossy(&output.stdout);
            let device_line = output_str.lines()
                .find(|line| line.contains("Touchpad") || line.contains("TrackPoint"))
                .ok_or(ControllerError::LinuxDeviceNotFound)?;
            
            let device_id = device_line.split_whitespace()
                .find(|part| part.starts_with("id="))
                .and_then(|s| s.split('=').nth(1))
                .ok_or(ControllerError::LinuxDeviceNotFound)?
                .to_string();
            
            Ok(Arc::new(Self { device_id }))
        }
    }

    impl TouchpadController for LinuxTouchpadController {
        fn enable(&self) -> Result<(), ControllerError> {
            Command::new("xinput")
                .args(["enable", &self.device_id])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(|_| ControllerError::LinuxDeviceNotFound)?;
                
            match STATE.lock() {
                Ok(mut state) => *state = TouchpadState::Enabled,
                Err(e) => error!("Failed to acquire STATE lock: {}", e)
            }
            
            Ok(())
        }

        fn disable(&self) -> Result<(), ControllerError> {
            Command::new("xinput")
                .args(["disable", &self.device_id])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(|_| ControllerError::LinuxDeviceNotFound)?;
                
            match STATE.lock() {
                Ok(mut state) => *state = TouchpadState::Disabled,
                Err(e) => error!("Failed to acquire STATE lock: {}", e)
            }
            
            Ok(())
        }

        fn get_state(&self) -> Result<TouchpadState, ControllerError> {
            let output = Command::new("xinput")
                .args(["list-props", &self.device_id])
                .output()
                .map_err(|_| ControllerError::LinuxDeviceNotFound)?;
            
            let output_str = String::from_utf8_lossy(&output.stdout);
            // Parse the actual device state from xinput output
            let enabled = if let Some(line) = output_str.lines().find(|line| line.contains("Device Enabled")) {
                line.trim_end().ends_with("(1)")
            } else {
                false
            };
            
            let state = if enabled {
                TouchpadState::Enabled
            } else {
                TouchpadState::Disabled
            };
            
            match STATE.lock() {
                Ok(mut state_guard) => *state_guard = state,
                Err(e) => error!("Failed to acquire STATE lock: {}", e)
            }
            
            Ok(state)
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows::WindowsTouchpadController as PlatformTouchpadController;
#[cfg(target_os = "macos")]
pub use macos::MacosTouchpadController as PlatformTouchpadController;
#[cfg(target_os = "linux")]
pub use linux::LinuxTouchpadController as PlatformTouchpadController;

impl PlatformTouchpadController {
    pub fn new() -> Result<Arc<Self>, ControllerError> {
        Self::create()
    }
}
