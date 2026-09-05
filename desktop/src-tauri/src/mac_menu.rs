use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    App,
};

pub fn install(app: &App) -> tauri::Result<()> {
    // AppKit's predefined Quit calls terminate: directly, skipping Tauri's exit hook.
    let quit = MenuItem::with_id(app, "app_quit", "Quit ESP Gauge", true, Some("CmdOrCtrl+Q"))?;
    let application = Submenu::with_items(
        app,
        "ESP Gauge",
        true,
        &[
            &PredefinedMenuItem::about(app, None, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;
    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;
    let window = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &MenuItem::with_id(app, "app_open", "Open ESP Gauge", true, Some("CmdOrCtrl+1"))?,
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;
    app.set_menu(Menu::with_items(app, &[&application, &edit, &window])?)?;
    Ok(())
}
