use eframe::egui::Context;
use eframe::wasm_bindgen::{self, prelude::*};
use logsim::app::App;
use logsim::board::Board;
use logsim::presets::{DevicePreset, Library};
use logsim::settings::Settings;
use rfd::AsyncFileDialog;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Arc;
use std::time::Duration;

#[wasm_bindgen]
pub async fn main_web(canvas_id: &str) {
    unsafe {
        let (sender, receiver) = sync_channel(1000);
        MERGE_PRESETS = Some((Arc::new(sender), receiver));
    }

    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    tracing_wasm::set_as_global_default();

    eframe::start_web(
        canvas_id,
        eframe::WebOptions::default(),
        Box::new(|_| Box::new(WebApp::new())),
    )
    .await
    .expect("failed to start web app");
}

pub fn get_os() -> Option<&'static str> {
    let mut os_name = "Unknown";
    let navigator = web_sys::window()?.navigator();
    let app_version = navigator.app_version().ok()?;

    if app_version.contains("Win") {
        os_name = "windows";
    }
    if app_version.contains("Mac") {
        os_name = "macos";
    }
    if app_version.contains("X11") {
        os_name = "unix";
    }
    if app_version.contains("Linux") {
        os_name = "linux";
    }
    Some(os_name)
}

macro_rules! console_log {
    ($($t:tt)*) => {{
    	let string = format_args!($($t)*).to_string();
    	web_sys::console::log_1(&string.into())
    }};
}

type MergePresets = (Arc<SyncSender<DevicePreset>>, Receiver<DevicePreset>);
static mut MERGE_PRESETS: Option<MergePresets> = None;
fn merge_presets() -> &'static MergePresets {
    unsafe { MERGE_PRESETS.as_ref().unwrap() }
}

struct WebApp {
    app: App,
}
impl WebApp {
    fn new() -> Self {
        let info = logsim::IntegrationInfo {
            name: format!("Web"),
            native: false,
        };
        let settings = Settings::default();
        let library = Library::default();
        let board = Board::default();
        Self {
            app: App::new(info, settings, library, board),
        }
    }
}
impl eframe::App for WebApp {
    fn update(&mut self, ctx: &Context, _win_frame: &mut eframe::Frame) {
        // merge presets if needed
        if let Ok(preset) = merge_presets().1.try_recv() {
            self.app.library.add_preset(preset, true);
        }

        // rest of update
        let event = self.app.update(ctx);

        match event {
            logsim::OutEvent::None => {}
            logsim::OutEvent::Quit => {}
            logsim::OutEvent::ToggleFullscreen => {}

            logsim::OutEvent::ImportPresets => {
                let sender = Arc::clone(&merge_presets().0);
                let future = async move {
                    let entries = AsyncFileDialog::new().pick_files().await;
                    for entry in entries.unwrap_or(Vec::new()) {
                        let bytes = entry.read().await;
                        let Ok(preset) = bincode::deserialize::<DevicePreset>(&bytes) else {
                        console_log!("failed to parse preset {:?}", entry.file_name());
                        continue;
                    };
                        sender.send(preset).unwrap();
                    }
                };
                wasm_bindgen_futures::spawn_local(future);
            }
            logsim::OutEvent::RevealConfigDir => {}

            logsim::OutEvent::LoadBoard => {}
            logsim::OutEvent::LoadLibrary => {}
            logsim::OutEvent::LoadSettings => {}

            logsim::OutEvent::SaveBoard => {}
            logsim::OutEvent::SaveLibrary => {}
            logsim::OutEvent::SaveSettings => {}

            logsim::OutEvent::SaveAll => {}
            _ => {}
        }

        ctx.request_repaint_after(Duration::from_millis(1000 / 60));
    }
}
