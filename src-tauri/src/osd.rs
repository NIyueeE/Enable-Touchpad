// src-tauri/src/osd.rs
use tauri::{AppHandle, Manager, WindowBuilder, WindowUrl};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use log::{info, error};

const OSD_POOL_SIZE: usize = 3;
const OSD_DURATION_MS: u64 = 1500;

pub struct OSDManager {
    app_handle: AppHandle,
    window_pool: Mutex<VecDeque<String>>,
}

impl OSDManager {
    pub fn new(app_handle: AppHandle) -> Arc<Self> {
        let manager = Arc::new(Self {
            app_handle,
            window_pool: Mutex::new(VecDeque::with_capacity(OSD_POOL_SIZE)),
        });
        
        manager.precreate_windows();
        manager
    }

    fn precreate_windows(&self) {
        for i in 0..OSD_POOL_SIZE {
            let label = format!("osd_{}", i);
            if let Err(e) = self.create_window(&label, false) {
                info!("Failed to create OSD window: {:?}", e);
            }
            
            match self.window_pool.lock() {
                Ok(mut pool) => pool.push_back(label),
                Err(e) => error!("Failed to acquire window_pool lock: {}", e)
            }
        }
    }

    fn create_window(&self, label: &str, visible: bool) -> tauri::Result<()> {
        WindowBuilder::new(
            &self.app_handle,
            label,
            WindowUrl::App("osd.html".into())
        )
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .center()
        .inner_size(200.0, 200.0)
        .visible(visible)
        .build()?;
        
        Ok(())
    }

    pub fn show(&self, enabled: bool) {
        let label = match self.window_pool.lock() {
            Ok(mut pool) => {
                match pool.pop_front() {
                    Some(label) => label,
                    None => {
                        // Create new window if pool is empty
                        let new_label = format!("osd_dyn_{}", std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis())
                            .unwrap_or_else(|_| 0));
                        
                        if let Err(e) = self.create_window(&new_label, true) {
                            error!("Failed to create dynamic OSD window: {:?}", e);
                        }
                        new_label
                    }
                }
            },
            Err(e) => {
                error!("Failed to acquire window_pool lock: {}", e);
                // Create new window if we can't access the pool
                let new_label = format!("osd_dyn_{}", std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or_else(|_| 0));
                
                if let Err(e) = self.create_window(&new_label, true) {
                    error!("Failed to create dynamic OSD window: {:?}", e);
                }
                new_label
            }
        };

        if let Some(window) = self.app_handle.get_window(&label) {
            if let Err(e) = window.emit("update-state", enabled) {
                error!("Failed to emit update-state event: {:?}", e);
            }
            
            if let Err(e) = window.show() {
                error!("Failed to show OSD window: {:?}", e);
            }
            
            if let Err(e) = window.set_focus() {
                error!("Failed to focus OSD window: {:?}", e);
            }

            // Schedule window return to pool
            let manager_clone = self.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(OSD_DURATION_MS));
                if let Err(e) = window.hide() {
                    error!("Failed to hide OSD window: {:?}", e);
                }
                
                match manager_clone.window_pool.lock() {
                    Ok(mut pool) => pool.push_back(label),
                    Err(e) => error!("Failed to acquire window_pool lock: {}", e)
                }
            });
        }
    }

    pub fn show_permission_warning(&self) {
        // Similar to show but with different message and longer duration
        // For now, we'll just log that this was called
        info!("Permission warning requested");
    }
}

impl Clone for OSDManager {
    fn clone(&self) -> Self {
        Self {
            app_handle: self.app_handle.clone(),
            window_pool: Mutex::new(VecDeque::new()), // Create a new empty pool
        }
    }
}