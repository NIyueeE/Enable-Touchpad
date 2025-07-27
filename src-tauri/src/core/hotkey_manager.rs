// src-tauri/src/core/hotkey_manager.rs
use rdev::{listen, Event, EventType, Key};
use std::sync::Arc;
use std::time::{Instant, Duration};
use crate::core::state::{SharedState, TouchpadState};
use crate::core::input_controller::TouchpadController;
use crate::core::mouse_emulator::MouseEmulator;
use log::{info, warn, error};
use crossbeam::channel::Sender;
use parking_lot::RwLock;
use once_cell::sync::Lazy;

pub enum HotkeyEvent {
    TouchpadEnabled,
    TouchpadDisabled,
    PermissionNeeded,
}

static KEY_PRESS_TIMES: Lazy<RwLock<Option<Instant>>> = Lazy::new(|| RwLock::new(None));
static KEY_PRESSED: Lazy<RwLock<Option<Key>>> = Lazy::new(|| RwLock::new(None));

pub struct HotkeyManager {
    state: SharedState,
    touchpad_controller: Arc<dyn TouchpadController>,
    mouse_emulator: Arc<MouseEmulator>,
    event_channel: Sender<HotkeyEvent>,
}

impl HotkeyManager {
    pub fn new(
        state: SharedState,
        touchpad_controller: Arc<dyn TouchpadController>,
        mouse_emulator: Arc<MouseEmulator>,
        event_channel: Sender<HotkeyEvent>
    ) -> Self {
        Self {
            state,
            touchpad_controller,
            mouse_emulator,
            event_channel,
        }
    }

    pub fn start(&self) {
        let state_clone = self.state.clone();
        let touchpad_clone = self.touchpad_controller.clone();
        let mouse_clone = self.mouse_emulator.clone();
        let event_tx = self.event_channel.clone();
        
        std::thread::spawn(move || {
            if let Err(e) = listen(move |event| {
                Self::process_event(
                    &event, 
                    &state_clone, 
                    &touchpad_clone, 
                    &mouse_clone,
                    &event_tx
                )
            }) {
                error!("Failed to listen to keyboard events: {:?}", e);
            }
        });
    }

    fn process_event(
        event: &Event,
        state: &SharedState,
        touchpad_controller: &Arc<dyn TouchpadController>,
        mouse_emulator: &Arc<MouseEmulator>,
        event_tx: &Sender<HotkeyEvent>
    ) {
        // Check permissions
        if !*state.permission_granted.read() {
            if let EventType::KeyPress(_) = event.event_type {
                if event_tx.send(HotkeyEvent::PermissionNeeded).is_err() {
                    error!("Failed to send PermissionNeeded event");
                }
            }
            return;
        }

        let settings = state.settings.read().clone();
        let hotkey = settings.long_press_hotkey;

        match event.event_type {
            EventType::KeyPress(key) => {
                if key == hotkey {
                    match KEY_PRESSED.try_write() {
                        Some(mut key_pressed) => *key_pressed = Some(key),
                        None => error!("Failed to acquire KEY_PRESSED write lock")
                    }
                    
                    match KEY_PRESS_TIMES.try_write() {
                        Some(mut press_times) => *press_times = Some(Instant::now()),
                        None => error!("Failed to acquire KEY_PRESS_TIMES write lock")
                    }
                    
                    match state.hotkey_pressed.try_write() {
                        Some(mut hotkey_pressed) => *hotkey_pressed = true,
                        None => error!("Failed to acquire hotkey_pressed write lock")
                    }
                }

                // Handle mouse emulation if touchpad is enabled
                let touchpad_state = *state.touchpad_state.read();
                if touchpad_state == TouchpadState::Enabled {
                    if mouse_emulator.is_mouse_emulation_key(&key, &settings.mouse_emulation_keys) {
                        mouse_emulator.handle_key_press(key, &settings.mouse_emulation_keys);
                    }
                }
            }
            EventType::KeyRelease(key) => {
                if key == hotkey {
                    match state.hotkey_pressed.try_write() {
                        Some(mut hotkey_pressed) => *hotkey_pressed = false,
                        None => error!("Failed to acquire hotkey_pressed write lock")
                    }
                    
                    let press_time = match KEY_PRESS_TIMES.read() {
                        Ok(times) => times.as_ref().map(|t| t.elapsed()),
                        Err(_) => {
                            error!("Failed to acquire KEY_PRESS_TIMES read lock");
                            None
                        }
                    };
                    
                    if let Some(duration) = press_time {
                        if duration >= Duration::from_millis(settings.long_press_duration_ms) {
                            // Long press release - disable touchpad
                            if let Err(e) = touchpad_controller.disable() {
                                error!("Failed to disable touchpad: {:?}", e);
                            }
                            
                            match state.touchpad_state.try_write() {
                                Some(mut touchpad_state) => *touchpad_state = TouchpadState::Disabled,
                                None => error!("Failed to acquire touchpad_state write lock")
                            }
                            
                            if event_tx.send(HotkeyEvent::TouchpadDisabled).is_err() {
                                error!("Failed to send TouchpadDisabled event");
                            }
                        } else {
                            // Short press - do nothing or show menu
                        }
                    }
                    
                    match KEY_PRESS_TIMES.try_write() {
                        Some(mut press_times) => *press_times = None,
                        None => error!("Failed to acquire KEY_PRESS_TIMES write lock")
                    }
                    
                    match KEY_PRESSED.try_write() {
                        Some(mut key_pressed) => *key_pressed = None,
                        None => error!("Failed to acquire KEY_PRESSED write lock")
                    }
                }

                // Release mouse emulation keys
                let touchpad_state = *state.touchpad_state.read();
                if touchpad_state == TouchpadState::Enabled {
                    if mouse_emulator.is_mouse_emulation_key(&key, &settings.mouse_emulation_keys) {
                        mouse_emulator.handle_key_release(key);
                    }
                }
            }
            _ => {}
        }

        // Check for long press activation
        let (press_time, pressed_key) = match (KEY_PRESS_TIMES.read(), KEY_PRESSED.read()) {
            (Ok(times), Ok(keys)) => (times.as_ref().cloned(), keys.as_ref().cloned()),
            _ => {
                error!("Failed to acquire read locks for KEY_PRESS_TIMES or KEY_PRESSED");
                (None, None)
            }
        };
        
        if let (Some(press_time), Some(pressed_key)) = (press_time, pressed_key) {
            if pressed_key == hotkey && press_time.elapsed() >= Duration::from_millis(settings.long_press_duration_ms) {
                match state.touchpad_state.try_write() {
                    Some(mut touchpad_state) => {
                        if *touchpad_state != TouchpadState::Enabled {
                            if let Err(e) = touchpad_controller.enable() {
                                error!("Failed to enable touchpad: {:?}", e);
                            } else {
                                *touchpad_state = TouchpadState::Enabled;
                                if event_tx.send(HotkeyEvent::TouchpadEnabled).is_err() {
                                    error!("Failed to send TouchpadEnabled event");
                                }
                            }
                        }
                    },
                    None => error!("Failed to acquire touchpad_state write lock")
                }
            }
        }
    }
}