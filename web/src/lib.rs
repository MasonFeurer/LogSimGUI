use eframe::egui::{Context, Event, Key, Pos2};
use eframe::wasm_bindgen::{self, prelude::*};
use log_sim_gui::integration::FrameInput;
use log_sim_gui::*;
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

macro_rules! console_log {
    ($($t:tt)*) => {{
    	let string = format_args!($($t)*).to_string();
    	web_sys::console::log_1(&string.into())
    }};
}

fn save(_app: &mut App) {
    console_log!("saving is not implemented for web");
}

type MergePresets = (Arc<SyncSender<DevicePreset>>, Receiver<DevicePreset>);
static mut MERGE_PRESETS: Option<MergePresets> = None;
fn merge_presets() -> &'static MergePresets {
    unsafe { MERGE_PRESETS.as_ref().unwrap() }
}

struct WebApp {
    app: App,
    input: FrameInput,
    hovered: AppItem,
}
impl WebApp {
    fn new() -> Self {
        let create_app = CreateApp {
            settings: Settings::default(),
            presets: Presets::default(),
            scene: Scene::new(),
            keybind_toggle_auto_link: Keybind::Control(Key::L),
            keybind_step_sim: Keybind::Control(Key::S),
            keybind_duplicate_devices: Keybind::Control(Key::D),
        };
        Self {
            app: App::new(create_app),
            input: FrameInput::default(),
            hovered: AppItem::default(),
        }
    }
}
impl eframe::App for WebApp {
    fn update(&mut self, ctx: &Context, _win_frame: &mut eframe::Frame) {
        // merge presets if needed
        if let Ok(preset) = merge_presets().1.try_recv() {
            self.app.presets.merge(&[preset]);
        }

        for event in &ctx.input().events {
            let Event::Key { key, pressed: true, .. } = event else {
        		continue;
        	};
        }

        // rest of update
        self.input.update(ctx, self.hovered);
        let output = self.app.update(ctx, &self.input);

        self.hovered = output.hovered;
        self.input.hovered_changed = self.input.prev_hovered != output.hovered;
        self.input.prev_hovered = output.hovered;
        if output.void_click {
            self.input.press_pos = Pos2::ZERO;
        }

        if output.save {
            save(&mut self.app);
        }
        if output.reveal_persist_dir {
            console_log!("not available in web");
        }
        if output.import_presets {
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
        if output.load_presets {
            console_log!("not available in web");
        }

        ctx.request_repaint_after(Duration::from_millis(1000 / 60));
    }

    fn on_exit(&mut self, _ctx: Option<&eframe::glow::Context>) {
        self.app.on_exit();
    }
}
