//! Tauri commands for managing native browser child webviews.
//!
//! Each browser pane gets its own `WebviewWindow` positioned over a placeholder
//! `<div>` in the main window's layout. This bypasses X-Frame-Options / CSP
//! restrictions that limit the iframe-based `BrowserPane`.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// Create a child WebviewWindow overlaid on the main window at the given
/// screen-pixel position and size.
#[tauri::command]
pub fn create_webview(
    app: AppHandle,
    id: String,
    url: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let label = format!("browser-{}", id);
    let parent = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;

    let webview_url = WebviewUrl::External(url.parse().map_err(|e| format!("invalid URL: {e}"))?);

    WebviewWindowBuilder::new(&app, &label, webview_url)
        .title("ymux browser")
        .inner_size(width, height)
        .position(x, y)
        .decorations(false)
        .always_on_top(false)
        .parent(&parent)
        .map_err(|e| format!("failed to set parent: {e}"))?
        .build()
        .map_err(|e| format!("failed to create webview window: {e}"))?;

    Ok(())
}

/// Destroy (close) the child webview with the given id.
#[tauri::command]
pub fn destroy_webview(app: AppHandle, id: String) -> Result<(), String> {
    let label = format!("browser-{}", id);
    if let Some(win) = app.get_webview_window(&label) {
        win.close().map_err(|e| format!("close failed: {e}"))?;
    }
    Ok(())
}

/// Navigate an existing child webview to a new URL.
#[tauri::command]
pub fn navigate_webview(app: AppHandle, id: String, url: String) -> Result<(), String> {
    let label = format!("browser-{}", id);
    let win = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("webview '{label}' not found"))?;

    let parsed: url::Url = url.parse().map_err(|e| format!("invalid URL: {e}"))?;
    win.navigate(parsed)
        .map_err(|e| format!("navigate failed: {e}"))?;
    Ok(())
}

/// Reposition and resize an existing child webview.
#[tauri::command]
pub fn resize_webview(
    app: AppHandle,
    id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let label = format!("browser-{}", id);
    let win = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("webview '{label}' not found"))?;

    use tauri::PhysicalPosition;
    use tauri::PhysicalSize;

    win.set_position(PhysicalPosition::new(x as i32, y as i32))
        .map_err(|e| format!("set_position failed: {e}"))?;
    win.set_size(PhysicalSize::new(width as u32, height as u32))
        .map_err(|e| format!("set_size failed: {e}"))?;
    Ok(())
}
