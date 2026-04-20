use serde::{Deserialize, Serialize};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager,
};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_store::StoreExt;

// ── Data Models ──────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
struct WaterEntry {
    amount: u32,
    time: String,
}

#[derive(Serialize)]
struct TodayData {
    date: String,
    entries: Vec<WaterEntry>,
    total: u32,
    target: u32,
}

#[derive(Serialize)]
struct DayData {
    date: String,
    day_name: String,
    total: u32,
    target: u32,
}

#[derive(Serialize, Deserialize, Clone)]
struct Settings {
    daily_target: u32,
    reminder_interval: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            daily_target: 2000,
            reminder_interval: 60,
        }
    }
}

// ── Helper Functions ─────────────────────────────────────────

fn get_settings_from_store(app: &tauri::AppHandle) -> Settings {
    if let Ok(store) = app.store("water-data.json") {
        store
            .get("settings")
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default()
    } else {
        Settings::default()
    }
}

fn add_water_to_store(app: &tauri::AppHandle, amount: u32) -> Result<TodayData, String> {
    let store = app.store("water-data.json").map_err(|e| e.to_string())?;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let time = chrono::Local::now().format("%H:%M").to_string();

    let mut entries: Vec<WaterEntry> = store
        .get(&today)
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    entries.push(WaterEntry { amount, time });
    let total: u32 = entries.iter().map(|e| e.amount).sum();

    store.set(
        today.clone(),
        serde_json::to_value(&entries).unwrap(),
    );
    let _ = store.save();

    let settings = get_settings_from_store(app);

    Ok(TodayData {
        date: today,
        entries,
        total,
        target: settings.daily_target,
    })
}

// ── Tauri Commands ───────────────────────────────────────────

#[tauri::command]
fn add_water(app: tauri::AppHandle, amount: u32) -> Result<TodayData, String> {
    add_water_to_store(&app, amount)
}

#[tauri::command]
fn get_today_data(app: tauri::AppHandle) -> Result<TodayData, String> {
    let store = app.store("water-data.json").map_err(|e| e.to_string())?;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let entries: Vec<WaterEntry> = store
        .get(&today)
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    let total: u32 = entries.iter().map(|e| e.amount).sum();
    let settings = get_settings_from_store(&app);

    Ok(TodayData {
        date: today,
        entries,
        total,
        target: settings.daily_target,
    })
}

#[tauri::command]
fn get_weekly_data(app: tauri::AppHandle) -> Result<Vec<DayData>, String> {
    let store = app.store("water-data.json").map_err(|e| e.to_string())?;
    let settings = get_settings_from_store(&app);
    let today = chrono::Local::now().naive_local().date();

    let mut week = Vec::new();
    for i in (0..7i64).rev() {
        let date = today - chrono::Duration::days(i);
        let date_str = date.format("%Y-%m-%d").to_string();
        let entries: Vec<WaterEntry> = store
            .get(&date_str)
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        let total: u32 = entries.iter().map(|e| e.amount).sum();
        let day_name = date.format("%a").to_string();
        week.push(DayData {
            date: date_str,
            day_name,
            total,
            target: settings.daily_target,
        });
    }

    Ok(week)
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> Settings {
    get_settings_from_store(&app)
}

#[tauri::command]
fn update_settings(
    app: tauri::AppHandle,
    daily_target: u32,
    reminder_interval: u32,
) -> Result<Settings, String> {
    let store = app.store("water-data.json").map_err(|e| e.to_string())?;
    let settings = Settings {
        daily_target,
        reminder_interval,
    };
    store.set("settings", serde_json::to_value(&settings).unwrap());
    let _ = store.save();
    Ok(settings)
}

#[tauri::command]
fn send_reminder(app: tauri::AppHandle) -> Result<(), String> {
    app.notification()
        .builder()
        .title("💧 Water Reminder")
        .body("Time to drink water! Stay hydrated.")
        .show()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn remove_last_entry(app: tauri::AppHandle) -> Result<TodayData, String> {
    let store = app.store("water-data.json").map_err(|e| e.to_string())?;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let mut entries: Vec<WaterEntry> = store
        .get(&today)
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    entries.pop();
    let total: u32 = entries.iter().map(|e| e.amount).sum();

    store.set(
        today.clone(),
        serde_json::to_value(&entries).unwrap(),
    );
    let _ = store.save();

    let settings = get_settings_from_store(&app);

    Ok(TodayData {
        date: today,
        entries,
        total,
        target: settings.daily_target,
    })
}

// ── App Entry Point ──────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app.get_webview_window("main").map(|w| {
                let _ = w.show();
                let _ = w.set_focus();
            });
        }))
        .setup(|app| {
            // ── System Tray Menu ──
            let show_i =
                MenuItem::with_id(app, "show", "Open Water Intake", true, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let add_300_i =
                MenuItem::with_id(app, "add_300", "Add 300ml", true, None::<&str>)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[&show_i, &sep1, &add_300_i, &sep2, &quit_i],
            )?;

            // ── Build Tray Icon ──
            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("Water Intake Tracker")
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "add_300" => {
                        let _ = add_water_to_store(app, 300);
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.emit("refresh-data", ());
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        // ── Minimize to tray on close ──
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            add_water,
            get_today_data,
            get_weekly_data,
            get_settings,
            update_settings,
            send_reminder,
            remove_last_entry,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
