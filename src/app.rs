use crate::dev::DevOptions;
use crate::graphics::{Graphics, View};
use crate::integration::{FrameInput, FrameOutput, Keybind};
use crate::preset::*;
use crate::preset_placer::PresetPlacer;
use crate::scene::{Device, IoSel, Scene, SceneItem};
use crate::settings::Settings;
use crate::*;
use egui::*;

const COMBINATIONAL_MSG: &str = "will use a truth table for the created preset";
const SAVE_SCENE_MSG: &str =
    "store state of scene along side the created preset,\nallowing the preset to be later modified";
const AUTO_LINK_MSG: &str = "automatically start/finish placing a link when you hover a pin";
const CREATE_MSG: &str = "pack the scene into a preset for later use";

#[derive(Clone)]
struct CreatePreset {
    name: String,
    color: Color32,
    cat: String,
    combinational: bool,
    save_scene: bool,
}
impl CreatePreset {
    pub fn default() -> Self {
        Self {
            name: String::from("New Chip"),
            color: Color32::from_rgb(255, 255, 255),
            cat: String::from("Basic"),
            combinational: false,
            save_scene: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppItem {
    // off screen, or undetermined
    None,
    SceneBackground,
    SceneItem(SceneItem),
    EditPopup,
    Other,
    PresetPlacer,
    DevOptions,
}
impl Default for AppItem {
    fn default() -> Self {
        Self::None
    }
}
impl AppItem {
    /// If a.layer() > b.layer(), then a is shown above b
    pub fn layer(self) -> u8 {
        match self {
            Self::None => 0,
            Self::SceneBackground => 1,
            Self::SceneItem(_) => 2,
            Self::EditPopup => 3,
            Self::Other => 4,
            Self::PresetPlacer | Self::DevOptions => 5,
        }
    }

    /// Replaces self with other if other is shown above self
    pub fn cond_replace(&mut self, other: AppItem) {
        if other.layer() > self.layer() {
            *self = other;
        }
    }
}

pub struct CreateApp {
    pub settings: Settings,
    pub presets: Presets,
    pub scene: Scene,
    pub keybind_toggle_auto_link: Keybind,
    pub keybind_duplicate_devices: Keybind,
    pub keybind_step_sim: Keybind,
}

pub struct App {
    pub settings: Settings,
    pub presets: Presets,
    pub scene: Scene,
    pub dev_options: DevOptions,

    view: View,
    paused: bool,
    speed: u32,

    /// If there is a focused text field this frame
    focused_text_field: bool,
    /// Where the pointer was when the context menu was opened
    context_menu_pos: Pos2,
    /// What preset we've selected in the presets menu
    presets_menu_sel: Option<String>,
    /// If we right click on some scene item, shows a popup
    pub edit_popup: Option<SceneItem>,
    /// If we started placing some links
    pub link_starts: Vec<LinkStart<u64>>,
    /// The config for creating a new preset from the scene
    create_preset: CreatePreset,
    /// If there was an error creating a preset
    create_err: Option<String>,
    /// The small window for searching and placing presets
    preset_placer: PresetPlacer,
    /// A list of the presets we've picked from the preset placer
    pub held_presets: Vec<String>,
    /// If we've selected multiple devices for bulk actions
    pub selected_devices: Vec<u64>,
    /// If true, we should automatically start/finish placing a link when we hover the pin
    auto_link: bool,

    // --- keybinds ---
    keybind_toggle_auto_link: Keybind,
    keybind_duplicate_devices: Keybind,
    keybind_step_sim: Keybind,
}
impl App {
    pub fn new(create: CreateApp) -> Self {
        Self {
            settings: create.settings,
            presets: create.presets,
            scene: create.scene,
            dev_options: DevOptions::default(),

            view: View::default(),
            paused: false,
            speed: 1,

            focused_text_field: false,
            context_menu_pos: Pos2::ZERO,
            presets_menu_sel: None,
            edit_popup: None,
            link_starts: Vec::new(),
            create_preset: CreatePreset::default(),
            create_err: None,
            preset_placer: PresetPlacer::default(),
            held_presets: Vec::new(),
            selected_devices: Vec::new(),
            auto_link: false,

            keybind_toggle_auto_link: create.keybind_toggle_auto_link,
            keybind_duplicate_devices: create.keybind_duplicate_devices,
            keybind_step_sim: create.keybind_step_sim,
        }
    }

    pub fn create(&mut self) {
        self.create_err = None;
        let data = if self.create_preset.combinational {
            match CombGatePreset::from_scene(&mut self.scene) {
                Ok(v) => PresetData::CombGate(v),
                Err(e) => {
                    self.create_err = Some(format!("Can't create combination gate: {}", e));
                    return;
                }
            }
        } else {
            PresetData::Chip(ChipPreset::from_scene(&self.scene))
        };

        let save_scene = self.create_preset.save_scene;
        self.presets.add_preset(DevicePreset {
            data,
            name: self.create_preset.name.clone(),
            color: self.create_preset.color.to_array(),
            src: PresetSource::Scene(save_scene.then(|| self.scene.clone())),
            cat: self.create_preset.cat.clone(),
        });
        self.scene = Scene::new();

        let cat = self.create_preset.cat.clone();
        self.create_preset = CreatePreset::default();
        self.create_preset.cat = cat;
    }
    pub fn place_preset(&mut self, name: &str, pos: Pos2) {
        if let Some(preset) = self.presets.get_preset(name) {
            let device = Device::from_preset(preset, pos);
            self.scene.add_device(rand_id(), device);
            self.preset_placer.push_recent(name);
        }
    }
    pub fn load_preset(&mut self, name: &str) {
        let preset = self.presets.get_preset(name).unwrap().clone();
        let PresetSource::Scene(Some(scene)) = preset.src.clone() else {
        	return;
        };
        if matches!(preset.data, PresetData::CombGate(_)) {
            self.create_preset.combinational = true;
        }
        let [r, g, b, a] = preset.color;
        self.create_preset.name = preset.name;
        self.create_preset.color = Color32::from_rgba_premultiplied(r, g, b, a);
        self.create_preset.cat = preset.cat;
        self.create_preset.save_scene = true;
        self.scene = scene;
    }
    pub fn start_link(&mut self, start: LinkStart<u64>) {
        for start2 in &self.link_starts {
            if *start2 == start {
                return;
            }
        }
        self.link_starts.insert(0, start);
    }
    pub fn finish_link(&mut self, target: LinkTarget<u64>) -> bool {
        let Some(start) = self.link_starts.last().cloned() else {
        	return false;
        };
        let new_link = match start {
            LinkStart::DeviceOutput(device, output) => {
                NewLink::DeviceOutputTo(device, output, target)
            }
            LinkStart::Input(id) => {
                let device_input = match target {
                    LinkTarget::DeviceInput(device, input) => DeviceInput(device, input),
                    LinkTarget::Output(_) => return false,
                };
                NewLink::InputToDeviceInput(id, device_input)
            }
        };
        self.scene.remove_link_to(target);
        self.scene.add_link(new_link);
        self.link_starts.pop().unwrap();
        true
    }

    // -----------------------------------------------------------
    // GUI

    pub fn settings_menu(&mut self, ui: &mut Ui, output: &mut FrameOutput) {
        const SPACE: f32 = 10.0;
        fn slider(ui: &mut Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>) {
            ui.horizontal(|ui| {
                ui.add_space(SPACE);
                ui.label(label);
                ui.add(Slider::new(v, range));
            });
        }
        fn checkbox(ui: &mut Ui, label: &str, v: &mut bool) {
            ui.horizontal(|ui| {
                ui.add_space(SPACE);
                ui.label(label);
                ui.checkbox(v, "");
            });
        }

        ui.horizontal(|ui| {
            ui.heading("Settings");
            output.load_settings = ui.button("reload").clicked();
            if ui.button("reset").clicked() {
                self.settings = Settings::default();
            }
        });
        ui.separator();

        let s = &mut self.settings;

        ui.heading("App");
        ui.horizontal(|ui| {
            ui.add_space(SPACE);
            ui.label(format!(
                "version: {}.{}.{}",
                s.version[0], s.version[1], s.version[2],
            ))
        });
        checkbox(ui, "dark mode: ", &mut s.dark_mode);
        checkbox(ui, "high contrast: ", &mut s.high_contrast);
        ui.horizontal(|ui| {
            ui.add_space(SPACE);
            output.reveal_persist_dir = ui.button("open config folder").clicked();
        });

        ui.heading("Scene IO");
        slider(ui, "column width: ", &mut s.scene_pin_col_w, 1.0..=100.0);
        slider(ui, "pin width", &mut s.scene_pin_size[0], 1.0..=100.0);
        slider(ui, "pin height", &mut s.scene_pin_size[1], 1.0..=100.0);
        slider(ui, "link width", &mut s.link_width, 1.0..=20.0);

        ui.heading("Devices");
        slider(ui, "name size: ", &mut s.device_name_font_size, 1.0..=100.0);
        slider(ui, "pin width", &mut s.device_pin_size[0], 1.0..=100.0);
        slider(ui, "pin height", &mut s.device_pin_size[1], 1.0..=100.0);
        slider(
            ui,
            "min pin spacing: ",
            &mut s.device_min_pin_spacing,
            1.0..=100.0,
        );
    }

    pub fn presets_menu(&mut self, ui: &mut Ui, output: &mut FrameOutput) {
        ui.horizontal(|ui| {
            ui.heading("Presets");
            output.load_presets = ui.button("reload").clicked();
            output.import_presets = ui.button("import").clicked();
        });
        ui.separator();

        'sel: {
            let Some(preset) = &self.presets_menu_sel else {
        		break 'sel;
        	};
            let Some(preset) = self.presets.get_preset(preset) else {
        		self.presets_menu_sel = None;
        		break 'sel;
        	};
            let name = preset.name.clone();
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
            }
            let (stat_str, can_del, can_load) = match &preset.src {
                PresetSource::Default => ("source: builtin", false, false),
                PresetSource::Scene(Some(_)) => ("source: user created (with scene)", true, true),
                PresetSource::Scene(None) => ("source: user created (no scene)", true, false),
            };
            stat(stat_str);

            let [mut load, mut delete, mut place] = [false; 3];
            ui.horizontal(|ui| {
                if self.dev_options.enabled {
                    if ui.button("debug").clicked() {
                        println!("{:#?}", preset);
                    }
                }
                delete = ui.add_enabled(can_del, Button::new("delete")).clicked();
                load = ui.add_enabled(can_load, Button::new("load")).clicked();
                place = ui.button("place").clicked();
            });
            ui.separator();
            match (load, delete, place) {
                (true, _, _) => self.load_preset(&name),
                (_, true, _) => self.presets.remove_preset(&name),
                (_, _, true) => self.held_presets.push(name),
                _ => {}
            }
        }

        let mut sel_preset: Option<String> = None;
        for (cat_name, presets) in self.presets.cats_sorted() {
            ui.collapsing(cat_name, |ui| {
                for preset in presets {
                    let rs = ui.button(&preset.name);
                    if rs.clicked() {
                        sel_preset = Some(preset.name.clone());
                    }
                    if self.presets_menu_sel.as_ref() == Some(&preset.name) {
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
            self.presets_menu_sel = Some(preset);
        }
    }

    pub fn cat_menu(&mut self, ui: &mut Ui) {
        ui.add_space(10.0);
        const LEFT_SP: f32 = 15.0;
        let mut choose_cat: Option<String> = None;

        for (cat_name, _) in self.presets.cats_sorted() {
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
            self.create_preset.cat = name;
        }

        ui.separator();
        ui.horizontal(|ui| {
            ui.add_space(LEFT_SP);
            let rs = ui.text_edit_singleline(&mut self.create_preset.cat);
            if rs.has_focus() {
                self.focused_text_field = true;
            }
            ui.add_space(5.0);
        });
        ui.add_space(5.0);
    }

    pub fn top_panel(&mut self, ui: &mut Ui, output: &mut FrameOutput) {
        ui.group(|ui| {
            ui.menu_button("Settings", |ui| {
                output.hovered.cond_replace(AppItem::Other);
                *ui.visuals_mut() = self.settings.visuals();
                self.settings_menu(ui, output);
            });
            ui.menu_button("Presets", |ui| {
                output.hovered.cond_replace(AppItem::Other);
                *ui.visuals_mut() = self.settings.visuals();
                self.presets_menu(ui, output);
            });

            let mut rs = ui.button("Create");
            rs = match &self.create_err {
                Some(msg) => rs.on_hover_text(RichText::new(msg).color(Color32::RED)),
                None => rs.on_hover_text(CREATE_MSG),
            };
            if rs.clicked() {
                self.create();
            }
        });

        let rs = ui.color_edit_button_srgba(&mut self.create_preset.color);
        if rs.changed() {
            output.hovered.cond_replace(AppItem::Other);
        }

        ui.menu_button(self.create_preset.cat.clone(), |ui| {
            output.hovered.cond_replace(AppItem::Other);
            *ui.visuals_mut() = self.settings.visuals();
            self.cat_menu(ui);
        });

        ui.style_mut().spacing.text_edit_width = 100.0;
        let rs = ui.text_edit_singleline(&mut self.create_preset.name);
        if rs.has_focus() {
            self.focused_text_field = true;
        }

        ui.checkbox(&mut self.create_preset.combinational, "Combinational")
            .on_hover_text(COMBINATIONAL_MSG);
        ui.checkbox(&mut self.create_preset.save_scene, "Save scene")
            .on_hover_text(SAVE_SCENE_MSG);
    }

    pub fn bottom_panel(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            let pause_button_label = if self.paused { "Unpause" } else { "Pause" };
            if ui.button(pause_button_label).clicked() {
                self.paused = !self.paused;
            }
            if ui
                .add_enabled(self.paused, Button::new("Step"))
                .on_hover_text(self.keybind_step_sim.show())
                .clicked()
            {
                self.scene.update();
            }
            if ui.add_enabled(self.speed > 1, Button::new("-")).clicked() {
                self.speed /= 2;
            }
            if ui.button("+").clicked() {
                self.speed *= 2;
            }
            ui.label(format!("{} (speed)", self.speed));
        });

        let label = {
            let count =
                self.scene.devices.len() + self.scene.inputs.len() + self.scene.outputs.len();
            if count > 0 {
                format!("Clear ({count} items)")
            } else {
                String::from("Clear")
            }
        };
        if ui.button(label).clicked() {
            self.scene = Scene::new();
            self.selected_devices.clear();
        }

        ui.label("Auto link: ").on_hover_text(AUTO_LINK_MSG);
        ui.checkbox(&mut self.auto_link, "")
            .on_hover_text(self.keybind_toggle_auto_link.show());
    }

    pub fn central_panel(&mut self, ui: &mut Ui, input: &FrameInput, output: &mut FrameOutput) {
        let (painter_rs, painter) = ui.allocate_painter(ui.available_size(), Sense::hover());
        self.scene.rect = painter_rs.rect;
        self.view.origin = self.scene.rect.min;

        let mut g = Graphics::new(ui, input.pointer_pos);

        if let Some(item) = self.show_scene(&mut g) {
            output.hovered.cond_replace(AppItem::SceneItem(item));
        }
        self.show_selected_devices(&mut g, input);

        // --- show links to pointer ---
        for idx in (0..self.link_starts.len()).rev() {
            let link_start = self.link_starts[idx].clone();
            let Some(state) = self.scene.link_start_state(link_start) else {
            	self.link_starts.remove(idx);
            	continue;
            };
            use graphics::{link_start_pos, show_link};

            let pos = link_start_pos(&self.settings, &self.view, &self.scene, link_start).unwrap();
            show_link(&mut g, &self.settings, state, pos, input.pointer_pos);
        }
        if input.pressed(Key::Escape) {
            self.link_starts.clear();
            self.held_presets.clear();
        }

        // --- show held presets ---
        if self.held_presets.len() > 1 {
            g.text(
                input.pointer_pos + Vec2::new(30.0, 0.0),
                20.0,
                &format!("{}", self.held_presets.len()),
                Color32::WHITE,
                Align2::LEFT_CENTER,
            );
        }

        let mut pos = input.pointer_pos + Vec2::new(0.0, 10.0);
        for name in &self.held_presets {
            let preset = self.presets.get_preset(name).unwrap();

            graphics::show_device_preset(&mut g, &self.settings, &self.view, pos, preset);
            pos.y += preset.size(&self.settings).y * self.view.scale();
        }
        painter.extend(g.shapes);

        match self.show_edit_popup(ui, input) {
            None => self.edit_popup = None,
            Some(true) => output.hovered.cond_replace(AppItem::EditPopup),
            Some(_) => {}
        }
        let request_focus = input.pressed(Key::Space) && !self.focused_text_field;
        let rs = self.preset_placer.show(
            self.settings.preset_placer_pos,
            ui,
            input,
            &self.presets,
            request_focus,
        );
        if rs.hovered {
            output.hovered.cond_replace(AppItem::PresetPlacer);
        }
        if rs.has_focus {
            self.focused_text_field = true;
        }
        if let Some(preset) = rs.picked {
            self.held_presets.push(preset);
        }
        if self.dev_options.enabled {
            DevOptions::show(ui, input, output, self);
        }
        self.dev_options.input(input);
    }

    pub fn show_edit_popup(&mut self, ui: &mut Ui, input: &FrameInput) -> Option<bool> {
        let Some(item) = &self.edit_popup else {
        	return None
        };
        let col_w = self.settings.scene_pin_col_w * self.view.scale();
        let rect = match item {
            SceneItem::InputBulb(id) => {
                let size = Vec2::new(100.0, 20.0);
                let y = graphics::scene_input_view_y(&self.scene, *id, &self.view)?;
                let pos = Pos2::new(self.scene.rect.min.x + col_w, y);
                Rect::from_min_size(pos, size)
            }
            SceneItem::OutputBulb(id) => {
                let y = graphics::scene_output_view_y(&self.scene, *id, &self.view)?;
                let size = Vec2::new(100.0, 20.0);
                let pos = Pos2::new(self.scene.rect.max.x - col_w - size.x, y);
                Rect::from_min_size(pos, size)
            }
            SceneItem::InputGroup(id) => {
                let group = self.scene.input_groups.get(id)?;
                let size = Vec2::new(100.0, 60.0);
                let pos = Pos2::new(
                    self.scene.rect.min.x + col_w,
                    graphics::map_io_y(&self.view, group.bottom_y(IoSel::Input, &self.scene))
                        + graphics::GROUP_HEADER_SIZE,
                );
                Rect::from_min_size(pos, size)
            }
            SceneItem::OutputGroup(id) => {
                let group = self.scene.output_groups.get(id)?;
                let size = Vec2::new(100.0, 60.0);
                let pos = Pos2::new(
                    self.scene.rect.max.x - col_w - size.x,
                    graphics::map_io_y(&self.view, group.bottom_y(IoSel::Output, &self.scene))
                        + graphics::GROUP_HEADER_SIZE,
                );
                Rect::from_min_size(pos, size)
            }
            _ => unreachable!(),
        };
        let mut child_ui = ui.child_ui(rect, ui.layout().clone());
        let rs = Frame::menu(child_ui.style()).show(&mut child_ui, |ui| match item.clone() {
            SceneItem::InputBulb(id) => {
                let input = self.scene.inputs.get(&id).unwrap();
                let group_count = input
                    .io
                    .group_member
                    .map(|id| self.scene.input_groups.get(&id).unwrap().members.len());
                let input = self.scene.inputs.get_mut(&id).unwrap();
                ui.horizontal(|ui| {
                    ui.label("name: ");
                    ui.text_edit_singleline(&mut input.io.name).request_focus();
                    self.focused_text_field = true;
                });
                ui.horizontal(|ui| {
                    ui.add_space(5.0);
                    let label = if let Some(count) = group_count {
                        format!("stack ({count})")
                    } else {
                        String::from("stack")
                    };
                    if ui.button(label).clicked() {
                        self.scene.stack_input(id, &self.settings);
                    }
                    if ui.button("delete").clicked() {
                        self.scene.remove_input(id);
                    }
                });
            }
            SceneItem::OutputBulb(id) => {
                let output = self.scene.outputs.get(&id).unwrap();
                let group_count = output
                    .io
                    .group_member
                    .map(|id| self.scene.output_groups.get(&id).unwrap().members.len());
                let output = self.scene.outputs.get_mut(&id).unwrap();
                ui.horizontal(|ui| {
                    ui.label("name: ");
                    ui.text_edit_singleline(&mut output.io.name).request_focus();
                    self.focused_text_field = true;
                });
                ui.horizontal(|ui| {
                    ui.add_space(5.0);
                    let label = if let Some(count) = group_count {
                        format!("stack ({count})")
                    } else {
                        String::from("stack")
                    };
                    if ui.button(label).clicked() {
                        self.scene.stack_output(id, &self.settings);
                    }
                    if ui.button("delete").clicked() {
                        self.scene.remove_output(id);
                    }
                });
            }
            SceneItem::InputGroup(id) => {
                let group = self.scene.input_groups.get_mut(&id).unwrap();
                ui.horizontal(|ui| {
                    ui.label("signed: ")
                        .on_hover_text("Read the bits as a signed integer");
                    ui.checkbox(&mut group.signed, "");
                });
                ui.horizontal(|ui| {
                    ui.label("lsb: ").on_hover_text("least significant bit");
                    let lsb = if group.lsb_top { "top" } else { "bottom" };
                    if ui.button(lsb).clicked() {
                        group.lsb_top = !group.lsb_top;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("display: ");
                    let display = if group.hex { "hex" } else { "decimal" };
                    if ui.button(display).clicked() {
                        group.hex = !group.hex;
                    }
                });
            }
            SceneItem::OutputGroup(id) => {
                let group = self.scene.output_groups.get_mut(&id).unwrap();
                ui.horizontal(|ui| {
                    ui.label("signed: ")
                        .on_hover_text("Read the bits as a signed integer");
                    ui.checkbox(&mut group.signed, "");
                });
                ui.horizontal(|ui| {
                    ui.label("lsb: ").on_hover_text("least significant bit");
                    let lsb = if group.lsb_top { "top" } else { "bottom" };
                    if ui.button(lsb).clicked() {
                        group.lsb_top = !group.lsb_top;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("display: ");
                    let display = if group.hex { "hex" } else { "decimal" };
                    if ui.button(display).clicked() {
                        group.hex = !group.hex;
                    }
                });
            }
            _ => unreachable!(),
        });
        if !rs.response.hovered() && input.pressed_prim {
            self.edit_popup = None;
        }
        Some(rs.response.hovered())
    }

    pub fn show_scene(&mut self, g: &mut Graphics) -> Option<SceneItem> {
        let mut dead_links = Vec::new();
        let output_link_err = match self.link_starts.last() {
            Some(LinkStart::Input(_)) => true,
            _ => false,
        };
        let hovered = graphics::show_scene(
            g,
            &self.settings,
            &self.view,
            &self.scene,
            &mut dead_links,
            output_link_err,
            self.dev_options.show_device_ids(),
        );

        // --- Remove the dead links ---
        // the dead links need to be sorted in reverse by the link index
        // so that when I go through and remove them, I don't invalidate other link indices
        dead_links.sort_by(|a, b| a.1.cmp(&b.1).reverse());
        for (link_start, link_idx) in dead_links {
            match link_start {
                LinkStart::Input(input) => {
                    let input = self.scene.inputs.get_mut(&input).unwrap();
                    input.links.remove(link_idx);
                }
                LinkStart::DeviceOutput(device, output) => {
                    let device = self.scene.devices.get_mut(&device).unwrap();
                    device.links[output].remove(link_idx);
                }
            };
        }
        hovered
    }

    pub fn scene_input(&mut self, input: &FrameInput) {
        let AppItem::SceneItem(item) = input.prev_hovered else {
    		return;
    	};
        let try_link = self.auto_link && input.hovered_changed;
        match item {
            SceneItem::Device(id) => {
                if input.pressed(Key::Backspace) {
                    if self.selected_devices.contains(&id) {
                        for id in &self.selected_devices {
                            self.scene.remove_device(*id);
                        }
                        self.selected_devices.clear();
                    } else {
                        self.scene.remove_device(id);
                    }
                }
                if input.pressed_prim && input.modifiers.shift {
                    if !self.selected_devices.contains(&id) {
                        self.selected_devices.push(id);
                    }
                }
            }
            SceneItem::InputBulb(id) => {
                if input.clicked_prim {
                    let state = self.scene.inputs.get(&id).unwrap().io.state;
                    self.scene.set_input(id, !state);
                }
                if input.pressed_sec {
                    self.edit_popup = Some(item);
                }
                if input.pressed(Key::Backspace) && !self.focused_text_field {
                    self.scene.remove_input(id);
                }
                if input.pressed(Key::ArrowDown) {
                    self.scene.stack_input(id, &self.settings);
                }
                if input.pressed(Key::ArrowUp) {
                    self.scene.unstack_input(id);
                }
            }
            SceneItem::InputPin(id) => {
                if input.pressed_prim || try_link {
                    self.start_link(LinkStart::Input(id));
                }
            }
            SceneItem::InputLink(input_id, link_idx) => {
                if input.pressed(Key::Backspace) {
                    let links = &mut self.scene.inputs.get_mut(&input_id).unwrap().links;
                    let link = links[link_idx].clone();
                    links.remove(link_idx);
                    self.scene.write_queue.push(link.wrap(), false);
                }
            }
            SceneItem::InputGroup(_) => {
                if input.pressed_sec {
                    self.edit_popup = Some(item);
                }
            }
            SceneItem::OutputBulb(id) => {
                if input.pressed(Key::Backspace) {
                    self.scene.remove_output(id);
                }
                if input.pressed_sec {
                    self.edit_popup = Some(item);
                }
                if input.pressed(Key::ArrowDown) && !self.focused_text_field {
                    self.scene.stack_output(id, &self.settings);
                }
                if input.pressed(Key::ArrowUp) {
                    self.scene.unstack_output(id);
                }
            }
            SceneItem::OutputGroup(_) => {
                if input.pressed_sec {
                    self.edit_popup = Some(item);
                }
            }
            SceneItem::OutputPin(id) => {
                if input.pressed_prim || try_link {
                    self.finish_link(LinkTarget::Output(id));
                }
            }
            SceneItem::DeviceInput(device, device_input) => {
                let mut created_link = false;
                if input.pressed_prim || try_link {
                    created_link = self.finish_link(LinkTarget::DeviceInput(device, device_input));
                }
                if input.pressed_prim && !created_link {
                    let state = self.scene.get_device_input(device, device_input).unwrap();
                    self.scene.set_device_input(device, device_input, !state);
                }
            }
            SceneItem::DeviceOutput(device, output) => {
                if input.pressed_prim || try_link {
                    self.start_link(LinkStart::DeviceOutput(device, output));
                }
                if input.pressed(Key::Backspace) {
                    let device = self.scene.devices.get_mut(&device).unwrap();
                    device.links[output].clear();
                }
            }
            SceneItem::DeviceOutputLink(device_id, output_idx, link_idx) => {
                if input.pressed(Key::Backspace) {
                    let links =
                        &mut self.scene.devices.get_mut(&device_id).unwrap().links[output_idx];
                    let link = links[link_idx].clone();
                    links.remove(link_idx);
                    self.scene.write_queue.push(link, false);
                }
            }
        }
    }

    pub fn show_selected_devices(&mut self, g: &mut Graphics, input: &FrameInput) {
        let mut clear = input.prev_hovered != AppItem::Other;
        for device_id in &self.selected_devices {
            if input.prev_hovered == AppItem::SceneItem(SceneItem::Device(*device_id)) {
                clear = false;
            }
            let device = self.scene.devices.get(device_id).unwrap();
            let pos = graphics::device_pos(device, &self.view);
            let size = graphics::device_size(device, &self.settings, &self.view);
            let rect = Rect::from_min_size(pos, size);
            let (rounding, stroke) = (Rounding::same(2.0), Stroke::new(2.0, Color32::WHITE));
            g.shapes.push(Shape::rect_stroke(rect, rounding, stroke));
        }
        if input.clicked_prim && !input.modifiers.shift && clear {
            self.selected_devices.clear();
        }
    }

    pub fn clone_selected_devices(&mut self, pointer_pos: Pos2) {
        let mut selection_min = Pos2::new(f32::INFINITY, f32::INFINITY);
        let mut devices = Vec::with_capacity(self.selected_devices.len());
        for device_id in &self.selected_devices {
            let device = self.scene.devices.get(device_id).unwrap();
            selection_min.x = f32::min(selection_min.x, device.pos.x);
            selection_min.y = f32::min(selection_min.y, device.pos.y);
            devices.push(device.clone());
        }
        let offset = self.view.unmap_pos(pointer_pos) - selection_min;
        let mut ids = Vec::with_capacity(devices.len());
        for mut device in devices {
            device.pos += offset;
            let id = rand_id();
            self.scene.add_device(id, device);
            ids.push(id);
        }
        self.selected_devices = ids;
    }

    pub fn update(&mut self, ctx: &Context, input: &FrameInput) -> FrameOutput {
        let mut output = FrameOutput::default();
        self.focused_text_field = false;

        if self.scene.rect.contains(input.pointer_pos) {
            output.hovered.cond_replace(AppItem::SceneBackground);
        } else {
            output.hovered.cond_replace(AppItem::Other);
        }

        // --- Update sim ---
        if !self.paused {
            for _ in 0..self.speed {
                self.scene.update();
            }
        }

        // --- Show UI ---
        ctx.set_visuals(self.settings.visuals());
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| self.top_panel(ui, &mut output));
        });
        TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| self.bottom_panel(ui));
        });

        let scene_rs = CentralPanel::default()
            .show(ctx, |ui| self.central_panel(ui, &input, &mut output))
            .response;

        // --- Handle key binds ---
        if input.keybind_used(&self.keybind_toggle_auto_link) {
            self.auto_link = !self.auto_link;
        }
        if self.paused && input.keybind_used(&self.keybind_step_sim) {
            self.scene.update();
        }
        if self.selected_devices.len() > 0 && input.keybind_used(&self.keybind_duplicate_devices) {
            self.clone_selected_devices(input.pointer_pos);
        }

        // --- Handle scene input ---
        self.scene_input(&input);

        // --- Handle dragging ---
        if let Some((delta, item)) = input.drag_delta() {
            match item {
                AppItem::SceneBackground => {
                    self.view.drag(delta);
                }
                AppItem::PresetPlacer => {
                    self.settings.preset_placer_pos += delta;
                }
                AppItem::DevOptions => {
                    self.dev_options.pos += delta;
                }
                AppItem::SceneItem(SceneItem::InputBulb(id)) => {
                    self.scene.drag_input(id, delta * self.view.inv_scale());
                }
                AppItem::SceneItem(SceneItem::OutputBulb(id)) => {
                    self.scene.drag_output(id, delta * self.view.inv_scale());
                }
                AppItem::SceneItem(SceneItem::Device(id)) => {
                    if self.selected_devices.contains(&id) {
                        for id in &self.selected_devices {
                            self.scene.drag_device(*id, delta * self.view.inv_scale());
                        }
                    } else {
                        self.scene.drag_device(id, delta * self.view.inv_scale());
                    }
                }
                _ => {}
            }
        }

        // --- Handle scrolling ---
        self.view.drag(input.scroll_delta);

        // --- Handle zooming ---
        let zoom_delta = ctx.input().zoom_delta();
        if zoom_delta != 1.0 {
            let pos = input.pointer_pos - self.scene.rect.min;
            self.view.zoom(zoom_delta, pos.to_pos2());
        }

        // --- Handle placing inputs & outputs
        if input.pressed_prim && input.prev_hovered == AppItem::SceneBackground {
            let col_w = self.settings.scene_pin_col_w * self.view.scale();
            let output_col_x = self.scene.rect.max.x - col_w;
            let input_col_x = self.scene.rect.min.x + col_w;
            let x = input.pointer_pos.x;
            let y = graphics::unmap_io_y(&self.view, input.pointer_pos.y);

            if x < input_col_x {
                self.scene.add_input(y);
                output.void_click = true;
            } else if x > output_col_x {
                self.scene.add_output(y);
                output.void_click = true;
            }
        }

        // --- Handle placing presets ---
        let can_place_preset = match input.prev_hovered {
            AppItem::SceneBackground => true,
            AppItem::SceneItem(SceneItem::Device(..)) => true,
            AppItem::SceneItem(SceneItem::DeviceInput(..)) => true,
            AppItem::SceneItem(SceneItem::DeviceOutput(..)) => true,
            AppItem::SceneItem(SceneItem::DeviceOutputLink(..)) => true,
            AppItem::SceneItem(SceneItem::InputLink(..)) => true,
            _ => false,
        };
        if self.held_presets.len() > 0 && input.pressed_prim && can_place_preset {
            let mut held_presets = Vec::new();
            std::mem::swap(&mut held_presets, &mut self.held_presets);

            let mut pos = self
                .view
                .unmap_pos(input.pointer_pos + Vec2::new(0.0, 10.0));
            for name in held_presets {
                self.place_preset(&name, pos);

                let preset = self.presets.get_preset(&name).unwrap();
                pos.y += preset.size(&self.settings).y;
            }
        }

        // --- Handle context menu ---
        if input.pressed_sec {
            self.context_menu_pos = input.pointer_pos;
        }
        scene_rs.context_menu(|ui| {
            if !can_place_preset {
                ui.close_menu();
                return;
            }

            ui.set_width(100.0);
            const LEFT_SP: f32 = 15.0;
            let mut place_preset = None;

            for (cat, presets) in self.presets.cats_sorted() {
                ui.menu_button(cat, |ui| {
                    ui.set_width(100.0);
                    for preset in presets {
                        if ui.button(&preset.name).clicked() {
                            place_preset = Some(preset.name.clone());
                            ui.close_menu();
                        }
                    }
                });
            }

            if self.dev_options.enabled {
                if ui.button("debug").clicked() {
                    println!("{:#?}", self.scene);
                }
            }
            if let Some(name) = place_preset {
                self.place_preset(&name, self.view.unmap_pos(self.context_menu_pos));
            }
        });
        output
    }

    pub fn on_exit(&mut self) {}
}
