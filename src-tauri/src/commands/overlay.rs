use tauri::{AppHandle, Manager};

#[tauri::command]
pub async fn open_crop_overlay(
    app: AppHandle,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    // Close existing overlay if any
    if let Some(w) = app.get_webview_window("crop-overlay") {
        let _ = w.close();
    }

    let builder = tauri::WebviewWindowBuilder::new(
        &app,
        "crop-overlay",
        tauri::WebviewUrl::App("overlay.html".into()),
    )
    .title("Select Recording Area")
    .inner_size(width, height)
    .position(x, y)
    .decorations(false)
    .transparent(true)
    .shadow(false) // required on Windows for transparency to work (v2 enables shadows by default)
    .always_on_top(true)
    .resizable(false) // we handle resize ourselves via pointer events
    .skip_taskbar(true);

    builder
        .build()
        .map_err(|e| format!("Failed to create overlay: {e}"))?;

    Ok(())
}

#[tauri::command]
pub async fn close_crop_overlay(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("crop-overlay") {
        w.close().map_err(|e| format!("{e}"))?;
    }
    Ok(())
}
