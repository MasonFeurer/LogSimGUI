use crate::app::{App, AppItem};
use crate::integration::{FrameInput, FrameOutput};
use egui::*;

const ENABLE_SEQ: &[Key] = &[Key::I, Key::M, Key::A, Key::D, Key::E, Key::V];

pub struct DevOptions {
    pub enabled: bool,
    pub enable_step: usize,
    pub pos: Pos2,
    device_ids: bool,
}
impl Default for DevOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            enable_step: 0,
            pos: Pos2::new(200.0, 200.0),
            device_ids: false,
        }
    }
}
impl DevOptions {
    pub fn show_device_ids(&self) -> bool {
        self.enabled && self.device_ids
    }

    pub fn ui(ui: &mut Ui, input: &FrameInput, app: &mut App) {
        ui.style_mut().wrap = Some(false);
        ui.separator();
        ui.checkbox(&mut app.dev_options.device_ids, "device IDs");

        ui.label(format!("hovered: {:?}", input.prev_hovered));
        ui.label(format!("drag: {:?}", input.drag));
        ui.label(format!("selected devices: {:?}", app.selected_devices));
        ui.label(format!("link starts: {:?}", app.link_starts));
        ui.label(format!("edit popup: {:?}", app.edit_popup));

        ui.label(format!("write queue: ({})", app.scene.write_queue.len()));
        for write in &app.scene.write_queue.writes {
            ui.horizontal(|ui| {
                ui.add_space(15.0);
                ui.label(format!("{:?}", write));
            });
        }
    }

    pub fn show(ui: &mut Ui, input: &FrameInput, output: &mut FrameOutput, app: &mut App) {
        let rect = Rect::from_min_size(app.dev_options.pos, Vec2::new(300.0, 100.0));
        let mut ui = ui.child_ui(rect, ui.layout().clone());
        let rs = Frame::menu(ui.style()).show(&mut ui, |ui| {
            Self::ui(ui, input, app);
        });
        if rs.response.rect.contains(input.pointer_pos) {
            output.hovered.cond_replace(AppItem::DevOptions);
        }
    }

    pub fn input(&mut self, input: &FrameInput) {
        // Handle secret activation input
        if self.enabled {
            return;
        }
        if input.pressed(ENABLE_SEQ[self.enable_step]) {
            self.enable_step += 1;
            if self.enable_step == ENABLE_SEQ.len() {
                self.enabled = true;
            }
        }
    }
}
