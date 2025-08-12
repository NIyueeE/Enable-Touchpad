use tauri::AppHandle;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum TouchpadState {
    Enabled,
    Disabled,
}

#[allow(dead_code)]
pub struct AppState {
    pub app_handle: AppHandle,
    pub touchpad_state: Arc<Mutex<TouchpadState>>,
}

impl AppState {
    pub fn new(app_handle: &AppHandle) -> Self {
        Self {
            app_handle: app_handle.clone(),
            touchpad_state: Arc::new(Mutex::new(TouchpadState::Disabled)),
        }
    }
    
    pub fn get_touchpad_state(&self) -> TouchpadState {
        match self.touchpad_state.lock() {
            Ok(guard) => *guard,
            Err(_) => {
                eprintln!("Failed to acquire touchpad state lock");
                TouchpadState::Disabled
            }
        }
    }
    
    pub fn set_touchpad_state(&self, state: TouchpadState) {
        match self.touchpad_state.lock() {
            Ok(mut guard) => *guard = state,
            Err(_) => eprintln!("Failed to acquire touchpad state lock"),
        }
    }
}

#[allow(dead_code)]
pub type SharedState = Arc<AppState>;