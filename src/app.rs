use crate::graphics::{Graphics, SceneItem, View};
use crate::preset::{ChipPreset, DevicePreset, PresetData};
use crate::settings::Settings;
use crate::*;
use eframe::egui::*;

struct CreatePreset {
    name: String,
    color: Color,
    cat: IntId,
    new_cat_name: String,
    combinational: bool,
}
impl CreatePreset {
    pub fn default() -> Self {
        Self {
            name: String::from("New Chip"),
            color: Color::from_rgb(255, 255, 255),
            cat: IntId(0), // cat ID 0 should always be the 'Basic' cat
            new_cat_name: String::new(),
            combinational: false,
        }
    }
}

#[derive(Clone)]
struct PresetPicker {
    pub pos: Pos2,
    pub response_rect: Rect,
    pub minimized: bool,
    pub cat: Option<IntId>,
}
impl PresetPicker {
    pub fn new(pos: Pos2) -> Self {
        Self {
            pos,
            response_rect: Rect::from_min_max(Pos2::ZERO, Pos2::ZERO),
            minimized: false,
            cat: None,
        }
    }
}

#[derive(Clone)]
struct EditIo {
    id: IntId,
}
impl EditIo {
    fn new(id: IntId) -> Self {
        Self { id }
    }
}

pub struct App {
    settings: Settings,
    presets: preset::Presets,

    scene: scene::Scene,
    view: View,
    paused: bool,
    speed: u32,

    hovered: Option<SceneItem>,
    scene_hovered: bool,
    pointer_pos: Pos2,

    link_start: Option<LinkStart<IntId>>,
    create_preset: CreatePreset,
    preset_picker: PresetPicker,
    held_presets: Vec<(IntId, IntId)>,
    edit_input: Option<EditIo>,
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

            hovered: None,
            pointer_pos: Pos2::ZERO,
            scene_hovered: false,

            link_start: None,
            create_preset: CreatePreset::default(),
            preset_picker,
            held_presets: Vec::new(),
            edit_input: None,
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

        if combinational {
            println!(
                "\
                    The 'combinational' flag is currently not used.\n\
                    In the future, creating a preset for a combinational chip would create a truth table \n\
                    to optimize performance of using such chips.
                "
            );
        }

        let chip = ChipPreset::from_scene(&self.scene);
        let preset = DevicePreset {
            data: PresetData::Chip(chip),
            name,
            color: color.to_array(),
            src: preset::PresetSource::BuiltIn, // TODO PresetSource::Scene
        };

        self.presets.mut_cat(cat).unwrap().add_preset(preset);
        self.scene = scene::Scene::new();

        settings::save_presets(&self.presets);
    }

    pub fn place_preset(&mut self, cat_id: IntId, id: IntId, pos: Pos2) {
        let preset = self
            .presets
            .get_cat(cat_id)
            .unwrap()
            .get_preset(id)
            .unwrap();

        let name = preset.name.clone();
        let [r, g, b, a] = preset.color;
        let color = Color::from_rgba_premultiplied(r, g, b, a);

        let inputs = preset.data.inputs().to_vec();
        let outputs = preset.data.outputs().to_vec();

        let data = scene::DeviceData::from_preset(&preset.data);
        let device = scene::Device::new(pos, data, name, color, inputs, outputs);

        self.scene.add_device(IntId::new(), device);
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

            scene_pin_col_w,
            scene_pin_size,

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

        ui.heading("Devices");
        slider(ui, "name size: ", device_name_font_size, 1.0..=100.0);
        slider(ui, "pin width", &mut device_pin_size[0], 1.0..=100.0);
        slider(ui, "pin height", &mut device_pin_size[1], 1.0..=100.0);
        slider(ui, "min pin spacing: ", device_min_pin_spacing, 1.0..=100.0);

        ui.heading("Debug");
        checkbox(ui, "show device IDs: ", show_device_id);
        checkbox(ui, "show write queue: ", show_write_queue);
    }

    pub fn show_debug(&mut self, _ctx: &Context, ui: &mut Ui) {
        ui.heading("Debug");
        ui.separator();

        #[inline(always)]
        fn debug<T: std::fmt::Debug>(ui: &mut Ui, name: &str, t: &T) {
            if ui.button(name).clicked() {
                println!("{} = {:#?}", name, t);
            }
        }
        debug(ui, "scene.write_queue", &self.scene.write_queue);
        debug(ui, "scene.devices", &self.scene.devices);
        debug(ui, "scene.inputs", &self.scene.inputs);
        debug(ui, "scene.outputs", &self.scene.outputs);
        debug(ui, "presets", &self.presets);

        debug(ui, "config dir", &settings::config_dir());
        if ui.button("open config dir").clicked() {
            if let Some(dir) = settings::config_dir() {
                let _ = settings::reveal_dir(dir.to_str().unwrap());
            }
        }
    }

    pub fn show_presets(&mut self, _ctx: &Context, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("Preset settings");
            if ui.button("save").clicked() {
                settings::save_presets(&self.presets);
            }
            if ui.button("load").clicked() {
                if let Some(presets) = settings::load_presets() {
                    self.presets = presets;
                }
            }
            if ui.button("merge").clicked() {
                if let Some(file) = rfd::FileDialog::new().set_directory("/").pick_file() {
                    let bytes = std::fs::read(file).unwrap();
                    let presets = settings::decode_presets(&bytes).unwrap();
                    self.presets.merge(&presets);
                }
            }
        });
        ui.separator();
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
            ui.menu_button("Debug", |ui| {
                self.show_debug(ctx, ui);
            });
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

        // Frame::none().fill(Color::GREEN).show(ui, |ui| {
        //     self.show_scene(ctx, ui);
        // });

        self.show_scene(ctx, &mut g);
        painter.extend(g.shapes.drain(..).collect());

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

        // DRAW LINK TO CURSOR (IF CREATING ONE)
        if let Some(link_start) = &self.link_start {
            if let Some((pin, state)) =
                self.scene
                    .get_link_start(link_start, &self.settings, &self.view)
            {
                graphics::show_link(
                    &mut g,
                    &self.settings,
                    state,
                    pin.tip(&self.settings),
                    self.pointer_pos,
                );
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
                Color::WHITE,
                Align2::LEFT_CENTER,
            );
        }

        let mut pos = self.pointer_pos + Vec2::new(0.0, 10.0);
        for (cat_id, preset_id) in &self.held_presets {
            let cat = self.presets.get_cat(*cat_id).unwrap();
            let preset = cat.get_preset(*preset_id).unwrap();

            graphics::show_device_preset(&mut g, &self.settings, &self.view, pos, preset);
            pos.y += preset.size(&self.settings).y;
        }

        // SHOW INPUT EDITOR
        if let Some(EditIo { id }) = self.edit_input.clone() {
            if let Some(input) = self.scene.inputs.get(&id) {
                let pin_rect = self
                    .scene
                    .input_pin(input, &self.settings, &self.view)
                    .rect(&self.settings);

                let rect = Rect::from_min_size(pin_rect.right_top(), Vec2::new(100.0, 30.0));

                // g.shapes.push(Shape::rect_stroke(
                //     rect,
                //     Rounding::same(3.0),
                //     Stroke::new(1.0, Color::from_gray(100)),
                // ));

                let input = self.scene.inputs.get_mut(&id).unwrap();
                let mut child_ui = g.ui.child_ui(rect, g.ui.layout().clone());
                let rs = Frame::menu(child_ui.style())
                    .fill(Color::BLACK)
                    .show(&mut child_ui, |ui| {
                        // child_ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            ui.label("name: ");
                            ui.text_edit_singleline(&mut input.name);
                        });
                    })
                    .response;

                self.edit_input = Some(EditIo { id });

                if ctx.input().pointer.primary_clicked() && !rs.hovered {
                    self.edit_input = None;
                }
            } else {
                self.edit_input = None;
            }
        }

        self.show_preset_picker(ctx, &mut g);

        let shapes = g.shapes;
        painter.extend(shapes);
    }

    pub fn show_scene(&mut self, ctx: &Context, g: &mut Graphics) {
        let mut dead_links = Vec::new();
        let scene_rs =
            graphics::show_scene(g, &self.settings, &self.view, &self.scene, &mut dead_links);
        self.hovered = scene_rs.as_ref().map(|(_, item)| item.clone());

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
        if let Some((rs, item)) = scene_rs {
            self.scene_hovered = false;

            match item {
                SceneItem::Device(id) => {
                    if rs.drag_delta() != Vec2::ZERO {
                        self.scene.drag_device(id, rs.drag_delta());
                    }
                    if rs.hovered() && ctx.input().key_pressed(Key::Backspace) {
                        self.scene.del_device(id);
                    }
                }
                SceneItem::InputBulb(id) => {
                    if rs.drag_delta() != Vec2::ZERO {
                        self.scene.drag_input(id, rs.drag_delta());
                    }

                    if rs.clicked() {
                        let state = self.scene.get_input(id).unwrap().state;
                        self.scene.set_input(id, !state);
                    }
                    if rs.hovered() && ctx.input().key_pressed(Key::Backspace) {
                        self.scene.del_input(id);
                    }
                    if rs.secondary_clicked() {
                        self.edit_input = Some(EditIo::new(id));
                    }
                }
                SceneItem::InputPin(id) => {
                    if rs.clicked() {
                        self.link_start = Some(LinkStart::Input(id));
                    }
                }
                SceneItem::OutputBulb(id) => {
                    if rs.drag_delta() != Vec2::ZERO {
                        self.scene.drag_output(id, rs.drag_delta());
                    }
                    if rs.hovered() && ctx.input().key_pressed(Key::Backspace) {
                        self.scene.del_output(id);
                    }
                }
                SceneItem::OutputPin(id) => {
                    if rs.clicked() {
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
                    if rs.clicked() {
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
                    if rs.clicked() {
                        self.link_start = Some(LinkStart::DeviceOutput(device, output));
                    }
                }
                _ => {}
            }
        }
    }

    pub fn show_preset_picker(&mut self, _ctx: &Context, g: &mut Graphics) {
        let title = if let Some(cat) = self.preset_picker.cat {
            self.presets.get_cat(cat).unwrap().name.clone()
        } else {
            String::from("Presets")
        };
        let size = Vec2::new(200.0, 20.0);

        // clamp position
        let pos = &mut self.preset_picker.pos;
        pos.x = f32::max(pos.x, self.scene.rect.min.x);
        pos.x = f32::min(pos.x, self.scene.rect.max.x);
        pos.y = f32::max(pos.y, self.scene.rect.min.y);
        pos.y = f32::min(pos.y, self.scene.rect.max.y);
        self.settings.preset_picker_pos = (*pos).into();
        let mut rect = Rect::from_min_size(*pos, size);

        // show impl
        let mut show = |ui: &mut Ui| {
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
        };

        // show picker
        let mut child_ui = g.ui.child_ui(rect, g.ui.layout().clone());
        let rs = CollapsingHeader::new(title)
            .default_open(!self.preset_picker.minimized)
            .show(&mut child_ui, |ui| {
                let style = ui.style_mut();
                style.visuals.widgets.hovered.bg_stroke = Stroke::none();
                style.visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
                style.spacing.item_spacing = Vec2::ZERO;
                show(ui);
            });

        if let Some(body_rs) = rs.body_response {
            rect.max.y = body_rs.rect.max.y;
            rect.max.x = body_rs.rect.max.x;
        } else {
            rect.max.x = rs.header_response.rect.max.x;
        }

        let rs =
            g.ui.interact(rect, Id::new("preset_picker"), Sense::click_and_drag());
        g.shapes.push(Shape::rect_stroke(
            rect,
            Rounding::same(3.0),
            Stroke::new(1.0, Color::from_gray(100)),
        ));

        // drag picker
        self.preset_picker.response_rect = rect;
        if rect.contains(self.pointer_pos) {
            self.scene_hovered = false;
        }
        self.preset_picker.pos += rs.drag_delta();
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _win_frame: &mut eframe::Frame) {
        if let Some(pos) = ctx.input().pointer.interact_pos() {
            self.pointer_pos = pos;
        }
        let clicked = ctx.input().pointer.primary_clicked();

        self.scene_hovered = self.pointer_pos.x >= self.scene.rect.min.x
            && self.pointer_pos.x <= self.scene.rect.max.x
            && self.pointer_pos.y >= self.scene.rect.min.y
            && self.pointer_pos.y <= self.scene.rect.max.y;

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

        let response = CentralPanel::default()
            .show(ctx, |ui| self.show_central_panel(ctx, ui))
            .response
            .interact(Sense::drag());

        let zoom_delta = ctx.input().zoom_delta();
        if zoom_delta != 1.0 {
            let prev_scale = self.view.scale();
            self.view.zoom *= zoom_delta;
            let scale_change = self.view.scale() - prev_scale;

            let zoom_point = self.pointer_pos - self.scene.rect.min.to_vec2();

            self.view.offset.x -= zoom_point.x * scale_change;
            self.view.offset.y -= zoom_point.y * scale_change;
        }
        if response.drag_delta() != Vec2::ZERO && zoom_delta == 1.0 && self.scene_hovered {
            self.view.offset += response.drag_delta();
        }

        if clicked && self.scene_hovered {
            // PLACE INPUTS/OUTPUTS
            let col_w = self.settings.scene_pin_col_w;
            let output_col_x = self.scene.rect.max.x - col_w;
            let input_col_x = self.scene.rect.min.x + col_w;
            let Pos2 { x, y } = self.pointer_pos;

            if x < input_col_x {
                self.scene.add_input(IntId::new(), scene::Input::new(y));
            } else if x > output_col_x {
                self.scene.add_output(IntId::new(), scene::Output::new(y));
            } else
            // PLACE HELD PRESETS
            if self.held_presets.len() > 0 {
                let mut held_presets = Vec::new();
                std::mem::swap(&mut held_presets, &mut self.held_presets);

                let mut pos = self.pointer_pos + Vec2::new(0.0, 10.0);
                for (cat_id, preset_id) in held_presets {
                    self.place_preset(cat_id, preset_id, pos);

                    let cat = self.presets.get_cat(cat_id).unwrap();
                    let preset = cat.get_preset(preset_id).unwrap();
                    pos.y += preset.size(&self.settings).y;
                }
            }
        }

        // CONTEXT MENU (PLACE DEVICES)
        response.context_menu(|ui| {
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
                self.place_preset(cat_id, id, self.pointer_pos);
            }
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(1000 / 60))
    }

    fn on_exit(&mut self, _glow: Option<&eframe::glow::Context>) {
        settings::save_settings(&self.settings);
        settings::save_presets(&self.presets);
    }
}
