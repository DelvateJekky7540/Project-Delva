use serde::Serialize;
#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{
    AppHandle, Manager, Runtime, State,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_system_info::SysInfoState;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemStats {
    cpu_usage: f32,
    ram_usage_gb: f64,
    battery_percent: Option<f32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NetworkStatus {
    wifi_connected: bool,
    wifi_name: String,
    bt_connected: bool,
    bt_name: String,
}

#[tauri::command]
fn get_system_stats(state: State<'_, SysInfoState>) -> Result<SystemStats, String> {
    let mut sysinfo = state.sysinfo.lock().map_err(|err| err.to_string())?;

    // Refresh only what we need. CPU usage is calculated from the delta between refreshes.
    sysinfo.refresh_cpu();
    sysinfo.refresh_memory();

    let cpu_usage = sysinfo.sys.global_cpu_info().cpu_usage();
    let ram_usage_gb = sysinfo.sys.used_memory() as f64 / 1024.0_f64.powi(3);

    // The plugin's Battery fields are private, so read the percentage through its
    // Serialize implementation instead of adding a direct starship-battery dependency.
    let battery_percent = sysinfo
        .batteries()
        .ok()
        .and_then(|batteries| batteries.into_iter().next())
        .and_then(|battery| serde_json::to_value(battery).ok())
        .and_then(|battery| {
            battery
                .get("state_of_charge")
                .and_then(|charge| charge.as_f64())
        })
        // starship-battery exposes state_of_charge as a 0.0..=1.0 ratio.
        .map(|charge| charge as f32 * 100.0);

    Ok(SystemStats {
        cpu_usage,
        ram_usage_gb,
        battery_percent,
    })
}

#[cfg(target_os = "windows")]
#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ThermalZoneTemperature {
    current_temperature: u32,
}

#[cfg(target_os = "windows")]
#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ThermalZoneInformation {
    temperature: u32,
}

#[cfg(target_os = "windows")]
fn valid_celsius(celsius: f32) -> Option<f32> {
    (celsius > -20.0 && celsius < 150.0).then_some(celsius)
}

#[cfg(target_os = "windows")]
fn read_cpu_temperature() -> Result<f32, String> {
    // Primary source: ACPI thermal zones, reported in tenths of a degree Kelvin.
    // This class is not exposed by every laptop, so failures fall through.
    if let Ok(wmi) = wmi::WMIConnection::with_namespace_path("ROOT\\WMI") {
        if let Ok(zones) = wmi.raw_query::<ThermalZoneTemperature>(
            "SELECT CurrentTemperature FROM MSAcpi_ThermalZoneTemperature",
        ) {
            if let Some(celsius) = zones
                .into_iter()
                .filter_map(|zone| {
                    valid_celsius(zone.current_temperature as f32 / 10.0 - 273.15)
                })
                .reduce(f32::max)
            {
                return Ok(celsius);
            }
        }
    }

    // Fallback source: performance counters, reported in degrees Kelvin.
    let wmi = wmi::WMIConnection::new().map_err(|err| err.to_string())?;
    let zones: Vec<ThermalZoneInformation> = wmi
        .raw_query(
            "SELECT Temperature FROM Win32_PerfFormattedData_Counters_ThermalZoneInformation",
        )
        .map_err(|err| err.to_string())?;

    zones
        .into_iter()
        .filter_map(|zone| valid_celsius(zone.temperature as f32 - 273.15))
        .reduce(f32::max)
        .ok_or_else(|| "No valid thermal zone temperature was reported".to_string())
}

#[cfg(not(target_os = "windows"))]
fn read_cpu_temperature() -> Result<f32, String> {
    Err("CPU temperature is only supported on Windows".to_string())
}

#[tauri::command]
async fn get_cpu_temperature() -> Result<f32, String> {
    tauri::async_runtime::spawn_blocking(read_cpu_temperature)
        .await
        .map_err(|err| err.to_string())?
}

#[cfg(target_os = "windows")]
static POWERSHELL_OUTPUT_COUNTER: AtomicU64 = AtomicU64::new(0);

#[cfg(target_os = "windows")]
fn run_powershell(command: &str) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    let sequence = POWERSHELL_OUTPUT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let output_path = std::env::temp_dir().join(format!(
        "project-delva-powershell-{}-{sequence}.txt",
        std::process::id()
    ));
    let output_file = std::fs::File::create(&output_path)
        .map_err(|err| format!("Gagal membuat file output PowerShell: {err}"))?;

    // Redirect stdout to a file instead of a pipe. Some Windows tools spawn
    // child processes that keep inherited pipe handles open, which makes
    // Command::output() wait even after PowerShell itself has exited.
    let mut child = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", command])
        .stdout(Stdio::from(output_file))
        .stderr(Stdio::null())
        // Prevent a console window from flashing every time we poll.
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|err| {
            let _ = std::fs::remove_file(&output_path);
            format!("Gagal menjalankan PowerShell: {err}")
        })?;

    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = std::fs::remove_file(&output_path);
                    return Err(format!("Perintah PowerShell timeout: {command}"));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_file(&output_path);
                return Err(format!("Gagal menunggu PowerShell: {err}"));
            }
        }
    };

    let bytes = std::fs::read(&output_path).unwrap_or_default();
    let _ = std::fs::remove_file(&output_path);

    if !status.success() {
        return Err(format!("Perintah PowerShell gagal: {command}"));
    }

    Ok(String::from_utf8_lossy(&bytes)
        .trim_start_matches('\u{feff}')
        .to_string())
}

#[cfg(target_os = "windows")]
fn parse_wifi_ssid(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        let key = key.trim();

        // BSSID/AP BSSID must not be treated as the Wi-Fi name.
        if !key.eq_ignore_ascii_case("SSID") && !key.eq_ignore_ascii_case("Nama SSID") {
            return None;
        }

        let value = value.trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

#[cfg(target_os = "windows")]
fn read_network_status() -> Result<NetworkStatus, String> {
    let wifi_output = run_powershell("netsh wlan show interfaces").unwrap_or_default();
    let wifi_name = parse_wifi_ssid(&wifi_output).unwrap_or_default();
    let wifi_connected = !wifi_name.is_empty();

    // A targeted CIM query is much faster than enumerating every Bluetooth
    // device with Get-PnpDevice inside a hidden PowerShell process.
    let bt_command = concat!(
        "Get-CimInstance -Query \"SELECT Name FROM Win32_PnPEntity ",
        "WHERE PNPClass='Bluetooth' AND Status='OK'\" ",
        "| Select-Object -First 1 -ExpandProperty Name"
    );
    let bt_name = run_powershell(bt_command)
        .unwrap_or_default()
        .trim()
        .to_string();
    let bt_connected = !bt_name.is_empty();

    Ok(NetworkStatus {
        wifi_connected,
        wifi_name,
        bt_connected,
        bt_name,
    })
}

#[cfg(not(target_os = "windows"))]
fn read_network_status() -> Result<NetworkStatus, String> {
    Ok(NetworkStatus {
        wifi_connected: false,
        wifi_name: String::new(),
        bt_connected: false,
        bt_name: String::new(),
    })
}

#[tauri::command]
async fn get_network_status() -> Result<NetworkStatus, String> {
    tauri::async_runtime::spawn_blocking(read_network_status)
        .await
        .map_err(|err| err.to_string())?
}

fn toggle_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_system_info::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_system_stats,
            get_cpu_temperature,
            get_network_status
        ])
        .setup(|app| {
            let toggle_item =
                MenuItem::with_id(app, "toggle", "Show/Hide Delva", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&toggle_item, &quit_item])?;

            let _tray = TrayIconBuilder::new()
                .icon(
                    app.default_window_icon()
                        .expect("missing default window icon")
                        .clone(),
                )
                .tooltip("Project Delva")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "toggle" => toggle_main_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
