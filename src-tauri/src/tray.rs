use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager, Result as TauriResult,
};

pub fn setup_tray(app: &App) -> TauriResult<()> {
    let quit_item = MenuItem::with_id(app, "quit", "quit", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "settings", true, None::<&str>)?;
    let pause_item = MenuItem::with_id(app, "pause", "pause", true, None::<&str>)?;
    
    let menu = Menu::with_items(app, &[&quit_item, &settings_item, &pause_item])?;
    
    // 创建托盘图标
    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Enable Touchpad")
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(handle_tray_event)
        .build(app)?;

    Ok(())
}
fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id.as_ref() {
        "quit" => {
            app.exit(0);
        }

        _ => {}
    }
}

fn handle_tray_event(_tray: &tauri::tray::TrayIcon, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: tauri::tray::MouseButtonState::Up,
        ..
    } = event
    {
        
        // 左键点击处理逻辑
        println!("托盘图标被左键点击");
    }
}