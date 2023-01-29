#![cfg_attr(debug, windows_subsystem = "windows")]

mod files;
use files::FileErrResult;

use eframe::egui::Context;
use eframe::{run_native, NativeOptions};
use futures::executor::ThreadPool;
use logsim::{app::App, presets::DevicePreset, IntegrationInfo, OutEvent};
use rfd::AsyncFileDialog;
use std::env::consts::{ARCH, OS};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

fn save_all(app: &mut App) {
    files::save_settings(&app.settings).log_err();
    files::save_board(&app.board).log_err();
    files::save_library(&mut app.library).log_err();
}

struct NativeApp {
    app: App,
    last_save: SystemTime,
    fullscreen: bool,

    recv_imported_presets: Receiver<DevicePreset>,
    send_imported_presets: Arc<SyncSender<DevicePreset>>,
    thread_pool: ThreadPool,
}
impl NativeApp {
    fn new() -> Self {
        let info = IntegrationInfo {
            name: format!("Native {OS} {ARCH}"),
            native: true,
        };

        let library = files::load_library().log_err().unwrap_or_default();
        let settings = files::load_settings().log_err().unwrap_or_default();
        let board = files::load_board().log_err().unwrap_or_default();

        let (send, recv) = sync_channel(100);
        Self {
            app: App::new(info, settings, library, board),
            last_save: SystemTime::now(),
            fullscreen: false,

            recv_imported_presets: recv,
            send_imported_presets: Arc::new(send),
            // TODO gracefully handle err (creating a thread pool is only required for importing presets)
            thread_pool: ThreadPool::new().expect("Failed to create thread pool"),
        }
    }
}
impl NativeApp {
    fn import_presets(&mut self) {
        let sender = Arc::clone(&self.send_imported_presets);
        let future = async move {
            let entries = AsyncFileDialog::new().pick_files().await;
            for entry in entries.unwrap_or(Vec::new()) {
                let bytes: Vec<_> = entry.read().await;
                let Ok(preset) = bincode::deserialize::<DevicePreset>(&bytes) else {
                    println!("failed to parse preset {:?}", entry.file_name());
                    continue;
                };
                sender.send(preset).unwrap();
            }
        };
        self.thread_pool.spawn_ok(future);
    }
}
impl eframe::App for NativeApp {
    fn update(&mut self, ctx: &Context, window: &mut eframe::Frame) {
        // Merge preset if we have imported some
        if let Ok(preset) = self.recv_imported_presets.try_recv() {
            self.app.library.add_preset(preset, true);
        }

        let event = self.app.update(ctx);
        match event {
            OutEvent::None => {}
            OutEvent::Quit => window.close(),
            OutEvent::ToggleFullscreen => {
                window.set_fullscreen(!self.fullscreen);
                self.fullscreen = !self.fullscreen;
            }
            OutEvent::ImportPresets => self.import_presets(),
            OutEvent::RevealConfigDir => {
                files::reveal_config_dir().log_err();
            }

            OutEvent::SaveAll => save_all(&mut self.app),
            OutEvent::SaveSettings => files::save_settings(&self.app.settings).log_err().unwrap(),
            OutEvent::LoadSettings => self.app.settings = files::load_settings().log_err().unwrap(),
            OutEvent::SaveBoard => files::save_board(&self.app.board).log_err().unwrap(),
            OutEvent::LoadBoard => self.app.board = files::load_board().log_err().unwrap(),
            OutEvent::SaveLibrary => files::save_library(&mut self.app.library)
                .log_err()
                .unwrap(),
            OutEvent::LoadLibrary => self.app.library = files::load_library().log_err().unwrap(),
            _ => {}
        }

        // auto save
        let since_last_save = SystemTime::now().duration_since(self.last_save).unwrap();
        if since_last_save.as_secs() > 30 {
            save_all(&mut self.app);
            self.last_save = SystemTime::now();
        }

        // repaint
        ctx.request_repaint_after(Duration::from_millis(1000 / 60));
    }

    fn on_exit(&mut self, _ctx: Option<&eframe::glow::Context>) {
        save_all(&mut self.app);
    }
}
fn main() {
    run_native(
        "LogSim Native",
        NativeOptions::default(),
        Box::new(|_cc| Box::new(NativeApp::new())),
    );
}
