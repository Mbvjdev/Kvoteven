mod cli;
mod display_state;
pub mod models;
pub mod providers;
pub mod runner;
pub mod service;
mod smoke;
pub mod vault;

use models::{AppError, AppStatus, CodexSource, Provider, Snapshot};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use service::Service;
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Mutex,
};
use std::time::{Duration, Instant};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, State,
};

struct RuntimeState {
    smoke: bool,
    seen: AtomicU8,
    inner: Mutex<display_state::DisplayState<Provider, CodexSource, Snapshot>>,
}
struct MutationGuard<'a>(&'a RuntimeState);
impl Drop for MutationGuard<'_> {
    fn drop(&mut self) {
        self.0.inner.lock().unwrap().end_mutation();
    }
}
impl RuntimeState {
    fn mutation(&self, app: &tauri::AppHandle) -> Result<MutationGuard<'_>, AppError> {
        let mut inner = self.inner.lock().unwrap();
        if !inner.start_mutation() {
            return Err(fixed_error("busy"));
        }
        paint_tray(app, None);
        Ok(MutationGuard(self))
    }
}
fn fixed_error(code: &str) -> AppError {
    AppError { code: code.into() }
}
fn show(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
fn fresh(snapshot: &Snapshot) -> bool {
    let now = chrono::Utc::now();
    let Ok(fetched) = chrono::DateTime::parse_from_rfc3339(&snapshot.fetched_at) else {
        return false;
    };
    if !(-60..=600).contains(&(now - fetched.with_timezone(&chrono::Utc)).num_seconds()) {
        return false;
    }
    snapshot
        .resets_at
        .as_ref()
        .map(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|t| t > now)
                .unwrap_or(false)
        })
        .unwrap_or(true)
}
fn paint_tray(app: &tauri::AppHandle, snapshot: Option<&Snapshot>) {
    let Some(tray) = app.tray_by_id("quota") else {
        return;
    };
    let snapshot = snapshot.filter(|s| fresh(s));
    let title = snapshot
        .map(|s| {
            let data = s
                .remaining_percent
                .map(|p| format!("{p:.0}%"))
                .unwrap_or_else(|| match (&s.balance, &s.currency) {
                    (Some(balance), Some(currency)) => format!("{balance} {currency}"),
                    _ => "—".into(),
                });
            if s.source == "demo_synthetic" {
                format!("DEMO {data}")
            } else {
                data
            }
        })
        .unwrap_or_else(|| "—".into());
    let _ = tray.set_tooltip(Some(format!("Kvoteven · {title}")));
    #[cfg(target_os = "macos")]
    let _ = tray.set_title(Some(&title));
    if let Ok(image) = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png")) {
        let mood = snapshot.map(|s| serde_json::to_value(s.mood).unwrap_or(Value::Null));
        let color = match mood.as_ref().and_then(Value::as_str) {
            Some("happy") => [156, 212, 166],
            Some("steady") => [184, 209, 158],
            Some("sleepy") => [227, 204, 143],
            Some("asleep") => [191, 189, 214],
            _ => [194, 199, 196],
        };
        let mut bytes = image.rgba().to_vec();
        for pixel in bytes.chunks_exact_mut(4) {
            if pixel[..3] == [156, 212, 166] {
                pixel[..3].copy_from_slice(&color);
            }
        }
        let _ = tray.set_icon(Some(tauri::image::Image::new_owned(
            bytes,
            image.width(),
            image.height(),
        )));
    }
}
fn accept(app: &tauri::AppHandle, generation: u64, result: &Result<Snapshot, AppError>) {
    let runtime = app.state::<RuntimeState>();
    let mut inner = runtime.inner.lock().unwrap();
    if inner.finish(generation, result.as_ref().ok().cloned()) {
        paint_tray(app, inner.snapshot.as_ref());
    }
}
#[tauri::command]
async fn get_status(state: State<'_, Service>) -> Result<AppStatus, AppError> {
    state.status().await
}
#[tauri::command]
async fn read_quota(
    app: tauri::AppHandle,
    state: State<'_, Service>,
    provider: Provider,
    codex_source: CodexSource,
) -> Result<Snapshot, AppError> {
    let runtime = app.state::<RuntimeState>();
    let generation = {
        let mut inner = runtime.inner.lock().unwrap();
        let generation = inner
            .begin(provider, codex_source, Instant::now())
            .ok_or_else(|| fixed_error("busy"))?;
        paint_tray(&app, None);
        generation
    };
    let result = state.read_quota(provider, codex_source).await;
    if let Ok(snapshot) = &result {
        if runtime.smoke && snapshot.source == "demo_synthetic" {
            match serde_json::to_value(snapshot.provider)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .as_deref()
            {
                Some("deepseek") => {
                    runtime.seen.fetch_or(1, Ordering::SeqCst);
                }
                Some("openai-codex") => {
                    runtime.seen.fetch_or(2, Ordering::SeqCst);
                }
                _ => {}
            }
        }
    }
    accept(&app, generation, &result);
    result
}
#[tauri::command]
async fn save_deepseek_key(
    app: tauri::AppHandle,
    state: State<'_, Service>,
    key: String,
) -> Result<(), AppError> {
    let runtime = app.state::<RuntimeState>();
    let _mutation = runtime.mutation(&app)?;
    state.save_key(key).await
}
#[tauri::command]
async fn delete_deepseek_key(
    app: tauri::AppHandle,
    state: State<'_, Service>,
) -> Result<(), AppError> {
    let runtime = app.state::<RuntimeState>();
    let _mutation = runtime.mutation(&app)?;
    state.delete_key().await
}
#[tauri::command]
async fn set_demo(
    app: tauri::AppHandle,
    state: State<'_, Service>,
    enabled: bool,
) -> Result<AppStatus, AppError> {
    let runtime = app.state::<RuntimeState>();
    let _mutation = runtime.mutation(&app)?;
    state.set_demo(enabled).await
}
#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}
#[derive(Serialize)]
struct SmokeContext {
    enabled: bool,
}
#[tauri::command]
fn smoke_context(runtime: State<'_, RuntimeState>) -> SmokeContext {
    SmokeContext {
        enabled: runtime.smoke,
    }
}
#[derive(Deserialize)]
struct SmokeProof {
    providers: Vec<String>,
    language: String,
    demo: bool,
}
#[tauri::command]
fn report_smoke(
    app: tauri::AppHandle,
    runtime: State<'_, RuntimeState>,
    proof: SmokeProof,
) -> Result<(), AppError> {
    if !smoke::valid_proof(runtime.smoke, proof.demo, &proof.providers, &proof.language)
        || runtime.seen.load(Ordering::SeqCst) != 3
    {
        return Err(fixed_error("invalid_smoke_proof"));
    }
    println!(
        "{}",
        json!({"native_smoke":true,"providers":proof.providers,"languages":["en","da"],"source":"demo_synthetic"})
    );
    app.exit(0);
    Ok(())
}
fn periodic(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let runtime = app.state::<RuntimeState>();
            let selected = {
                let mut inner = runtime.inner.lock().unwrap();
                if inner.snapshot.as_ref().is_some_and(|s| !fresh(s)) {
                    inner.snapshot = None;
                    paint_tray(&app, None);
                }
                inner.due(Instant::now())
            };
            if let Some(s) = selected {
                let result = app
                    .state::<Service>()
                    .read_quota(s.provider, s.source)
                    .await;
                accept(&app, s.generation, &result);
            }
        }
    });
}
pub fn run() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = match cli::parse(&args) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let (demo, smoke) = match mode {
        cli::Mode::Help => {
            println!("Kvoteven 0.4.0\nUsage: kvoteven [--demo [--smoke-test]]\n       kvoteven --check [--provider deepseek|openai-codex] [--source auto|codex|hermes]\n       kvoteven --help | --version\nDemo is offline. Live checks never print credentials or account values.");
            return;
        }
        cli::Mode::Version => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            return;
        }
        cli::Mode::Check { provider, source } => {
            let runtime = tokio::runtime::Runtime::new().expect("Cannot start runtime");
            let result = runtime.block_on(Service::new(false).read_quota(
                serde_json::from_value(json!(provider)).expect("Validated provider"),
                serde_json::from_value(json!(source)).expect("Validated source"),
            ));
            match result {
                Ok(s) if fresh(&s) => println!(
                    "{}",
                    json!({"ok":true,"provider":s.provider,"source":s.source,"fresh":true,"weekly_present":s.remaining_percent.is_some()})
                ),
                Ok(_) => {
                    eprintln!("{{\"ok\":false,\"code\":\"stale_data\"}}");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("{}", json!({"ok":false,"code":e.code}));
                    std::process::exit(1);
                }
            }
            return;
        }
        cli::Mode::App { demo, smoke } => (demo, smoke),
    };
    let mut builder = tauri::Builder::default();
    // An explicit offline demo must never redirect to an already-running live instance.
    if !demo {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _, _| show(app)));
    }
    builder
        .manage(Service::new(demo))
        .manage(RuntimeState {
            smoke,
            seen: AtomicU8::new(0),
            inner: Mutex::new(display_state::DisplayState::default()),
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            read_quota,
            save_deepseek_key,
            delete_deepseek_key,
            set_demo,
            quit_app,
            smoke_context,
            report_smoke
        ])
        .setup(move |app| {
            let open = MenuItem::with_id(app, "show", "Kvoteven", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit / Afslut", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            let tray = TrayIconBuilder::with_id("quota")
                .icon(tauri::image::Image::from_bytes(include_bytes!(
                    "../icons/32x32.png"
                ))?)
                .tooltip("Kvoteven")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        show(tray.app_handle());
                    }
                })
                .build(app);
            if tray.is_ok() && !demo {
                if let Some(window) = app.get_webview_window("main") {
                    let handle = window.clone();
                    window.on_window_event(move |event| {
                        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                            if handle.hide().is_ok() {
                                api.prevent_close();
                            }
                        }
                    });
                }
            }
            paint_tray(app.handle(), None);
            if smoke {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(Duration::from_secs(45)).await;
                    eprintln!("Native WebView smoke timed out");
                    handle.exit(1);
                });
            } else {
                periodic(app.handle().clone());
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Kvoteven could not initialize its desktop runtime");
}
