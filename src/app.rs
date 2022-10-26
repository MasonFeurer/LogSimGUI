use crate::*;
use debug::good_debug;
use eframe::egui::*;

struct CreatePreset {
    name: String,
    color: Color,
    cat: IntId,
    new_cat_name: String,
    combinational: bool,
}
impl CreatePreset {
    pub fn new() -> Self {
        Self {
            name: format!("New Chip {}", fastrand::u16(10000..)),
            color: Color::from_rgb(255, 255, 255),
            cat: IntId(0),
            new_cat_name: String::new(),
            combinational: false,
        }
    }
}

#[derive(Clone)]
struct ShortcutPlacer {
    pos: Pos2,
    cat_id: IntId,
}

pub struct App {
    presets: preset::Presets,
    scene: scene::Scene,
    paused: bool,
    speed: u32,

    link_start: Option<LinkStart<IntId>>,
    create_preset: CreatePreset,
    shortcut_placer: Option<ShortcutPlacer>,
    settings: graphics::Settings,
}
impl App {
    pub fn new() -> Self {
        Self {
            presets: preset::Presets::default(),
            scene: scene::Scene::new(),
            paused: false,
            speed: 1,

            link_start: None,
            create_preset: CreatePreset::new(),
            shortcut_placer: None,
            settings: graphics::Settings::default(),
        }
    }

    pub fn create(&mut self) {
        let mut create_preset = CreatePreset::new();
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

        let chip = preset::chip::Chip::from_scene(&self.scene);
        let preset = preset::Preset {
            data: preset::PresetData::Chip(chip),
            vis: DeviceVisuals { name, color },
        };
        self.presets.mut_cat(cat).unwrap().add_preset(preset);
        self.scene = scene::Scene::new();
    }

    pub fn place_preset(&mut self, cat_id: IntId, id: IntId, pos: Pos2) {
        let preset = self
            .presets
            .get_cat(cat_id)
            .unwrap()
            .get_preset(id)
            .unwrap();

        let input = preset.data.inputs();
        let output = preset.data.outputs();

        let data = scene::DeviceData::from_preset(&preset.data);
        let device = scene::Device::new(preset.vis.clone(), data, pos, input, output);

        self.scene.add_device(device);
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _win_frame: &mut eframe::Frame) {
        let pointer_pos = ctx
            .input()
            .pointer
            .interact_pos()
            .unwrap_or(Pos2::new(0.0, 0.0));
        let pressed_del = ctx.input().key_pressed(Key::Backspace);

        if !self.paused {
            for _ in 0..self.speed {
                self.scene.update();
            }
        }

        ctx.set_visuals(Visuals::dark());
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("⚙", |ui| {
                    // the ui given in `menu_button` has it's widgets background be fully transparent, so just undoing that
                    *ui.visuals_mut() = Visuals::dark();

                    ui.heading("UI Settings");
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("io_col_w: ");
                        ui.add(Slider::new(&mut self.settings.io_col_w, 0.0..=100.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("device_name_font_size: ");
                        ui.add(Slider::new(
                            &mut self.settings.device_name_font_size,
                            1.0..=200.0,
                        ));
                    });
                    ui.horizontal(|ui| {
                        ui.label("device_name_hover_text: ");
                        ui.checkbox(&mut self.settings.device_name_hover_text, "");
                    });
                    ui.horizontal(|ui| {
                        ui.label("small_io_size.x: ");
                        ui.add(Slider::new(&mut self.settings.small_io_size.x, 1.0..=100.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("small_io_size.y: ");
                        ui.add(Slider::new(&mut self.settings.small_io_size.y, 1.0..=100.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("large_io_size.x: ");
                        ui.add(Slider::new(&mut self.settings.large_io_size.x, 1.0..=100.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("large_io_size.y: ");
                        ui.add(Slider::new(&mut self.settings.large_io_size.y, 1.0..=100.0));
                    });
                });
                ui.separator();

                if ui.button("Create Preset").clicked() {
                    self.create();
                }
                ui.separator();

                let cat_name = self
                    .presets
                    .get_cat(self.create_preset.cat)
                    .unwrap()
                    .name
                    .to_owned();

                ui.menu_button(format!("category: {cat_name}"), |ui| {
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
                            if pressed_del && cat_button.hovered() {
                                del_cat = Some(*cat_id);
                            }
                        });
                    }

                    if let Some(cat_id) = del_cat && self.presets.cats.len() > 1 {
                        self.presets.remove_cat(cat_id);
                        if self.create_preset.cat == cat_id {
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
                        let new_cat_name_valid = !self.create_preset.new_cat_name.trim().is_empty();

                        let add_button = ui.button("+");
                        if add_button.clicked() && new_cat_name_valid {
                            let cat_id = self
                                .presets
                                .add_cat(self.create_preset.new_cat_name.clone());
                            self.create_preset.new_cat_name = String::new();
                            self.create_preset.cat = cat_id;
                        }
                        add_button.on_hover_text("New Preset Category");
                        ui.add_space(5.0);
                    });
                    ui.add_space(5.0);
                });

                ui.color_edit_button_srgba(&mut self.create_preset.color);
                ui.text_edit_singleline(&mut self.create_preset.name);
                ui.checkbox(&mut self.create_preset.combinational, "combinational");
            });
        });

        TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("sim");
                ui.separator();

                let pause_button_label = if self.paused { "unpause" } else { "pause" };

                let pause_button = ui.button(pause_button_label);
                if pause_button.clicked() {
                    self.paused = !self.paused;
                }

                // TOOD
                // let step_button = ui.add_enabled(self.paused, Button::new("step"));
                let step_button = ui.button("step");
                if step_button.clicked() {
                    self.scene.update();
                }

                ui.add(Slider::new(&mut self.speed, 0..=999));

                if ui.button("clear").clicked() {
                    self.scene = scene::Scene::new();
                }

                if ui.button("debug scene").clicked() {
                    println!("scene: {}\n", good_debug(&self.scene));
                }
                if ui.button("debug presets").clicked() {
                    println!("presets: {}\n", good_debug(&self.presets));
                }

                let home = std::env::var("HOME").unwrap_or("/".to_owned());
                let save_path = format!("{}/logic-sim-presets", home);
                if ui.button("save presets").clicked() {
                    let bytes: Vec<u8> =
                        bincode::serialize(&self.presets, bincode::Bounded(10000)).unwrap();
                    std::fs::write(&save_path, &bytes).unwrap();
                }
                if ui.button("load presets").clicked() {
                    let bytes = std::fs::read(&save_path).unwrap();
                    self.presets = bincode::deserialize(&bytes).unwrap();
                }
            });
        });

        let response = CentralPanel::default()
            .show(ctx, |ui| {
                Frame::canvas(ui.style()).show(ui, |ui| {
                    let (response, painter) = ui.allocate_painter(
                        Vec2::new(ui.available_width(), ui.available_height()),
                        Sense::hover(),
                    );
                    self.scene.rect = response.rect;

                    let mut ctx =
                        graphics::Context::new(ui, &self.settings, response.rect, pointer_pos);

                    // SHOW SCENE
                    let mut dead_links = Vec::new();
                    let scene_int = graphics::show_scene(&mut ctx, &self.scene, &mut dead_links);

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
                                self.scene.devices.get_mut(&device).unwrap().links[output]
                                    .remove(link_idx);
                            }
                        };
                    }

                    // HANDLE SCENE INTERACTIONS
                    use graphics::SceneInteraction as Int;
                    use graphics::SubInteraction as SubInt;
                    match scene_int {
                        None => (),
                        Some(Int::Device(int)) => {
                            let device = self.scene.devices.get_mut(&int.sub).unwrap();

                            if int.int.drag != Vec2::ZERO {
                                device.drag(int.int.drag);
                            }

                            if int.int.hovered && pressed_del {
                                self.scene.devices.remove(&int.sub);
                            }
                        }
                        Some(Int::Input(SubInt { sub: id, int })) => {
                            if int.clicked {
                                let state = self.scene.get_input(id).unwrap();
                                self.scene.set_input(id, !state);
                            } else if int.secondary_clicked {
                                self.link_start = Some(LinkStart::Input(id));
                            }
                            if int.hovered && pressed_del {
                                self.scene.inputs.remove(&id);
                            }
                        }
                        Some(Int::Output(SubInt { sub: id, int })) => {
                            if int.clicked {
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
                            if int.hovered && pressed_del {
                                self.scene.outputs.remove(&id);
                            }
                        }
                        Some(Int::DeviceInput(device, SubInt { sub: input, int })) => {
                            if int.clicked {
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
                                        let state =
                                            self.scene.get_device_input(device, input).unwrap();
                                        self.scene.set_device_input(device, input, !state);
                                    }
                                }
                            }
                        }
                        Some(Int::DeviceOutput(device, SubInt { sub: output, int })) => {
                            if int.clicked {
                                self.link_start = Some(LinkStart::DeviceOutput(device, output));
                            }
                        }
                    }

                    // DRAW LINK TO CURSOR (IF CREATING ONE)
                    if let Some(link_start) = &self.link_start {
                        if let Some((def, state)) = self.scene.get_link_start(link_start) {
                            graphics::show_link(
                                &mut ctx,
                                state,
                                def.tip_loc(&self.settings),
                                pointer_pos,
                            );
                        } else {
                            self.link_start = None;
                        }

                        if pressed_del {
                            self.link_start = None;
                        }
                    }

                    let graphics::Context { shapes, .. } = ctx;

                    painter.extend(shapes);
                });

                let over_scene = self.scene.rect.contains(pointer_pos);

                // HANDLE KEY PRESSES FOR SHORTCUT MENU
                let pressed_num: Option<usize> = match () {
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
                if ctx.input().key_pressed(Key::Escape) {
                    self.shortcut_placer = None;
                }
                if let Some(pressed_num) = pressed_num && over_scene {
                    if let Some(shortcut_placer) = &self.shortcut_placer {
                    	let cat_id = shortcut_placer.cat_id;
                    	let cat = self.presets.get_cat(cat_id).unwrap();

                    	if let Some((preset_id, _)) = cat.presets.get(pressed_num) {
                    		self.place_preset(cat_id, *preset_id, shortcut_placer.pos);
                            self.shortcut_placer = None;
                    	}
                    } else {
                    	if let Some((cat_id, _)) = self.presets.cats.get(pressed_num) {
                    		self.shortcut_placer = Some(ShortcutPlacer {
                                pos: pointer_pos,
                                cat_id: *cat_id,
                            });
                    	}
                    }
                }
                if let Some(ShortcutPlacer { pos, cat_id }) = &self.shortcut_placer {
                    let mut child_ui = ui.child_ui(
                        Rect::from_min_size(*pos, Vec2::new(300.0, 300.0)),
                        ui.layout().clone(),
                    );
                    const SP: f32 = 20.0;
                    child_ui.set_width(80.0);
                    child_ui.separator();

                    let presets = &self.presets.get_cat(*cat_id).unwrap().presets;
                    for (idx, (_, preset)) in presets.iter().enumerate() {
                        child_ui.horizontal(|ui| {
                            ui.add_space(SP);
                            ui.label(format!("{} - {}", idx, preset.vis.name));
                        });
                    }
                    child_ui.separator();
                }
            })
            .response
            .interact(Sense::click_and_drag());

        // PLACE INPUTS/OUTPUTS
        if response.clicked() {
            let io_col_w = self.settings.io_col_w;
            if pointer_pos.x < self.scene.rect.min.x + io_col_w {
                self.scene.add_input(scene::Input::new(pointer_pos.y));
            } else if pointer_pos.x > self.scene.rect.max.x - io_col_w {
                self.scene.add_output(scene::Output::new(pointer_pos.y));
            }
        }

        // CONTEXT MENU (PLACE DEVICES)
        response.context_menu(|ui| {
            ui.set_width(80.0);
            let Some(pos) = ctx.input().pointer.interact_pos() else {return };

            const LEFT_SP: f32 = 15.0;

            let mut place_preset = None;
            let mut del_preset = None;

            for (cat_id, cat) in &self.presets.cats {
                ui.menu_button(&cat.name, |ui| {
                    for (preset_id, preset) in &cat.presets {
                        ui.horizontal(|ui| {
                            let button = Button::new(&preset.vis.name).ui(ui);

                            if button.clicked() {
                                place_preset = Some((*cat_id, *preset_id));
                                ui.close_menu();
                            }
                            if button.hovered() && pressed_del {
                                del_preset = Some((*cat_id, *preset_id));
                            }
                        });
                    }
                });
            }

            if let Some((cat_id, id)) = place_preset {
                self.place_preset(cat_id, id, pos);
            }
            if let Some((cat_id, id)) = del_preset {
                self.presets.mut_cat(cat_id).unwrap().remove_preset(id);
            }
        });

        ctx.request_repaint_after(core::time::Duration::from_millis(1000 / 60))
    }
}
