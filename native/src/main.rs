#![windows_subsystem = "windows"]

mod persist;

use crate::persist::ErrResult;
use eframe::egui::{Context, Key, Pos2};
use eframe::{run_native, NativeOptions};
use log_sim_gui::*;
use std::time::{Duration, SystemTime};

fn save(app: &mut App) {
    persist::save_scene(&app.scene).log_err();
    persist::save_settings(&app.settings).log_err();
    persist::save_presets(&mut app.presets).log_err();
}

struct NativeApp {
    app: App,
    /// The time of the most recent auto-save
    last_save: SystemTime,
    input: FrameInput,
    hovered: AppItem,
}
impl NativeApp {
    fn new() -> Self {
        let create_app = CreateApp {
            settings: persist::load_settings().unwrap_res_or(|| Settings::default()),
            presets: persist::load_presets().unwrap_res_or(|| Presets::default()),
            scene: persist::load_scene().unwrap_res_or(|| Scene::new()),
            keybind_toggle_auto_link: Keybind::Command(Key::L),
            keybind_step_sim: Keybind::Command(Key::S),
            keybind_duplicate_devices: Keybind::Command(Key::D),
        };
        Self {
            app: App::new(create_app),
            last_save: SystemTime::now(),
            input: FrameInput::default(),
            hovered: AppItem::default(),
        }
    }
}
impl eframe::App for NativeApp {
    fn update(&mut self, ctx: &Context, _win_frame: &mut eframe::Frame) {
        self.input.update(ctx, self.hovered);

        let output = self.app.update(ctx, &self.input);
        self.hovered = output.hovered;

        self.input.hovered_changed = self.input.prev_hovered != output.hovered;
        self.input.prev_hovered = output.hovered;
        if output.void_click {
            self.input.press_pos = Pos2::ZERO;
        }

        let since_last_save = SystemTime::now().duration_since(self.last_save).unwrap();
        if since_last_save.as_secs() > 30 || output.save {
            save(&mut self.app);
            self.last_save = SystemTime::now();
        }

        if output.reveal_persist_dir {
            persist::reveal_dir(&persist::persist_dir()).log_err();
        }
        if output.import_presets {
            for path in rfd::FileDialog::new().pick_files().unwrap_or(Vec::new()) {
                match persist::load::<_, DevicePreset>(&path, persist::Encoding::Data) {
                    Ok(preset) => self.app.presets.add_presets(&[preset]),
                    Err(err) => err.context("Failed to load preset").log(),
                }
            }
        }
        if output.load_presets {
            match persist::load_presets() {
                Ok(presets) => self.app.presets = presets,
                Err(err) => err.log(),
            }
        }

        ctx.request_repaint_after(Duration::from_millis(1000 / 60));
    }

    fn on_exit(&mut self, _ctx: Option<&eframe::glow::Context>) {
        save(&mut self.app);
        self.app.on_exit();
    }
}
fn main() {
    run_native(
        "LogSimGUI",
        NativeOptions::default(),
        Box::new(|_cc| Box::new(NativeApp::new())),
    );
}
