use crate::graphics::{Graphics, SceneItem, View};
use crate::input::{FrameInput, CMD_KEY};
use crate::preset::{ChipPreset, CombGatePreset, DevicePreset, PresetData, PresetSource, Presets};
use crate::scene::{Device, Scene};
use crate::settings::{ErrResult, Settings};
use crate::*;
use eframe::egui::*;
use std::time::SystemTime;

const COMBINATIONAL_MSG: &str = "will use a truth table for the created preset";
const SAVE_SCENE_MSG: &str =
    "store state of scene along side the created preset,\nallowing the preset to be later modified";
const AUTO_LINK_MSG: &str = "automatically start/finish a link when you hover a pin";
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

#[derive(Clone)]
struct PresetPlacer {
    pub pos: Pos2,
    pub width: f32,
    pub first_frame: bool,
    pub field: String,
    pub results: Vec<String>,
    pub sel: usize,
}
impl PresetPlacer {
    pub fn new(pos: Pos2) -> Self {
        Self {
            pos,
            width: 100.0,
            first_frame: true,
            field: String::new(),
            results: Vec::new(),
            sel: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppItem {
    SceneItem(SceneItem),
    SceneBackground,
    EditPopup,
    PresetPlacer,
    Other,
}

pub struct App {
    settings: Settings,
    presets: Presets,

    scene: Scene,
    view: View,
    paused: bool,
    speed: u32,

    input: FrameInput,
    /// Where the pointer was when the context menu was opened
    context_menu_pos: Pos2,
    /// What preset we've selected in the presets menu
    preset_menu_sel: Option<String>,
    /// What app item is the pointer over this frame
    hovered: AppItem,
    /// What app item was the pointer over previous frame
    prev_hovered: AppItem,
    /// If we right click on some scene item, shows a popup
    edit_popup: Option<SceneItem>,
    /// If we started creating some links
    link_starts: Vec<LinkStart<u64>>,
    /// The config for creating a new preset from the scene
    create_preset: CreatePreset,
    /// The small window for searching and placing presets
    preset_placer: Option<PresetPlacer>,
    /// A list of the presets we've picked from the preset picker
    held_presets: Vec<String>,
    /// If we've selected multiple devices for bulk actions
    selected_devices: Vec<u64>,
    /// If true, we should automatically start/finish a link when we hover the pin
    auto_link: bool,
    /// The time of the most recent auto-save
    last_save: SystemTime,

    debug_device_ids: bool,
    debug_misc: bool,
}
impl App {
    pub fn new() -> Self {
        let settings = settings::load_settings().unwrap_res_or(Settings::default);
        let presets = settings::load_presets().unwrap_res_or(Presets::default);
        let scene = settings::load_scene().unwrap_res_or(Scene::new);

        Self {
            settings,
            presets,

            scene,
            view: View::default(),
            paused: false,
            speed: 1,

            input: FrameInput::default(),
            context_menu_pos: Pos2::ZERO,
            preset_menu_sel: None,
            hovered: AppItem::Other,
            prev_hovered: AppItem::Other,
            edit_popup: None,

            link_starts: Vec::new(),
            create_preset: CreatePreset::default(),
            preset_placer: None,
            held_presets: Vec::new(),
            selected_devices: Vec::new(),
            auto_link: false,
            last_save: SystemTime::now(),

            debug_device_ids: false,
            debug_misc: false,
        }
    }

    pub fn create(&mut self) {
        let mut create_preset = CreatePreset {
            name: CreatePreset::default().name,
            ..self.create_preset.clone()
        };
        std::mem::swap(&mut create_preset, &mut self.create_preset);

        let CreatePreset {
            name,
            color,
            cat,
            combinational,
            save_scene,
            ..
        } = create_preset;

        let data = if combinational {
            match CombGatePreset::from_scene(&mut self.scene) {
                Ok(v) => PresetData::CombGate(v),
                Err(e) => {
                    eprintln!("Can't create combination gate: {}", e);
                    return;
                }
            }
        } else {
            let chip = ChipPreset::from_scene(&self.scene);
            PresetData::Chip(chip)
        };
        let preset = DevicePreset {
            data,
            name,
            color: color.to_array(),
            src: PresetSource::Scene(save_scene.then(|| self.scene.clone())),
            cat,
        };
        self.presets.add_preset(preset);
        self.scene = Scene::new();
    }
    pub fn place_preset(&mut self, name: &str, pos: Pos2) {
        if let Some(preset) = self.presets.get_preset(name) {
            let device = Device::from_preset(preset, pos);
            self.scene.add_device(rand_id(), device);
        }
    }
    pub fn load_preset(&mut self, name: &str) {
        let preset = self.presets.get_preset(name).unwrap().clone();
        let PresetSource::Scene(Some(scene)) = preset.src.clone() else {
        	eprintln!("this preset doesn't have a scene source!");
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
        let Some(link) = self.link_starts.last().cloned() else {
        	return false;
        };
        let new_link = match link {
            LinkStart::DeviceOutput(device, output) => {
                NewLink::DeviceOutputTo(device, output, target)
            }
            LinkStart::Input(id) => {
                let device_input = match target {
                    LinkTarget::DeviceInput(device, input) => DeviceInput(device, input),
                    LinkTarget::Output(_) => {
                        println!("A scene input can't be linked to a scene output");
                        return false;
                    }
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

    pub fn settings_menu(&mut self, ui: &mut Ui) {
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
            if ui.button("reload").clicked() {
                match settings::load_settings() {
                    Ok(settings) => self.settings = settings,
                    Err(err) => err.log(),
                }
            }
            if ui.button("reset").clicked() {
                self.settings = Settings::default();
            }
        });
        ui.separator();

        let s = &mut self.settings;

        ui.heading("App");
        checkbox(ui, "dark mode: ", &mut s.dark_mode);
        checkbox(ui, "high contrast: ", &mut s.high_contrast);
        ui.horizontal(|ui| {
            ui.add_space(SPACE);
            if ui.button("open config folder").clicked() {
                let path = String::from(settings::config_dir().to_str().unwrap());
                if let Err(err) = settings::reveal_dir(&path) {
                    eprintln!("failed to open config dir {:?}: {:?}", path, err);
                }
            }
        });
        let mut save = false;
        ui.horizontal(|ui| {
            ui.add_space(SPACE);
            if ui.button("save").clicked() {
                save = true;
            }
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

        if s.dev_options {
            ui.heading("Debug");
            checkbox(ui, "device IDs: ", &mut self.debug_device_ids);
            checkbox(ui, "misc: ", &mut self.debug_misc);
        }

        if save {
            settings::save_settings(&self.settings).log_err();
            settings::save_presets(&mut self.presets).log_err();
            settings::save_scene(&self.scene).log_err();
        }
    }

    pub fn presets_menu(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("Presets");
            if ui.button("reload").clicked() {
                match settings::load_presets() {
                    Ok(presets) => self.presets = presets,
                    Err(err) => err.log(),
                }
            }
            if ui.button("import").clicked() {
                if let Some(mut path) = rfd::FileDialog::new().pick_folder() {
                    match settings::load_presets_at(&mut path) {
                        Ok(presets) => self.presets.merge(&presets),
                        Err(err) => err.log(),
                    }
                }
            }
        });
        ui.separator();

        'sel: {
            let Some(preset) = &self.preset_menu_sel else {
        		break 'sel;
        	};
            let Some(preset) = self.presets.get_preset(preset) else {
        		self.preset_menu_sel = None;
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

            let mut load = false;
            let mut delete = false;
            ui.horizontal(|ui| {
                delete = ui.add_enabled(can_del, Button::new("delete")).clicked();
                load = ui.add_enabled(can_load, Button::new("load")).clicked();
                if self.settings.dev_options {
                    if ui.button("debug").clicked() {
                        println!("{:#?}", preset);
                    }
                }
            });

            match (load, delete) {
                (true, _) => self.load_preset(&name),
                (_, true) => self.presets.remove_preset(&name),
                _ => {}
            }
            ui.separator();
        }

        let mut sel_preset: Option<String> = None;
        for (cat_name, presets) in self.presets.cats_sorted() {
            ui.collapsing(cat_name, |ui| {
                for preset in presets {
                    let rs = ui.button(&preset.name);
                    if rs.clicked() {
                        sel_preset = Some(preset.name.clone());
                    }
                    if self.preset_menu_sel.as_ref() == Some(&preset.name) {
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
            self.preset_menu_sel = Some(preset);
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
            ui.text_edit_singleline(&mut self.create_preset.cat);
            ui.add_space(5.0);
        });
        ui.add_space(5.0);
    }

    pub fn top_panel(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.menu_button("Settings", |ui| {
                self.hovered = AppItem::Other;
                *ui.visuals_mut() = self.settings.visuals();
                self.settings_menu(ui);
            });
            ui.menu_button("Presets", |ui| {
                self.hovered = AppItem::Other;
                *ui.visuals_mut() = self.settings.visuals();
                self.presets_menu(ui);
            });

            if ui.button("Create").on_hover_text(CREATE_MSG).clicked() {
                self.create();
            }
        });

        let rs = ui.color_edit_button_srgba(&mut self.create_preset.color);
        // there doesn't seem to be a way to check if the pointer is over the color picker.
        // We only get access to the button. But we can check if the color picker has changed.
        if rs.changed() {
            self.hovered = AppItem::Other;
        }

        ui.menu_button(self.create_preset.cat.clone(), |ui| {
            self.hovered = AppItem::Other;
            *ui.visuals_mut() = self.settings.visuals();
            self.cat_menu(ui);
        });

        ui.style_mut().spacing.text_edit_width = 100.0;
        ui.text_edit_singleline(&mut self.create_preset.name);

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
                .on_hover_text(format!("{CMD_KEY} + S"))
                .clicked()
            {
                self.scene.update();
            }
            ui.add(Slider::new(&mut self.speed, 1..=999));
        });

        if ui.button("Clear").clicked() {
            self.scene = Scene::new();
        }

        ui.label("Auto link: ").on_hover_text(AUTO_LINK_MSG);
        ui.checkbox(&mut self.auto_link, "")
            .on_hover_text(format!("{CMD_KEY} + L"));
    }

    pub fn central_panel(&mut self, ui: &mut Ui) {
        let (painter_rs, painter) = ui.allocate_painter(ui.available_size(), Sense::hover());
        self.scene.rect = painter_rs.rect;
        self.view.origin = self.scene.rect.min;

        let mut g = Graphics::new(ui, painter_rs.rect, self.input.pointer_pos);

        self.show_scene(&mut g);
        painter.extend(g.shapes.drain(..));

        // SHOW LINKS TO CURSOR
        for idx in (0..self.link_starts.len()).rev() {
            let link_start = &self.link_starts[idx];
            if let Some(state) = self.scene.get_link_start(link_start) {
                use graphics::{link_start_pos, show_link};

                if let Some(pos) =
                    link_start_pos(&self.settings, &self.view, &self.scene, *link_start)
                {
                    show_link(&mut g, &self.settings, state, pos, self.input.pointer_pos);
                } else {
                    self.link_starts.remove(idx);
                }
            } else {
                self.link_starts.remove(idx);
            }

            if self.input.pressed_esc {
                self.link_starts.remove(idx);
            }
        }

        // SHOW HELD PRESETS
        if self.input.pressed_esc {
            self.held_presets.clear();
        }

        if self.held_presets.len() > 1 {
            g.text(
                self.input.pointer_pos + Vec2::new(30.0, 0.0),
                20.0,
                &format!("{}", self.held_presets.len()),
                Color32::WHITE,
                Align2::LEFT_CENTER,
            );
        }

        let mut pos = self.input.pointer_pos + Vec2::new(0.0, 10.0);
        for name in &self.held_presets {
            let preset = self.presets.get_preset(name).unwrap();

            graphics::show_device_preset(&mut g, &self.settings, &self.view, pos, preset);
            pos.y += preset.size(&self.settings).y * self.view.scale();
        }

        // SHOW PRESET PLACER
        self.show_preset_placer(&mut g.ui);

        painter.extend(g.shapes);

        // SHOW EDIT POPUP
        if self.show_edit_popup(ui).is_none() {
            self.edit_popup = None;
        }
    }

    pub fn show_edit_popup(&mut self, ui: &mut Ui) -> Option<()> {
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
                    graphics::map_io_y(&self.view, group.input_bottom_y(&self.scene))
                        + graphics::GROUP_HEADER_SIZE,
                );
                Rect::from_min_size(pos, size)
            }
            SceneItem::OutputGroup(id) => {
                let group = self.scene.output_groups.get(id)?;
                let size = Vec2::new(100.0, 60.0);
                let pos = Pos2::new(
                    self.scene.rect.max.x - col_w - size.x,
                    graphics::map_io_y(&self.view, group.output_bottom_y(&self.scene))
                        + graphics::GROUP_HEADER_SIZE,
                );
                Rect::from_min_size(pos, size)
            }
            _ => unreachable!(),
        };
        let mut child_ui = ui.child_ui(rect, ui.layout().clone());
        let rs = Frame::menu(child_ui.style()).show(&mut child_ui, |ui| match item.clone() {
            SceneItem::InputBulb(id) => {
                let input = self.scene.inputs.get_mut(&id).unwrap();
                ui.horizontal(|ui| {
                    ui.label("name: ");
                    ui.text_edit_singleline(&mut input.name);
                });
            }
            SceneItem::OutputBulb(id) => {
                let output = self.scene.outputs.get_mut(&id).unwrap();
                ui.horizontal(|ui| {
                    ui.label("name: ");
                    ui.text_edit_singleline(&mut output.name);
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
        if !rs.response.hovered() && self.input.pressed_prim {
            self.edit_popup = None;
        }
        if rs.response.hovered() {
            self.hovered = AppItem::EditPopup;
        }
        Some(())
    }

    pub fn show_scene(&mut self, g: &mut Graphics) {
        let mut dead_links = Vec::new();
        let scene_hovered = graphics::show_scene(
            g,
            &self.settings,
            &self.view,
            &self.scene,
            &mut dead_links,
            self.debug_device_ids,
        );

        // HANDLE DEAD LINKS
        dead_links.sort_by(|a, b| a.1.cmp(&b.1).reverse());
        for (start, link_idx) in dead_links {
            match start {
                LinkStart::Input(input) => {
                    self.scene
                        .inputs
                        .get_mut(&input)
                        .unwrap()
                        .links
                        .remove(link_idx);
                }
                LinkStart::DeviceOutput(device, output) => {
                    self.scene.devices.get_mut(&device).unwrap().links[output].remove(link_idx);
                }
            };
        }

        // HANDLE SCENE INTERACTIONS
        if let Some(item) = scene_hovered {
            self.hovered = AppItem::SceneItem(item);
            let try_link = self.auto_link && self.prev_hovered != AppItem::SceneItem(item);

            match item {
                SceneItem::Device(id) => {
                    if self.input.pressed_del {
                        self.scene.del_device(id);
                    }
                    if self.input.pressed_prim && self.input.shift {
                        self.selected_devices.push(id);
                    }
                }
                SceneItem::InputBulb(id) => {
                    if self.input.clicked_prim {
                        let state = self.scene.inputs.get(&id).unwrap().state;
                        self.scene.set_input(id, !state);
                    }
                    if self.input.pressed_sec {
                        self.edit_popup = Some(item);
                    }
                    if self.input.pressed_del {
                        self.scene.del_input(id);
                    }
                    if self.input.pressed_down {
                        self.scene.stack_input(id, &self.settings);
                    }
                    if self.input.pressed_up {
                        self.scene.unstack_input(id);
                    }
                }
                SceneItem::InputPin(id) => {
                    if self.input.pressed_prim || try_link {
                        self.start_link(LinkStart::Input(id));
                    }
                }
                SceneItem::InputLink(input_id, link_idx) => {
                    if self.input.pressed_del {
                        let links = &mut self.scene.inputs.get_mut(&input_id).unwrap().links;
                        let link = links[link_idx].clone();
                        links.remove(link_idx);
                        self.scene.write_queue.push(link.wrap(), false);
                    }
                }
                SceneItem::InputGroup(_) => {
                    if self.input.pressed_sec {
                        self.edit_popup = Some(item);
                    }
                }
                SceneItem::OutputBulb(id) => {
                    if self.input.pressed_del {
                        self.scene.del_output(id);
                    }
                    if self.input.pressed_sec {
                        self.edit_popup = Some(item);
                    }
                    if self.input.pressed_down {
                        self.scene.stack_output(id, &self.settings);
                    }
                    if self.input.pressed_up {
                        self.scene.unstack_output(id);
                    }
                }
                SceneItem::OutputGroup(_) => {
                    if self.input.pressed_sec {
                        self.edit_popup = Some(item);
                    }
                }
                SceneItem::OutputPin(id) => {
                    if self.input.pressed_prim || try_link {
                        self.finish_link(LinkTarget::Output(id));
                    }
                }
                SceneItem::DeviceInput(device, input) => {
                    let mut created_link = false;
                    if self.input.pressed_prim || try_link {
                        created_link = self.finish_link(LinkTarget::DeviceInput(device, input));
                    }
                    if self.input.pressed_prim && !created_link {
                        let state = self.scene.get_device_input(device, input).unwrap();
                        self.scene.set_device_input(device, input, !state);
                    }
                }
                SceneItem::DeviceOutput(device, output) => {
                    if self.input.pressed_prim || try_link {
                        self.start_link(LinkStart::DeviceOutput(device, output));
                    }
                    if self.input.pressed_del {
                        let device = self.scene.devices.get_mut(&device).unwrap();
                        device.links[output].clear();
                    }
                }
                SceneItem::DeviceOutputLink(device_id, output_idx, link_idx) => {
                    if self.input.pressed_del {
                        let links =
                            &mut self.scene.devices.get_mut(&device_id).unwrap().links[output_idx];
                        let link = links[link_idx].clone();
                        links.remove(link_idx);
                        self.scene.write_queue.push(link, false);
                    }
                }
            }
        }
    }

    pub fn show_preset_placer(&mut self, ui: &mut Ui) {
        if self.input.pressed_esc
            || (self.input.clicked_prim && self.prev_hovered != AppItem::PresetPlacer)
        {
            self.preset_placer = None;
            return;
        }
        let Some(picker) = &mut self.preset_placer else {
        	return
        };
        let size = Vec2::new(picker.width, 20.0);
        let rect = Rect::from_min_size(picker.pos, size);

        if self.input.pressed_up && picker.sel > 0 {
            picker.sel -= 1;
        }
        if self.input.pressed_down && picker.sel < picker.results.len() - 1 {
            picker.sel += 1;
        }
        if picker.sel >= picker.results.len() && picker.sel != 0 {
            picker.sel = picker.results.len() - 1;
        }

        let mut place_preset = false;
        let mut grab_preset = None;
        let mut ui = ui.child_ui(rect, ui.layout().clone());
        let frame_rs = Frame::menu(ui.style()).show(&mut ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(5.0);
                let rs = ui.text_edit_singleline(&mut picker.field);
                if rs.lost_focus() && self.input.pressed_enter {
                    place_preset = true;
                }
                rs.request_focus();
            });
            for (idx, result) in picker.results.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.add_space(5.0);
                    let rs = ui.label(result);
                    if rs.rect.contains(self.input.pointer_pos) && self.input.clicked_prim {
                        grab_preset = Some(result.clone());
                    }
                    if idx == picker.sel || rs.hovered() {
                        ui.painter().add(Shape::rect_stroke(
                            rs.rect,
                            Rounding::none(),
                            Stroke::new(1.0, Color32::from_gray(200)),
                        ));
                    }
                });
            }
        });
        picker.first_frame = false;
        picker.width = frame_rs.response.rect.width();

        let (pos, field, sel) = (picker.pos, picker.field.clone(), picker.sel);
        // (drop picker borrow)
        if frame_rs.response.rect.contains(self.input.pointer_pos) {
            self.hovered = AppItem::PresetPlacer;
        }
        let mut results = Vec::new();
        for preset in self.presets.get() {
            if preset.name.to_lowercase().contains(&field.to_lowercase()) {
                results.push(preset.name.clone());
            }
            if results.len() >= 6 {
                break;
            }
        }
        if let Some(preset) = grab_preset {
            self.held_presets.push(preset);
        }
        if place_preset && sel < results.len() {
            self.preset_placer = None;
            let pos = self.view.unmap_pos(pos);
            self.place_preset(&results[sel], pos);
            return;
        }
        self.preset_placer.as_mut().unwrap().results = results;
    }

    pub fn misc_debug(&mut self, ui: &mut Ui) {
        ui.label(format!("hovered: {:?}", self.prev_hovered));
        ui.label("write queue: ");
        for write in &self.scene.write_queue.0 {
            ui.horizontal(|ui| {
                ui.add_space(15.0);
                ui.label(format!(
                    "{} - {:?} = {}",
                    write.delay, write.target, write.state
                ));
            });
        }
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _win_frame: &mut eframe::Frame) {
        self.input.update(ctx, self.hovered);

        self.prev_hovered = self.hovered;
        self.hovered = if self.scene.rect.contains(self.input.pointer_pos) {
            AppItem::SceneBackground
        } else {
            AppItem::Other
        };

        if self.input.pressed_s && self.input.cmd && self.paused {
            self.scene.update();
        }
        if self.input.pressed_l && self.input.cmd {
            self.auto_link = !self.auto_link;
        }

        if !self.paused {
            for _ in 0..self.speed {
                self.scene.update();
            }
        }

        ctx.set_visuals(self.settings.visuals());

        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| self.top_panel(ui));
            if self.debug_misc {
                self.misc_debug(ui);
            }
        });
        TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| self.bottom_panel(ui));
        });

        let rs = CentralPanel::default()
            .show(ctx, |ui| self.central_panel(ui))
            .response;

        if let Some((delta, item)) = self.input.drag_delta() {
            match item {
                AppItem::SceneBackground => {
                    self.view.drag(delta);
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
                        self.selected_devices.clear();
                    }
                }
                _ => {}
            }
        }

        let zoom_delta = ctx.input().zoom_delta();
        if zoom_delta != 1.0 {
            let pos = self.input.pointer_pos - self.scene.rect.min;
            self.view.zoom(zoom_delta, pos.to_pos2());
        }

        if self.input.pressed_prim
            && matches!(
                self.prev_hovered,
                AppItem::SceneBackground | AppItem::SceneItem(_)
            )
        {
            // PLACE HELD PRESETS
            if self.held_presets.len() > 0 {
                let mut held_presets = Vec::new();
                std::mem::swap(&mut held_presets, &mut self.held_presets);

                let mut pos = self
                    .view
                    .unmap_pos(self.input.pointer_pos + Vec2::new(0.0, 10.0));
                for name in held_presets {
                    self.place_preset(&name, pos);

                    let preset = self.presets.get_preset(&name).unwrap();
                    pos.y += preset.size(&self.settings).y;
                }
            }
        }

        let can_place_device = match self.prev_hovered {
            AppItem::SceneBackground => true,
            AppItem::SceneItem(SceneItem::Device(..)) => true,
            AppItem::SceneItem(SceneItem::DeviceInput(..)) => true,
            AppItem::SceneItem(SceneItem::DeviceOutput(..)) => true,
            AppItem::SceneItem(SceneItem::DeviceOutputLink(..)) => true,
            AppItem::SceneItem(SceneItem::InputLink(..)) => true,
            _ => false,
        };
        if can_place_device {
            if ctx.input().pointer.secondary_clicked() {
                self.context_menu_pos = self.input.pointer_pos;
            }
            // CONTEXT MENU (PLACE DEVICES)
            if ctx.input().key_pressed(Key::Space) {
                self.preset_placer = Some(PresetPlacer::new(self.input.pointer_pos));
            }
            rs.context_menu(|ui| {
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

                if self.settings.dev_options {
                    if ui.button("debug").clicked() {
                        println!("{:#?}", self.scene);
                    }
                }
                if let Some(name) = place_preset {
                    self.place_preset(&name, self.view.unmap_pos(self.context_menu_pos));
                }
            });
        }

        let since_last_save = SystemTime::now().duration_since(self.last_save).unwrap();
        if since_last_save.as_secs() > 30 {
            settings::save_settings(&self.settings).log_err();
            settings::save_presets(&mut self.presets).log_err();
            settings::save_scene(&self.scene).log_err();
            self.last_save = SystemTime::now();
        }

        if self.input.pressed_prim && self.prev_hovered == AppItem::SceneBackground {
            // PLACE INPUTS/OUTPUTS
            let col_w = self.settings.scene_pin_col_w;
            let output_col_x = self.scene.rect.max.x - col_w;
            let input_col_x = self.scene.rect.min.x + col_w;
            let x = self.input.pointer_pos.x;
            let y = graphics::unmap_io_y(&self.view, self.input.pointer_pos.y);

            if x < input_col_x {
                self.scene.add_input(y);
                self.input.press_pos = Pos2::ZERO;
            } else if x > output_col_x {
                self.scene.add_output(y);
                self.input.press_pos = Pos2::ZERO;
            }
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(1000 / 60))
    }

    fn on_exit(&mut self, _ctx: Option<&eframe::glow::Context>) {
        settings::save_settings(&self.settings).log_err();
        settings::save_presets(&mut self.presets).log_err();
        settings::save_scene(&self.scene).log_err();
    }
}
