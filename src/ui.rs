use crate::app::{App, AppAction, AppItem};
use crate::board::{Board, BoardItem, DeviceData};
use crate::graphics::{Transform, View};
use crate::input::Input;
use crate::presets::{Library, PresetData, PresetSource};
use egui::*;

#[derive(Default, Clone)]
pub struct LibraryMenu {
    pub open: bool,
    pub sel: Option<String>,
}

#[derive(Clone)]
pub struct PackMenu {
    pub open: bool,
    pub name: String,
    pub color: Color32,
    pub cat: String,
    pub combinational: bool,
    pub err: Option<String>,
}
impl Default for PackMenu {
    fn default() -> Self {
        Self {
            open: false,
            name: format!("New Chip"),
            color: Color32::WHITE,
            cat: format!("Basic"),
            combinational: false,
            err: None,
        }
    }
}

#[derive(Clone)]
pub struct SimMenu {
    pub open: bool,
    pub speed: u32,
    pub paused: bool,
    pub view: View,
}
impl Default for SimMenu {
    fn default() -> Self {
        Self {
            open: false,
            view: View::default(),
            paused: false,
            speed: 1,
        }
    }
}

pub fn show_library_menu(
    ui: &mut Ui,
    debug: bool,
    menu: &mut LibraryMenu,
    native: bool,
    library: &Library,
) -> AppAction {
    let mut action = AppAction::None;

    ui.horizontal(|ui| {
        ui.heading("Library");
        if native && ui.button("reload").clicked() {
            action = AppAction::ReloadLibrary;
        }
        if ui.button("import").clicked() {
            action = AppAction::ImportLibrary;
        }
    });
    ui.separator();

    let sel_preset = menu.sel.clone().and_then(|name| {
        let preset = library.get_preset(&name);
        if preset.is_none() {
            menu.sel = None;
        }
        preset.map(|preset| (name, preset))
    });
    if let Some((name, preset)) = sel_preset {
        ui.heading(&name);

        let mut stat = |s: &str| {
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.label(s);
            });
        };
        match &preset.data {
            PresetData::Chip(chip) => {
                stat(&format!("inputs: {}", chip.inputs.len()));
                stat(&format!("outputs: {}", chip.outputs.len()));
            }
            PresetData::CombGate(comb_gate) => {
                stat(&format!(
                    "combinational ({} combinations)",
                    comb_gate.table.map.len()
                ));
                stat(&format!("inputs: {}", comb_gate.inputs.len()));
                stat(&format!("outputs: {}", comb_gate.outputs.len()));
            }
            _ => {}
        }
        let (stat_str, can_del, can_load) = match &preset.src {
            PresetSource::Default => ("source: default", false, false),
            PresetSource::Builtin => ("source: builtin", false, false),
            PresetSource::Board(_) => ("source: user created", true, true),
        };
        stat(stat_str);

        let [mut load, mut delete, mut place] = [false; 3];
        ui.horizontal(|ui| {
            if debug && ui.button("debug").clicked() {
                println!("{:#?}", preset);
            }
            delete = ui.add_enabled(can_del, Button::new("delete")).clicked();
            load = ui.add_enabled(can_load, Button::new("load")).clicked();
            place = ui.button("place").clicked();
        });
        ui.separator();
        match (load, delete, place) {
            (true, _, _) => action = AppAction::LoadPreset(name),
            (_, true, _) => action = AppAction::DeletePreset(name),
            (_, _, true) => action = AppAction::HoldPreset(name),
            _ => {}
        }
    }

    let mut sel_preset: Option<String> = None;
    for (cat_name, presets) in library.cats_sorted() {
        ui.collapsing(cat_name, |ui| {
            for preset in presets {
                let rs = ui.button(&preset.name);
                if rs.clicked() {
                    sel_preset = Some(preset.name.clone());
                }
                if menu.sel.as_ref() == Some(&preset.name) {
                    ui.painter().add(Shape::rect_stroke(
                        rs.rect,
                        Rounding::none(),
                        Stroke::new(1.0, Color32::from_gray(200)),
                    ));
                }
            }
        });
    }
    if let Some(preset) = sel_preset {
        menu.sel = Some(preset);
    }
    action
}

pub fn show_pack_menu(ui: &mut Ui, menu: &mut PackMenu, library: &Library) -> AppAction {
    let mut action = AppAction::default();
    ui.heading("Pack chip");
    ui.separator();

    ui.label("Name");
    ui.text_edit_singleline(&mut menu.name);

    ui.label("Category");
    ui.menu_button(menu.cat.clone(), |ui| {
        show_cat_menu(ui, &mut menu.cat, library);
    });

    ui.label("Color");
    ui.color_edit_button_srgba(&mut menu.color);

    ui.add_space(50.0);
    if ui.button("Done").clicked() {
        action = AppAction::PackBoard;
    }
    action
}
pub fn show_sim_menu(ui: &mut Ui, menu: &mut SimMenu) -> AppAction {
    let mut action = AppAction::default();
    ui.heading("Sim");
    ui.separator();

    let pause_label = match menu.paused {
        true => "Unpause",
        false => "Pause",
    };
    if ui.button(pause_label).clicked() {
        menu.paused = !menu.paused;
    }

    if ui.add_enabled(menu.paused, Button::new("Step")).clicked() {
        action = AppAction::StepSim;
    }
    ui.group(|ui| {
        ui.label("speed");

        ui.horizontal(|ui| {
            if ui.button("+").clicked() {
                menu.speed <<= 1;
            }
            if ui.add_enabled(menu.speed > 1, Button::new("-")).clicked() {
                menu.speed >>= 1;
            }
            ui.label(format!("{}", menu.speed));
        });
    });
    action
}

pub fn show_cat_menu(ui: &mut Ui, cat: &mut String, library: &Library) {
    const LEFT_SP: f32 = 15.0;

    ui.horizontal(|ui| {
        ui.add_space(LEFT_SP);
        ui.add(TextEdit::singleline(cat));
    });

    ui.separator();
    ui.label("Existing categories");
    let mut choose_cat: Option<String> = None;
    for (cat_name, _) in library.cats_sorted() {
        ui.horizontal(|ui| {
            ui.add_space(LEFT_SP);
            let cat_button = ui.button(cat_name);

            if cat_button.clicked() {
                choose_cat = Some(String::from(cat_name));
                ui.close_menu();
            }
        });
    }
    if let Some(name) = choose_cat {
        *cat = name;
    }
}

pub fn show_top_panel(ui: &mut Ui) -> AppAction {
    let mut action = AppAction::None;
    if ui.button("Settings").clicked() {
        action = AppAction::OpenSettings;
    }
    if ui.button("Library").clicked() {
        action = AppAction::ToggleLibraryMenu;
    }
    if ui.button("Pack").clicked() {
        action = AppAction::TogglePackMenu;
    }
    if ui.button("Sim").clicked() {
        action = AppAction::ToggleSimMenu;
    }
    action
}

#[derive(Clone)]
pub struct ChipPlacer {
    // A search query into self.library
    pub field: String,
    // The search results from field
    pub results: Vec<String>,
    // If we are searching a category name (with ":cat")
    pub results_cat: Option<String>,
    pub recent: Vec<String>,
    pub first_frame: bool,
}
impl ChipPlacer {
    pub fn default() -> Self {
        Self {
            field: String::new(),
            results: Vec::new(),
            results_cat: None,
            recent: Vec::new(),
            first_frame: true,
        }
    }

    pub fn push_recent(&mut self, preset: &str) {
        if let Some(idx) = self.recent.iter().position(|e| e.as_str() == preset) {
            self.recent.remove(idx);
        }
        self.recent.insert(0, String::from(preset));
        if self.recent.len() > 10 {
            self.recent.pop();
        }
    }

    pub fn check_recent(&mut self, library: &Library) {
        for idx in (0..self.recent.len()).rev() {
            if library.get_preset(&self.recent[idx]).is_none() {
                self.recent.remove(idx);
            }
        }
    }

    pub fn show(
        &mut self,
        pos: Pos2,
        ui: &mut Ui,
        input: &Input,
        library: &Library,
        request_focus: bool,
    ) -> (bool, AppAction) {
        let mut action = AppAction::default();

        let size = vec2(200.0, 20.0);
        let rect = Rect::from_min_size(pos, size);

        let mut field_changed = self.first_frame;
        self.first_frame = true;
        let mut entered = false;
        let mut field_rs = None;

        let mut ui = ui.child_ui(rect, ui.layout().clone());
        let frame_rs = Frame::menu(ui.style()).show(&mut ui, |ui| {
            ui.horizontal(|ui| {
                ui.style_mut().spacing.text_edit_width = 100.0;
                ui.style_mut().spacing.item_spacing = vec2(5.0, 0.0);
                ui.style_mut().spacing.button_padding = Vec2::ZERO;

                let rs = ui.add(TextEdit::singleline(&mut self.field).hint_text("Search library"));
                if request_focus {
                    rs.request_focus();
                    self.field = String::new();
                }
                entered = rs.lost_focus() && input.pressed(Key::Enter);
                field_changed = field_changed | rs.changed();

                for result in &self.results {
                    if ui.button(result).clicked() {
                        action = AppAction::HoldPreset(result.clone());
                    }
                }
                field_rs = Some(rs);
            })
        });
        let field_rs = field_rs.unwrap();

        let hovered = frame_rs.response.rect.contains(input.pointer_pos);
        if entered && self.results.len() >= 1 {
            let preset = self.results[0].clone();
            action = AppAction::HoldPreset(preset);
            field_rs.request_focus();
        }
        if field_changed {
            (self.results, self.results_cat) = match &self.field {
                // If the search field starts with ':', show results of the cat name given
                s if s.starts_with(':') => match library.search_cats(&s[1..]) {
                    Some(cat) => (library.cat_presets(&cat), Some(cat)),
                    None => (vec![], None),
                },
                // If the search field is empty, show all presets, showing recent presets first
                s if s.trim().is_empty() => {
                    let mut results = library.preset_names();
                    results.sort_by(|a, b| self.recent.contains(a).cmp(&self.recent.contains(b)));
                    (results, None)
                }
                s => (library.search_presets(s), None),
            };
        }
        (hovered, action)
    }
}

#[derive(Default)]
pub struct NamePopupRs {
    pub hovered: bool,
    pub edit: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum PinType {
    Input,
    Output,
}

const FADE_TIME: u32 = 50;

#[derive(Clone, Debug)]
pub struct NamePopup {
    pub timer: u32,
    pub id: u64,
    pub ty: PinType,
}
impl NamePopup {
    pub fn input(id: u64) -> Self {
        Self {
            timer: FADE_TIME,
            id,
            ty: PinType::Input,
        }
    }
    pub fn output(id: u64) -> Self {
        Self {
            timer: FADE_TIME,
            id,
            ty: PinType::Output,
        }
    }

    pub fn is_dead(&self) -> bool {
        self.timer == 0
    }
    pub fn update(&mut self) {
        if self.timer > 0 {
            self.timer -= 1;
        }
    }
    pub fn persist(&mut self) {
        self.timer = FADE_TIME;
    }

    pub fn show(self, ui: &mut Ui, board: &Board, col_w: f32, t: Transform) -> NamePopupRs {
        let mut out = NamePopupRs::default();

        let size = vec2(100.0, 50.0);
        let (rect, mut name) = match self.ty {
            PinType::Input => {
                let input = &board.inputs.get(&self.id).unwrap().io;
                let pos = pos2(board.rect.left() + col_w, input.y_pos - t * (size.y * 0.5));
                (Rect::from_min_size(t * pos, size), input.name.clone())
            }
            PinType::Output => {
                let output = &board.outputs.get(&self.id).unwrap().io;
                let pos = pos2(
                    board.rect.right() - col_w - size.x,
                    output.y_pos - size.y * 0.5,
                );
                (Rect::from_min_size(t * pos, size), output.name.clone())
            }
        };
        if name.trim().is_empty() {
            name = format!("no-name");
        }

        let factor = self.timer as f32 / FADE_TIME as f32;
        let fade = |color: &mut Color32| {
            *color = color.linear_multiply(factor);
        };

        let mut ui = ui.child_ui(rect, ui.layout().clone());

        let frame = Frame::popup(ui.style()).multiply_with_opacity(factor);
        let rs = frame.show(&mut ui, |ui| {
            let vis = &mut ui.style_mut().visuals.widgets;
            fade(&mut vis.noninteractive.fg_stroke.color);
            ui.label(name);
        });
        let rs = rs.response.interact(Sense::click());

        if rs.hovered() {
            out.hovered = true;
        }
        if rs.clicked() {
            out.edit = true;
        }
        out
    }

    pub fn show_editor(self, _ui: &mut Ui) {}
}

pub fn debug_ui(ui: &mut Ui, app: &mut App) {
    ui.style_mut().wrap = Some(false);
    ui.separator();

    ui.label(format!("hovered: {:?}", app.input.hovered()));
    if let AppItem::Board(BoardItem::Device(id)) = app.input.hovered() {
        let Some(device) = app.board.devices.get(&id) else {
            return
        };
        match &device.data {
            DeviceData::Chip(chip) => {
                ui.label("data: Chip");
                ui.label(format!("writes: {}", chip.write_queue.len()));
                ui.label(format!("devices: {}", chip.devices.len()));
            }
            DeviceData::CombGate(_) => {
                ui.label("data: CombGate");
            }
        }
        ui.label(format!("preset: {}", device.preset));
        ui.add_space(10.0);
    }

    ui.label(format!("drag: {:?}", app.input.drag));
    ui.label(format!("selected devices: {:?}", app.selected_devices));
    ui.label(format!("name popup: {:?}", app.name_popup));

    ui.add_space(10.0);

    ui.label(format!("write queue: ({})", app.board.write_queue.len()));
    for write in &app.board.write_queue.writes {
        ui.horizontal(|ui| {
            ui.add_space(15.0);
            ui.label(format!("{:?}", write));
        });
    }
}
