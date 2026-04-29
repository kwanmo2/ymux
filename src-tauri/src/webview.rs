//! Tauri commands for managing native browser child webview windows.
//!
//! Each browser pane gets its own borderless `WebviewWindow`. The frontend
//! polls the placeholder's screen position and calls `resize_webview` to keep
//! the child window glued to the layout.
//!
//! IMPORTANT: window operations are dispatched to the main thread via
//! `app.run_on_main_thread` and the commands return immediately. Calling
//! `WebviewWindowBuilder::build()` (or `set_position` / `navigate`) directly
//! from the Tauri command worker thread on Windows causes the IPC response
//! to hang because internal main-thread dispatch waits for the window
//! operation to complete, blocking the message pump from processing the
//! IPC reply.

use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

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
    eprintln!(
        "[webview] create {} url={} pos=({},{}) size=({}x{})",
        label, url, x, y, width, height
    );
    let parsed_url: url::Url = url.parse().map_err(|e| format!("invalid URL: {e}"))?;
    let app2 = app.clone();
    let label2 = label.clone();

    app.run_on_main_thread(move || {
        match WebviewWindowBuilder::new(&app2, &label2, WebviewUrl::External(parsed_url))
            .title("ymux browser")
            .inner_size(width, height)
            .position(x, y)
            .decorations(false)
            .resizable(false)
            .skip_taskbar(true)
            .always_on_top(false)
            .focused(false)
            .build()
        {
            Ok(_) => eprintln!("[webview] {} created on main thread", label2),
            Err(e) => eprintln!("[webview] {} create FAILED: {}", label2, e),
        }
    })
    .map_err(|e| format!("dispatch failed: {e}"))?;

    Ok(())
}

#[tauri::command]
pub fn destroy_webview(app: AppHandle, id: String) -> Result<(), String> {
    let label = format!("browser-{}", id);
    let app2 = app.clone();
    app.run_on_main_thread(move || {
        if let Some(win) = app2.get_webview_window(&label) {
            let _ = win.close();
        }
    })
    .map_err(|e| format!("dispatch failed: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn navigate_webview(app: AppHandle, id: String, url: String) -> Result<(), String> {
    let label = format!("browser-{}", id);
    eprintln!("[webview] navigate {} -> {}", label, url);
    let parsed: url::Url = url.parse().map_err(|e| format!("invalid URL: {e}"))?;
    let escaped = url.replace('\\', "\\\\").replace('"', "\\\"");
    let app2 = app.clone();
    let label2 = label.clone();

    app.run_on_main_thread(move || {
        if let Some(win) = app2.get_webview_window(&label2) {
            match win.navigate(parsed) {
                Ok(_) => eprintln!("[webview] {} navigate() OK", label2),
                Err(e) => eprintln!("[webview] {} navigate() failed: {}", label2, e),
            }
            // Eval fallback for cases where navigate() is silently a no-op.
            let js = format!("window.location.href = \"{}\";", escaped);
            if let Err(e) = win.eval(&js) {
                eprintln!("[webview] {} eval fallback failed: {}", label2, e);
            }
        } else {
            eprintln!("[webview] navigate: '{}' not found", label2);
        }
    })
    .map_err(|e| format!("dispatch failed: {e}"))?;

    Ok(())
}

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
    let app2 = app.clone();

    app.run_on_main_thread(move || {
        if let Some(win) = app2.get_webview_window(&label) {
            let _ = win.set_position(PhysicalPosition::new(x as i32, y as i32));
            let _ = win.set_size(PhysicalSize::new(
                width.max(1.0) as u32,
                height.max(1.0) as u32,
            ));
        }
    })
    .map_err(|e| format!("dispatch failed: {e}"))?;
    Ok(())
}
