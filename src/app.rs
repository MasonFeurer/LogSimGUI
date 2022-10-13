use crate::scene::IO_COL_W;
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
    cat: Option<IntId>,
}

pub struct App {
    presets: preset::Presets,
    scene: scene::Scene,
    link_start: Option<LinkStart<IntId>>,
    paused: bool,
    speed: u32,
    create_preset: CreatePreset,
    shortcut_placer: Option<ShortcutPlacer>,
}
impl App {
    pub fn new() -> Self {
        Self {
            presets: preset::Presets::defaults(),
            scene: scene::Scene::new(),
            paused: false,
            speed: 1,
            link_start: None,
            create_preset: CreatePreset::new(),
            shortcut_placer: None,
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
            unimplemented!("creating combination gates is not supported yet");
        }

        let chip = preset::chip::Chip::from_scene(&name, color, &self.scene);
        self.presets.add_preset(cat, preset::Preset::Chip(chip));
        self.scene = scene::Scene::new();
    }

    pub fn place_preset(&mut self, id: IntId, pos: Pos2) {
        let preset = self.presets.get_preset(id).unwrap();

        let data = scene::DeviceData::from_preset(id, preset, &self.presets);

        let device = scene::Device::new(id, preset, data, pos);

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
                if ui.button("Create Preset").clicked() {
                    self.create();
                }
                ui.separator();

                let name = self
                    .presets
                    .get_cat(self.create_preset.cat)
                    .unwrap()
                    .to_owned();
                ui.menu_button(format!("category: {name}"), |ui| {
                    let mut choose_cat = None;

                    const LEFT_SP: f32 = 15.0;

                    let mut del_cat = None;

                    for (cat_id, cat_name) in self.presets.get_cats() {
                        ui.horizontal(|ui| {
                            ui.add_space(LEFT_SP);
                            let cat_button = ui.button(cat_name);
                            if cat_button.clicked() {
                                choose_cat = Some(cat_id);
                                ui.close_menu();
                            }
                            if pressed_del && cat_button.hovered() {
                                del_cat = Some(cat_id);
                            }
                        });
                    }

                    if let Some(cat_id) = del_cat && self.presets.get_cats().len() > 1 {
                        self.presets.remove_cat(cat_id);
                    }

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.create_preset.new_cat_name);
                        let add_button = ui.button("+");
                        if add_button.clicked() {
                            self.presets
                                .add_cat(self.create_preset.new_cat_name.clone());
                            self.create_preset.new_cat_name = String::new();
                        }
                        add_button.on_hover_text("New Preset Category");
                        ui.add_space(5.0);
                    });
                    ui.add_space(5.0);

                    if let Some(id) = choose_cat {
                        self.create_preset.cat = id;
                    }
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

                let pause_unpause_txt = if self.paused { "unpause" } else { "pause" };
                let pause_response = ui.button(pause_unpause_txt);
                if pause_response.clicked() {
                    self.paused = !self.paused;
                }

                let step_response = ui.button("step");
                if step_response.clicked() {
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
                        graphics::Context::new(ui, Id::new("board"), response.rect, pointer_pos);

                    // SHOW SCENE
                    let mut dead_links = Vec::new();
                    let scene_int =
                        graphics::show_scene(&mut ctx, &self.scene, &self.presets, &mut dead_links);

                    // HANDLE DEAD LINKS
                    dead_links.sort_by(|a, b| a.1.cmp(&b.1).reverse());
                    for (start, link_idx) in dead_links {
                        match start {
                            LinkStart::Input(input) => self
                                .scene
                                .inputs
                                .get_mut(&input)
                                .unwrap()
                                .links
                                .remove(link_idx),
                            LinkStart::DeviceOutput(device, output) => {
                                self.scene.devices.get_mut(&device).unwrap().links[output]
                                    .remove(link_idx)
                            }
                        };
                    }

                    // HANDLE SCENE INTERACTIONS
                    use graphics::SceneInteraction::*;
                    use graphics::SubInteraction;
                    match scene_int {
                        None => (),
                        Some(Device(int)) => {
                            let device = self.scene.devices.get_mut(&int.sub).unwrap();

                            device.pos[0] += int.int.drag.x;
                            device.pos[1] += int.int.drag.y;

                            if int.int.hovered && pressed_del {
                                self.scene.devices.remove(&int.sub);
                            }
                        }
                        Some(Input(SubInteraction { sub: id, int })) => {
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
                        Some(Output(SubInteraction { sub: id, int })) => {
                            if int.clicked {
                                if let Some(link_start) = self.link_start.clone() {
                                    self.scene.add_link(link_start, LinkTarget::Output(id));
                                    self.link_start = None;
                                }
                            }
                            if int.hovered && pressed_del {
                                self.scene.outputs.remove(&id);
                            }
                        }
                        Some(DeviceInput(device, sub_int)) => {
                            let SubInteraction { sub: input, int } = sub_int;

                            if int.clicked {
                                if let Some(link_start) = self.link_start.clone() {
                                    self.scene.add_link(
                                        link_start,
                                        LinkTarget::DeviceInput(device, input),
                                    );
                                    self.link_start = None;
                                } else {
                                    let state = self.scene.get_device_input(device, input).unwrap();
                                    self.scene.set_device_input(device, input, !state);
                                }
                            }
                        }
                        Some(DeviceOutput(device, sub_int)) => {
                            let SubInteraction { sub: output, int } = sub_int;

                            if int.clicked {
                                self.link_start = Some(LinkStart::DeviceOutput(device, output));
                            }
                        }
                    }

                    // DRAW LINK TO CURSOR (IF CREATING ONE)
                    if let Some(link_start) = &self.link_start {
                        if let Some((def, state)) = self.scene.get_link_start(link_start) {
                            graphics::show_link(&mut ctx, state, def.tip_loc(), pointer_pos);
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
                if let Some(pressed_num) = pressed_num && over_scene{
                    if let Some(shortcut_placer) = &self.shortcut_placer {
                        if let Some(cat) = shortcut_placer.cat {
                            if let Some(preset_id) = self
                                .presets
                                .get_cat_presets(cat)
                                .iter()
                                .nth(pressed_num)
                                .map(|(id, _)| *id)
                            {
                                self.place_preset(preset_id, shortcut_placer.pos);
                                self.shortcut_placer = None;
                            }
                        }
                    } else {
                        let cat = self
                            .presets
                            .get_cats()
                            .iter()
                            .nth(pressed_num)
                            .map(|(id, _)| *id);
                        if cat.is_some() {
                            self.shortcut_placer = Some(ShortcutPlacer {
                                pos: pointer_pos,
                                cat,
                            });
                        }
                    }
                }
                if let Some(ShortcutPlacer { pos, cat }) = &self.shortcut_placer {
                    let mut child_ui = ui.child_ui(
                        Rect::from_min_size(*pos, Vec2::new(300.0, 300.0)),
                        ui.layout().clone(),
                    );
                    const SP: f32 = 20.0;
                    if let Some(cat) = cat {
                        child_ui.set_width(80.0);
                        child_ui.separator();
                        for (idx, (_, preset)) in
                            self.presets.get_cat_presets(*cat).iter().enumerate()
                        {
                            child_ui.horizontal(|ui| {
                                ui.add_space(SP);
                                ui.label(format!("{} - {}", idx, preset.name()));
                            });
                        }
                        child_ui.separator();
                    } else {
                        for (_, cat_name) in &self.presets.get_cats() {
                            child_ui.horizontal(|ui| {
                                ui.add_space(SP);
                                ui.label(cat_name);
                            });
                        }
                    }
                }
            })
            .response
            .interact(Sense::click_and_drag());

        // PLACE INPUTS/OUTPUTS
        if response.clicked() {
            if pointer_pos.x < self.scene.rect.min.x + IO_COL_W {
                self.scene.add_input(scene::Io::default_at(pointer_pos.y));
            } else if pointer_pos.x > self.scene.rect.max.x - IO_COL_W {
                self.scene.add_output(scene::Io::default_at(pointer_pos.y));
            }
        }

        // CONTEXT MENU (PLACE DEVICES)
        response.context_menu(|ui| {
            ui.set_width(80.0);
            let Some(pos) = ctx.input().pointer.interact_pos() else {return };

            const LEFT_SP: f32 = 15.0;

            ui.add_space(5.0);

            let mut place_preset = None;
            let mut del_preset = None;

            for (cat_id, cat_name) in self.presets.get_cats() {
                ui.menu_button(cat_name, |ui| {
                    for (preset_id, preset) in self.presets.get_cat_presets(cat_id) {
                        ui.horizontal(|ui| {
                            let button = Button::new(preset.name()).ui(ui);
                            if button.clicked() {
                                place_preset = Some(preset_id);
                                ui.close_menu();
                            }
                            if button.hovered() && pressed_del {
                                del_preset = Some(preset_id);
                            }
                        });
                    }
                });
            }

            if let Some(id) = place_preset {
                self.place_preset(id, pos);
            }
            if let Some(id) = del_preset {
                self.presets.remove_preset(id);
            }
        });

        ctx.request_repaint_after(core::time::Duration::from_millis(1000 / 60))
    }
}
