use crate::graphics::{Graphics, SceneItem, View};
use crate::preset::{ChipPreset, CombGatePreset, DevicePreset, PresetData, PresetSource};
use crate::settings::Settings;
use crate::*;
use eframe::egui::*;

struct CreatePreset {
    name: String,
    color: Color32,
    cat: u64,
    new_cat_name: String,
    combinational: bool,
}
impl CreatePreset {
    pub fn default() -> Self {
        Self {
            name: String::from("New Chip"),
            color: Color32::from_rgb(255, 255, 255),
            cat: 0, // cat ID 0 should always be the 'Basic' cat
            new_cat_name: String::new(),
            combinational: false,
        }
    }
}

#[derive(Clone)]
struct PresetPicker {
    pub pos: Pos2,
    pub minimized: bool,
    pub cat: Option<u64>,
}
impl PresetPicker {
    pub fn new(pos: Pos2) -> Self {
        Self {
            pos,
            minimized: false,
            cat: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AppItem {
    SceneItem(SceneItem),
    SceneBackground,
    EditPopup,
    PresetPicker,
    Other,
}

#[derive(Clone)]
struct Drag {
    /// The item we were hovering when we pressed
    item: AppItem,
    /// Where the pointer was last frame
    pos: Pos2,
}

pub struct App {
    settings: Settings,
    presets: preset::Presets,

    scene: scene::Scene,
    view: View,
    paused: bool,
    speed: u32,

    /// Where the pointer is this frame
    pointer_pos: Pos2,
    /// What App item is the pointer over this frame
    hovered: AppItem,
    // If we've moved the mouse while pressed
    drag: Option<Drag>,
    /// If we've pressed down the mouse and haven't released it, where
    pressed: Option<Pos2>,
    /// If we right click on some scene item, shows a popup
    edit_popup: Option<SceneItem>,
    /// If we started creating a link, where
    link_start: Option<LinkStart<u64>>,
    /// The config for creating a new preset from the scene
    create_preset: CreatePreset,
    /// A small window for picking presets to place
    preset_picker: PresetPicker,
    /// A list of the presets we've picked from the preset picker
    held_presets: Vec<(u64, u64)>,
}
impl App {
    pub fn new() -> Self {
        let settings = settings::load_settings().unwrap_or(Settings::default());
        let presets = settings::load_presets().unwrap_or(preset::Presets::default());
        let scene = scene::Scene::new();
        let preset_picker = PresetPicker::new(settings.preset_picker_pos.into());

        Self {
            settings,
            presets,

            scene,
            view: View::default(),
            paused: false,
            speed: 1,

            pointer_pos: Pos2::ZERO,
            hovered: AppItem::Other,
            drag: None,
            pressed: None,
            edit_popup: None,

            link_start: None,
            create_preset: CreatePreset::default(),
            preset_picker,
            held_presets: Vec::new(),
        }
    }

    pub fn create(&mut self) {
        let mut create_preset = CreatePreset::default();
        create_preset.cat = self.create_preset.cat;
        std::mem::swap(&mut create_preset, &mut self.create_preset);

        let CreatePreset {
            name,
            color,
            cat,
            combinational,
            ..
        } = create_preset;

        let preset_data = if combinational {
            match CombGatePreset::from_scene(&mut self.scene) {
                Ok(v) => PresetData::CombGate(v),
                Err(e) => {
                    eprintln!("can't create combination gate: {}", e);
                    return;
                }
            }
        } else {
            let chip = ChipPreset::from_scene(&self.scene);
            PresetData::Chip(chip)
        };
        let preset = DevicePreset {
            data: preset_data,
            name,
            color: color.to_array(),
            src: preset::PresetSource::Scene(Some(self.scene.layout())),
        };

        self.presets.mut_cat(cat).unwrap().add_preset(preset);
        self.scene = scene::Scene::new();

        settings::save_presets(&self.presets);
    }

    pub fn place_preset(&mut self, cat_id: u64, id: u64, pos: Pos2) {
        let preset = self
            .presets
            .get_cat(cat_id)
            .unwrap()
            .get_preset(id)
            .unwrap();

        let name = preset.name.clone();
        let [r, g, b, a] = preset.color;
        let color = Color32::from_rgba_premultiplied(r, g, b, a);

        let inputs = preset.data.inputs().to_vec();
        let outputs = preset.data.outputs().to_vec();

        let data = scene::DeviceData::from_preset(&preset.data);
        let device = scene::Device::new(pos, data, name, color, inputs, outputs);

        self.scene.add_device(rand_id(), device);
    }

    // -----------------------------------------------------------
    // GUI

    pub fn show_settings(&mut self, _ctx: &Context, ui: &mut Ui) {
        fn slider<T: eframe::emath::Numeric>(
            ui: &mut Ui,
            label: &str,
            value: &mut T,
            range: std::ops::RangeInclusive<T>,
        ) {
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.label(label);
                ui.add(Slider::new(value, range));
            });
        }
        fn checkbox(ui: &mut Ui, label: &str, value: &mut bool) {
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.label(label);
                ui.checkbox(value, "");
            });
        }

        ui.horizontal(|ui| {
            ui.heading("Settings");
            if ui.button("reload").clicked() {
                if let Some(settings) = settings::load_settings() {
                    self.settings = settings;
                }
            }
            if ui.button("reset").clicked() {
                self.settings = Settings::default();
            }
        });
        ui.separator();

        let Settings {
            dark_mode,
            high_contrast,
            dev_options,

            scene_pin_col_w,
            scene_pin_size,
            link_width,

            device_name_font_size,
            device_pin_size,
            device_min_pin_spacing,

            show_device_id,
            show_write_queue,
            ..
        } = &mut self.settings;

        ui.heading("App");
        checkbox(ui, "dark mode: ", dark_mode);
        checkbox(ui, "high contrast: ", high_contrast);

        ui.heading("Scene IO");
        slider(ui, "column width: ", scene_pin_col_w, 1.0..=100.0);
        slider(ui, "pin width", &mut scene_pin_size[0], 1.0..=100.0);
        slider(ui, "pin height", &mut scene_pin_size[1], 1.0..=100.0);
        slider(ui, "link width", link_width, 1.0..=20.0);

        ui.heading("Devices");
        slider(ui, "name size: ", device_name_font_size, 1.0..=100.0);
        slider(ui, "pin width", &mut device_pin_size[0], 1.0..=100.0);
        slider(ui, "pin height", &mut device_pin_size[1], 1.0..=100.0);
        slider(ui, "min pin spacing: ", device_min_pin_spacing, 1.0..=100.0);

        if *dev_options {
            ui.heading("Debug");
            checkbox(ui, "show device IDs: ", show_device_id);
            checkbox(ui, "show write queue: ", show_write_queue);
        }
    }

    pub fn show_debug(&mut self, _ctx: &Context, ui: &mut Ui) {
        ui.heading("Debug");
        ui.separator();

        let mut string = String::new();

        if ui.button("scene").clicked() {
            string.push_str(&format!("{:#?}", self.scene));
        }
        for (_, cat) in &self.presets.cats {
            if ui.button(format!("presets: {}", cat.name)).clicked() {
                for (_, preset) in &cat.presets {
                    string.push_str(&format!(
                        "{}.{} = {:#?}\n",
                        cat.name, preset.name, preset.data
                    ));
                }
            }
        }
        if ui.button("open config dir").clicked() {
            if let Some(dir) = settings::config_dir() {
                let path = dir.to_str().unwrap();
                if let Err(err) = settings::reveal_dir(path) {
                    eprintln!("failed to open config dir {:?}: {:?}", path, err);
                }
            } else {
                eprintln!("no config dir!");
            }
        }

        if !string.is_empty() {
            settings::save_config("debug_info", string.as_bytes()).unwrap();
        }
    }

    pub fn show_presets(&mut self, ctx: &Context, ui: &mut Ui) {
        let pressed_del = ctx.input().key_pressed(Key::Backspace);

        ui.horizontal(|ui| {
            ui.heading("Preset Settings");
            if ui.button("reload").clicked() {
                if let Some(presets) = settings::load_presets() {
                    self.presets = presets;
                }
            }
            if ui.button("import").clicked() {
                if let Some(file) = rfd::FileDialog::new().set_directory("/").pick_file() {
                    let bytes = std::fs::read(file).unwrap();
                    let presets = settings::decode_presets(&bytes).unwrap();
                    self.presets.merge(&presets);
                }
            }
        });
        ui.separator();

        let mut del_cat = None;
        let mut del_preset = None;
        let mut load_preset = None;
        for (cat_id, cat) in &self.presets.cats {
            let rs = ui.collapsing(&cat.name, |ui| {
                for (preset_id, preset) in &cat.presets {
                    let rs = ui.button(&preset.name);
                    if rs.hovered() && pressed_del {
                        del_preset = Some((*cat_id, *preset_id));
                    }
                    if rs.clicked() {
                        load_preset = Some((*cat_id, *preset_id));
                    }
                }
            });
            if rs.header_response.hovered() && pressed_del {
                del_cat = Some(*cat_id);
            }
        }
        if let Some(id) = del_cat {
            self.presets.remove_cat(id);
        }
        if let Some((cat_id, preset_id)) = del_preset {
            self.presets
                .mut_cat(cat_id)
                .unwrap()
                .remove_preset(preset_id);
        }
        if let Some((cat_id, preset_id)) = load_preset {
            let preset = self
                .presets
                .get_cat(cat_id)
                .unwrap()
                .get_preset(preset_id)
                .unwrap()
                .clone();
            let PresetSource::Scene(Some(layout)) = preset.src.clone() else {
            	eprintln!("this preset doesn't have a scene source!");
            	return;
            };
            let [r, g, b, a] = preset.color;
            self.scene.load_layout(layout);
            self.create_preset.name = preset.name;
            self.create_preset.color = Color32::from_rgba_premultiplied(r, g, b, a);
            self.create_preset.cat = cat_id;
        }
    }

    pub fn show_cat_picker(&mut self, ctx: &Context, ui: &mut Ui) {
        ui.add_space(10.0);

        const LEFT_SP: f32 = 15.0;

        let mut del_cat = None;
        let mut choose_cat = None;

        for (cat_id, cat) in &self.presets.cats {
            ui.horizontal(|ui| {
                ui.add_space(LEFT_SP);
                let cat_button = ui.button(&cat.name);

                if cat_button.clicked() {
                    choose_cat = Some(*cat_id);
                    ui.close_menu();
                }
                if ctx.input().key_pressed(Key::Backspace) && cat_button.hovered() {
                    del_cat = Some(*cat_id);
                }
            });
        }

        if let Some(cat_id) = del_cat {
            if self.presets.remove_cat(cat_id) && self.create_preset.cat == cat_id {
                self.create_preset.cat = self.presets.cats.iter().next().unwrap().0;
            }
        }
        if let Some(id) = choose_cat {
            self.create_preset.cat = id;
        }

        ui.separator();
        ui.horizontal(|ui| {
            ui.add_space(LEFT_SP);

            ui.text_edit_singleline(&mut self.create_preset.new_cat_name);

            let add_button = ui.button("+");
            if add_button.clicked() {
                if let Some(cat_id) = self
                    .presets
                    .add_cat(self.create_preset.new_cat_name.clone())
                {
                    self.create_preset.new_cat_name = String::new();
                    self.create_preset.cat = cat_id;
                }
            }
            add_button.on_hover_text("New Preset Category");
            ui.add_space(5.0);
        });
        ui.add_space(5.0);
    }

    pub fn show_top_panel(&mut self, ctx: &Context, ui: &mut Ui) {
        ui.group(|ui| {
            if self.settings.dev_options {
                ui.menu_button("Debug", |ui| {
                    self.show_debug(ctx, ui);
                });
            }
            ui.menu_button("Settings", |ui| {
                *ui.visuals_mut() = self.settings.visuals();
                self.show_settings(ctx, ui);
            });
            ui.menu_button("Presets", |ui| {
                *ui.visuals_mut() = self.settings.visuals();
                self.show_presets(ctx, ui);
            });

            if ui.button("Create").clicked() {
                self.create();
            }
        });

        let cat_id = self.create_preset.cat;
        let cat_name = self.presets.get_cat(cat_id).unwrap().name.clone();

        ui.color_edit_button_srgba(&mut self.create_preset.color);

        ui.menu_button(cat_name, |ui| {
            *ui.visuals_mut() = self.settings.visuals();
            self.show_cat_picker(ctx, ui);
        });

        ui.text_edit_singleline(&mut self.create_preset.name);

        ui.checkbox(&mut self.create_preset.combinational, "Combinational");
    }

    pub fn show_bottom_panel(&mut self, _ctx: &Context, ui: &mut Ui) {
        // ui.menu_button(format!("speed: {}", self.speed), |ui| {
        //     ui.horizontal(|ui| {
        //         if ui.button("+").clicked() {
        //             self.speed += 1;
        //         }
        //         if ui.button("-").clicked() && self.speed > 0 {
        //             self.speed -= 1;
        //         }
        //     });
        // });

        ui.group(|ui| {
            let pause_button_label = if self.paused { "Unpause" } else { "Pause" };
            if ui.button(pause_button_label).clicked() {
                self.paused = !self.paused;
            }
            if ui.add_enabled(self.paused, Button::new("Step")).clicked() {
                self.scene.update();
            }
            ui.add(Slider::new(&mut self.speed, 1..=999));
        });

        if ui.button("Clear").clicked() {
            self.scene = scene::Scene::new();
        }
    }

    pub fn show_central_panel(&mut self, ctx: &Context, ui: &mut Ui) {
        let (painter_rs, painter) = ui.allocate_painter(ui.available_size(), Sense::hover());
        self.scene.rect = painter_rs.rect;
        self.view.origin = self.scene.rect.min;

        let mut g = Graphics::new(ui, painter_rs.rect, self.pointer_pos);

        self.show_scene(ctx, &mut g);
        painter.extend(g.shapes.drain(..));

        let _pressed_num: Option<usize> = match () {
            _ if ctx.input().key_pressed(Key::Num0) => Some(0),
            _ if ctx.input().key_pressed(Key::Num1) => Some(1),
            _ if ctx.input().key_pressed(Key::Num2) => Some(2),
            _ if ctx.input().key_pressed(Key::Num3) => Some(3),
            _ if ctx.input().key_pressed(Key::Num4) => Some(4),
            _ if ctx.input().key_pressed(Key::Num5) => Some(5),
            _ if ctx.input().key_pressed(Key::Num6) => Some(6),
            _ if ctx.input().key_pressed(Key::Num7) => Some(7),
            _ if ctx.input().key_pressed(Key::Num8) => Some(8),
            _ if ctx.input().key_pressed(Key::Num9) => Some(9),
            _ => None,
        };

        // SHOW WRITE QUEUE
        if self.settings.show_write_queue {
            let mut ui = g.ui.child_ui(
                Rect::from_min_size(self.scene.rect.min, Vec2::new(100.0, 400.0)),
                g.ui.layout().clone(),
            );

            for write in &self.scene.write_queue.0 {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.add_space(5.0);
                    let fmt = format!("{} : {:?} = {}", write.delay, write.target, write.state);
                    ui.label(RichText::new(fmt).strong());
                });
            }
        }

        // SHOW LINK TO CURSOR (IF CREATING ONE)
        if let Some(link_start) = &self.link_start {
            if let Some(state) = self.scene.get_link_start(link_start) {
                use graphics::{link_start_pos, show_link};

                if let Some(pos) =
                    link_start_pos(&self.settings, &self.view, &self.scene, *link_start)
                {
                    show_link(&mut g, &self.settings, state, pos, self.pointer_pos);
                } else {
                    self.link_start = None;
                }
            } else {
                self.link_start = None;
            }

            if ctx.input().key_pressed(Key::Escape) {
                self.link_start = None;
            }
        }

        // SHOW HELD PRESETS
        if ctx.input().key_pressed(Key::Escape) {
            self.held_presets.clear();
        }

        if self.held_presets.len() > 1 {
            g.text(
                self.pointer_pos + Vec2::new(30.0, 0.0),
                20.0,
                &format!("{}", self.held_presets.len()),
                Color32::WHITE,
                Align2::LEFT_CENTER,
            );
        }

        let mut pos = self.pointer_pos + Vec2::new(0.0, 10.0);
        for (cat_id, preset_id) in &self.held_presets {
            let cat = self.presets.get_cat(*cat_id).unwrap();
            let preset = cat.get_preset(*preset_id).unwrap();

            graphics::show_device_preset(&mut g, &self.settings, &self.view, pos, preset);
            pos.y += preset.size(&self.settings).y * self.view.scale();
        }

        // SHOW PRESET PICKER
        self.show_preset_picker(ctx, &mut g);

        painter.extend(g.shapes);

        // SHOW EDIT POPUP
        if self.show_edit_popup(ctx, ui).is_none() {
            self.edit_popup = None;
        }
    }

    pub fn show_edit_popup(&mut self, ctx: &Context, ui: &mut Ui) -> Option<()> {
        let Some(item) = &self.edit_popup else {
        	return None
        };
        let col_w = self.settings.scene_pin_col_w;
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
                    ui.label("signed: ");
                    ui.checkbox(&mut group.signed, "");
                });
                ui.horizontal(|ui| {
                    ui.label("display: ");
                    let display = if group.hex { "hex" } else { "decimal" };
                    if ui.button(display).clicked() {
                        group.hex = !group.hex;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("lsb: ");
                    let lsb = if group.lsb_top { "top" } else { "bottom" };
                    if ui.button(lsb).clicked() {
                        group.lsb_top = !group.lsb_top;
                    }
                });
            }
            SceneItem::OutputGroup(id) => {
                let group = self.scene.output_groups.get_mut(&id).unwrap();
                ui.horizontal(|ui| {
                    ui.label("signed: ");
                    ui.checkbox(&mut group.signed, "");
                });
                ui.horizontal(|ui| {
                    ui.label("display: ");
                    let display = if group.hex { "hex" } else { "decimal" };
                    if ui.button(display).clicked() {
                        group.hex = !group.hex;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("lsb: ");
                    let lsb = if group.lsb_top { "top" } else { "bottom" };
                    if ui.button(lsb).clicked() {
                        group.lsb_top = !group.lsb_top;
                    }
                });
            }
            _ => unreachable!(),
        });
        if !rs.response.hovered() && ctx.input().pointer.primary_clicked() {
            self.edit_popup = None;
        }
        if rs.response.hovered() {
            self.hovered = AppItem::EditPopup;
        }
        Some(())
    }

    pub fn show_scene(&mut self, ctx: &Context, g: &mut Graphics) {
        let mut dead_links = Vec::new();
        let scene_hovered =
            graphics::show_scene(g, &self.settings, &self.view, &self.scene, &mut dead_links);

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

            let pressed = ctx.input().pointer.primary_clicked();
            let secondary_pressed = ctx.input().pointer.secondary_clicked();
            let clicked = match self.pressed {
                None => false,
                Some(pos) => ctx.input().pointer.any_released() && pos == self.pointer_pos,
            };
            let pressed_del = ctx.input().key_pressed(Key::Backspace);
            let pressed_down = ctx.input().key_pressed(Key::ArrowDown);

            match item {
                SceneItem::Device(id) => {
                    if pressed_del {
                        self.scene.del_device(id);
                    }
                }
                SceneItem::InputBulb(id) => {
                    if clicked {
                        let state = self.scene.inputs.get(&id).unwrap().state;
                        self.scene.set_input(id, !state);
                    }
                    if secondary_pressed {
                        self.edit_popup = Some(item);
                    }
                    if pressed_del {
                        self.scene.del_input(id);
                    }
                    if pressed_down {
                        self.scene.stack_input(id, &self.settings);
                    }
                }
                SceneItem::InputPin(id) => {
                    if pressed {
                        self.link_start = Some(LinkStart::Input(id));
                    }
                }
                SceneItem::InputLink(input_id, link_idx) => {
                    if pressed_del {
                        let links = &mut self.scene.inputs.get_mut(&input_id).unwrap().links;
                        let link = links[link_idx].clone();
                        links.remove(link_idx);
                        self.scene.write_queue.push(link.wrap(), false);
                    }
                }
                SceneItem::InputGroup(_) => {
                    if secondary_pressed {
                        self.edit_popup = Some(item);
                    }
                }
                SceneItem::OutputBulb(id) => {
                    if pressed_del {
                        self.scene.del_output(id);
                    }
                    if secondary_pressed {
                        self.edit_popup = Some(item);
                    }
                    if pressed_down {
                        self.scene.stack_output(id, &self.settings);
                    }
                }
                SceneItem::OutputGroup(_) => {
                    if secondary_pressed {
                        self.edit_popup = Some(item);
                    }
                }
                SceneItem::OutputPin(id) => {
                    if pressed {
                        match self.link_start.clone() {
                            Some(LinkStart::Input(_)) => {
                                println!("a scene input can't be linked to a scene output");
                            }
                            Some(LinkStart::DeviceOutput(device, output)) => {
                                self.scene.add_link(NewLink::DeviceOutputTo(
                                    device,
                                    output,
                                    LinkTarget::Output(id),
                                ));
                                self.link_start = None;
                            }
                            None => {}
                        }
                    }
                }
                SceneItem::DeviceInput(device, input) => {
                    if pressed {
                        match self.link_start.clone() {
                            Some(LinkStart::Input(from_input)) => {
                                self.scene.add_link(NewLink::InputToDeviceInput(
                                    from_input,
                                    DeviceInput(device, input),
                                ));
                                self.link_start = None;
                            }
                            Some(LinkStart::DeviceOutput(from_device, from_output)) => {
                                self.scene.add_link(NewLink::DeviceOutputTo(
                                    from_device,
                                    from_output,
                                    LinkTarget::DeviceInput(device, input),
                                ));
                                self.link_start = None;
                            }
                            None => {
                                let state = self.scene.get_device_input(device, input).unwrap();
                                self.scene.set_device_input(device, input, !state);
                            }
                        }
                    }
                }
                SceneItem::DeviceOutput(device, output) => {
                    if pressed {
                        self.link_start = Some(LinkStart::DeviceOutput(device, output));
                    }
                }
                SceneItem::DeviceOutputLink(device_id, output_idx, link_idx) => {
                    if pressed_del {
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

    pub fn show_preset_picker(&mut self, _ctx: &Context, g: &mut Graphics) {
        let title = if let Some(cat) = self.preset_picker.cat {
            self.presets.get_cat(cat).unwrap().name.clone()
        } else {
            String::from("Presets")
        };
        let header_size = Vec2::new(200.0, 20.0);

        // clamp position
        let pos = &mut self.preset_picker.pos;
        pos.x = f32::max(pos.x, self.scene.rect.min.x);
        pos.x = f32::min(pos.x, self.scene.rect.max.x);
        pos.y = f32::max(pos.y, self.scene.rect.min.y);
        pos.y = f32::min(pos.y, self.scene.rect.max.y);
        self.settings.preset_picker_pos = (*pos).into();
        let header_rect = Rect::from_min_size(*pos, header_size);

        // show picker
        let mut child_ui = g.ui.child_ui(header_rect, g.ui.layout().clone());
        let frame_rs = Frame::menu(child_ui.style()).show(&mut child_ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(5.0);
                ui.label(title);
                ui.add_space(5.0);
            });
            if self.preset_picker.minimized {
                return;
            }
            let style = ui.style_mut();
            style.spacing.button_padding = Vec2::new(2.0, 0.0);
            style.visuals.widgets.active.bg_stroke = Stroke::none();
            style.visuals.widgets.hovered.bg_stroke = Stroke::none();
            style.visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
            style.visuals.widgets.inactive.bg_stroke = Stroke::none();

            if let Some(cat_id) = self.preset_picker.cat {
                let cat = self.presets.get_cat(cat_id).unwrap();

                if ui.button("0 : back").clicked() {
                    self.preset_picker.cat = None;
                }

                let mut picked_preset = None;
                for (i, (preset_id, preset)) in cat.presets.iter().enumerate() {
                    let rs = ui.button(format!("{} : {}", i + 1, preset.name));
                    if rs.clicked() {
                        picked_preset = Some(*preset_id);
                    }
                }
                if let Some(id) = picked_preset {
                    self.held_presets.push((cat_id, id));
                }
            } else {
                for (i, (cat_id, cat)) in self.presets.cats.iter().enumerate() {
                    let rs = ui.button(format!("{} : {}", i + 1, cat.name));
                    if rs.clicked() {
                        self.preset_picker.cat = Some(*cat_id);
                    }
                }
            }
        });

        let header_rs = g.ui.interact(
            header_rect,
            Id::new("preset_picker"),
            Sense::click_and_drag(),
        );
        if header_rs.clicked() {
            self.preset_picker.minimized = !self.preset_picker.minimized;
        }
        self.preset_picker.pos += header_rs.drag_delta();
        if frame_rs.response.rect.contains(self.pointer_pos) {
            self.hovered = AppItem::PresetPicker;
        }
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _win_frame: &mut eframe::Frame) {
        if let Some(pos) = ctx.input().pointer.interact_pos() {
            self.pointer_pos = pos;
        }
        let clicked = ctx.input().pointer.primary_clicked();

        if self.pointer_pos.x >= self.scene.rect.min.x
            && self.pointer_pos.x <= self.scene.rect.max.x
            && self.pointer_pos.y >= self.scene.rect.min.y
            && self.pointer_pos.y <= self.scene.rect.max.y
        {
            self.hovered = AppItem::SceneBackground;
        } else {
            self.hovered = AppItem::Other;
        }

        if ctx.input().key_pressed(Key::S) && self.paused {
            self.scene.update();
        }

        if !self.paused {
            for _ in 0..self.speed {
                self.scene.update();
            }
        }

        ctx.set_visuals(self.settings.visuals());

        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| self.show_top_panel(ctx, ui));
        });

        TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| self.show_bottom_panel(ctx, ui));
        });

        let rs = CentralPanel::default().show(ctx, |ui| self.show_central_panel(ctx, ui));
        let rs = rs.response.interact(Sense::hover());

        if clicked {
            self.pressed = Some(self.pointer_pos);
        }
        if let Some(pos) = self.pressed && pos != self.pointer_pos && self.drag.is_none() {
        	self.drag = Some(Drag {
                item: self.hovered,
                pos: self.pointer_pos,
            });
        }
        if ctx.input().pointer.any_released() {
            self.drag = None;
            self.pressed = None;
        }

        if let Some(Drag { item, pos }) = self.drag.clone() {
            let delta = self.pointer_pos - pos;
            self.drag = Some(Drag {
                item,
                pos: self.pointer_pos,
            });
            match item {
                AppItem::SceneBackground => {
                    self.view.offset += delta;
                }
                AppItem::SceneItem(SceneItem::InputBulb(id)) => {
                    self.scene.drag_input(id, delta);
                }
                AppItem::SceneItem(SceneItem::OutputBulb(id)) => {
                    self.scene.drag_output(id, delta);
                }
                AppItem::SceneItem(SceneItem::Device(id)) => {
                    self.scene.drag_device(id, delta * self.view.inv_scale());
                }
                _ => {}
            }
        }

        let zoom_delta = ctx.input().zoom_delta();
        if zoom_delta != 1.0 {
            let prev_scale = self.view.scale();
            self.view.zoom *= zoom_delta;

            let scale_change = self.view.scale() - prev_scale;

            let zoom_point = self.pointer_pos - self.scene.rect.min;

            self.view.offset.x -= zoom_point.x * scale_change;
            self.view.offset.y -= zoom_point.y * scale_change;
        }

        if clicked && self.hovered == AppItem::SceneBackground {
            // PLACE INPUTS/OUTPUTS
            let col_w = self.settings.scene_pin_col_w;
            let output_col_x = self.scene.rect.max.x - col_w;
            let input_col_x = self.scene.rect.min.x + col_w;
            let x = self.pointer_pos.x;
            let y = graphics::unmap_io_y(&self.view, self.pointer_pos.y);

            if x < input_col_x {
                self.pressed = None;
                self.scene.add_input(y);
            } else if x > output_col_x {
                self.pressed = None;
                self.scene.add_output(y);
            } else
            // PLACE HELD PRESETS
            if self.held_presets.len() > 0 {
                self.pressed = None;
                let mut held_presets = Vec::new();
                std::mem::swap(&mut held_presets, &mut self.held_presets);

                let mut pos = self.view.unmap_pos(self.pointer_pos + Vec2::new(0.0, 10.0));
                for (cat_id, preset_id) in held_presets {
                    self.place_preset(cat_id, preset_id, pos);

                    let cat = self.presets.get_cat(cat_id).unwrap();
                    let preset = cat.get_preset(preset_id).unwrap();
                    pos.y += preset.size(&self.settings).y;
                }
            }
        }

        if self.hovered == AppItem::SceneBackground {
            // CONTEXT MENU (PLACE DEVICES)
            rs.context_menu(|ui| {
                ui.set_width(100.0);
                const LEFT_SP: f32 = 15.0;

                let mut place_preset = None;

                for (cat_id, cat) in &self.presets.cats {
                    ui.menu_button(&cat.name, |ui| {
                        ui.set_width(100.0);

                        for (preset_id, preset) in &cat.presets {
                            let button = Button::new(&preset.name).ui(ui);

                            if button.clicked() {
                                place_preset = Some((*cat_id, *preset_id));
                                ui.close_menu();
                            }
                        }
                    });
                }

                if let Some((cat_id, id)) = place_preset {
                    self.place_preset(cat_id, id, self.view.unmap_pos(self.pointer_pos));
                }
            });
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(1000 / 60))
    }

    fn on_exit(&mut self, _glow: Option<&eframe::glow::Context>) {
        settings::save_settings(&self.settings);
        settings::save_presets(&self.presets);
    }
}
