// src-tauri/src/core/mouse_emulator.rs
use enigo::{Enigo, MouseControllable, MouseButton, Settings};
use rdev::Key;
use std::sync::{Arc, Mutex};
use crate::core::state::MouseKeybindings;
use std::time::{Duration, Instant};
use log::error;

#[derive(Debug)]
pub struct MouseEmulator {
    enigo: Mutex<Enigo>,
    move_queue: Mutex<Vec<(i32, i32)>>,
    last_move_time: Mutex<Instant>,
    key_state: Mutex<Vec<Key>>,
}

impl MouseEmulator {
    pub fn new() -> Arc<Self> {
        let emulator = Arc::new(Self {
            enigo: Mutex::new(Enigo::new(&Settings::default()).unwrap()),
            move_queue: Mutex::new(Vec::new()),
            last_move_time: Mutex::new(Instant::now()),
            key_state: Mutex::new(Vec::new()),
        });
        
        emulator.clone().start_processing();
        emulator
    }

    fn start_processing(self: Arc<Self>) {
        std::thread::spawn(move || {
            let interval = Duration::from_millis(8); // 120 FPS
            let move_threshold = Duration::from_millis(20);
            
            loop {
                let now = Instant::now();
                self.process_queue(&now);
                std::thread::sleep(interval.saturating_sub(now.elapsed()));
            }
        });
    }

    fn process_queue(&self, now: &Instant) {
        let mut queue = match self.move_queue.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to acquire move_queue lock: {}", e);
                return;
            }
        };
        
        if queue.is_empty() {
            return;
        }

        // Calculate total movement
        let (total_x, total_y) = queue.drain(..)
            .fold((0, 0), |(acc_x, acc_y), (x, y)| (acc_x + x, acc_y + y));
        
        // Only move if enough time has passed or significant movement
        if total_x != 0 || total_y != 0 {
            let last_move = match self.last_move_time.lock() {
                Ok(guard) => *guard,
                Err(e) => {
                    error!("Failed to acquire last_move_time lock: {}", e);
                    return;
                }
            };
            
            if now.duration_since(last_move) > Duration::from_millis(5) || 
               total_x.abs() > 5 || total_y.abs() > 5 
            {
                match self.enigo.lock() {
                    Ok(mut enigo) => {
                        enigo.mouse_move_relative(total_x, total_y);
                        match self.last_move_time.lock() {
                            Ok(mut last_move_time) => *last_move_time = *now,
                            Err(e) => error!("Failed to acquire last_move_time lock: {}", e)
                        }
                    },
                    Err(e) => error!("Failed to acquire enigo lock: {}", e)
                }
            }
        }
    }

    pub fn handle_key_press(&self, key: Key, bindings: &MouseKeybindings) {
        let mut key_state = match self.key_state.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to acquire key_state lock: {}", e);
                return;
            }
        };
        
        if key_state.contains(&key) {
            return; // Key already pressed
        }
        key_state.push(key);
        
        let mut queue = match self.move_queue.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to acquire move_queue lock: {}", e);
                return;
            }
        };
        
        let speed = 10; // Default speed, in a real implementation this would come from settings
        
        match key {
            k if k == bindings.up => queue.push((0, -speed as i32)),
            k if k == bindings.down => queue.push((0, speed as i32)),
            k if k == bindings.left => queue.push((-speed as i32, 0)),
            k if k == bindings.right => queue.push((speed as i32, 0)),
            k if k == bindings.left_click => {
                match self.enigo.lock() {
                    Ok(mut enigo) => enigo.mouse_click(MouseButton::Left),
                    Err(e) => error!("Failed to acquire enigo lock: {}", e)
                }
            }
            k if k == bindings.right_click => {
                match self.enigo.lock() {
                    Ok(mut enigo) => enigo.mouse_click(MouseButton::Right),
                    Err(e) => error!("Failed to acquire enigo lock: {}", e)
                }
            }
            k if k == bindings.middle_click => {
                match self.enigo.lock() {
                    Ok(mut enigo) => enigo.mouse_click(MouseButton::Middle),
                    Err(e) => error!("Failed to acquire enigo lock: {}", e)
                }
            }
            _ => {}
        }
    }

    pub fn handle_key_release(&self, key: Key) {
        let mut key_state = match self.key_state.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to acquire key_state lock: {}", e);
                return;
            }
        };
        
        if let Some(pos) = key_state.iter().position(|&k| k == key) {
            key_state.remove(pos);
        }
    }

    pub fn is_mouse_emulation_key(&self, key: &Key, bindings: &MouseKeybindings) -> bool {
        *key == bindings.up ||
        *key == bindings.down ||
        *key == bindings.left ||
        *key == bindings.right ||
        *key == bindings.left_click ||
        *key == bindings.right_click ||
        *key == bindings.middle_click
    }
}