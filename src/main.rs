// TODO impl create()
// TODO allow for changing IoLabel's in scene inputs/outputs
//  a button to the left, that opens a menu that allows for such edits

// TODO creating ID's for presets should be based off of time, so that they are stored in order of time created

pub mod graphics;
pub mod preset;
pub mod scene;

use eframe::egui::*;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SimId(u32);
impl SimId {
    pub fn new() -> Self {
        Self(fastrand::u32(..))
    }
}
impl Hash for SimId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&self.0, state)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BitField(pub u64);
impl BitField {
    pub fn set(&mut self, pos: usize, state: bool) {
        self.0 = (self.0 & !(1 << pos as u64)) | ((state as u64) << pos);
    }
    pub fn get(&self, pos: usize) -> bool {
        ((self.0 >> pos as u64) & 1) == 1
    }

    pub fn bits(self, size: usize) -> Vec<bool> {
        let mut bits = Vec::with_capacity(size);
        for i in 0..size {
            bits.push(self.get(i));
        }
        bits
    }
}

#[derive(Debug, Clone)]
pub struct TruthTable {
    pub num_inputs: usize,
    pub num_outputs: usize,
    pub map: Vec<BitField>,
}
impl TruthTable {
    #[inline(always)]
    pub fn get(&self, input: BitField) -> BitField {
        self.map[input.0 as usize]
    }
}

pub type Presets = HashMap<SimId, preset::Device>;

struct App {
    input_space: f32,
    output_space: f32,
    canvas_rect: Rect,
    presets: Presets,
    scene: scene::Scene,
    link_start: Option<scene::LinkStart>,
    paused: bool,
    speed: u32,
}
impl App {
    pub fn new() -> Self {
        let default_presets = preset::default_presets();

        let mut presets = HashMap::with_capacity(default_presets.len());
        for preset in default_presets {
            presets.insert(SimId::new(), preset);
        }

        Self {
            input_space: 40.0,
            output_space: 40.0,
            canvas_rect: Rect {
                min: Pos2::ZERO,
                max: Pos2::ZERO,
            },
            presets,
            scene: scene::Scene::new(),
            paused: false,
            speed: 1,
            link_start: None,
        }
    }

    pub fn create(&mut self) {
        let chip = preset::chip::Chip::from_scene(&self.scene);
        self.presets
            .insert(SimId::new(), preset::Device::Chip(chip));
        self.scene = scene::Scene::new();
    }

    pub fn place_preset(&mut self, preset_id: SimId, pos: Pos2) {
        let preset = self.presets.get(&preset_id).unwrap().clone();

        self.scene.alloc_preset(preset_id, &preset, pos);
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _win_frame: &mut eframe::Frame) {
        let pointer_pos = ctx
            .input()
            .pointer
            .interact_pos()
            .unwrap_or(Pos2::new(200.0, 200.0));
        let pressed_del = ctx.input().key_pressed(Key::Backspace);

        if !self.paused {
            self.scene.update();
        }

        ctx.set_visuals(Visuals::dark());
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.scene.name);

                ui.color_edit_button_rgb(&mut self.scene.color);

                ui.checkbox(&mut self.scene.combinational, "combinational");

                if ui.button("Create").clicked() {
                    self.create();
                }
            });
        });

        TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
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
                    println!("scene: {:#?}\n", self.scene);
                }
                if ui.button("debug presets").clicked() {
                    for (id, preset) in &self.presets {
                        println!("preset {:?}: {:#?}\n", id, preset);
                    }
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
                    self.canvas_rect = response.rect;

                    use graphics::SubInteraction;
                    let mut ctx =
                        graphics::Context::new(ui, Id::new("board"), self.canvas_rect, pointer_pos);

                    // SCENE INPUT
                    {
                        let input_x = ctx.canvas_rect.min.x + self.input_space;
                        let output_x = ctx.canvas_rect.max.x - self.output_space;
                        let stroke = Stroke {
                            width: 2.0,
                            color: Color32::WHITE,
                        };
                        ctx.shapes.push(Shape::line_segment(
                            [
                                Pos2::new(input_x, ctx.canvas_rect.min.y),
                                Pos2::new(input_x, ctx.canvas_rect.max.y),
                            ],
                            stroke,
                        ));
                        ctx.shapes.push(Shape::line_segment(
                            [
                                Pos2::new(output_x, ctx.canvas_rect.min.y),
                                Pos2::new(output_x, ctx.canvas_rect.max.y),
                            ],
                            stroke,
                        ));
                    }

                    let io_size = Vec2::new(20.0, 5.0);
                    for (_id, device) in &mut self.scene.devices {
                        let device_preset = self.presets.get(&device.preset).unwrap();
                        let (width, height) = device_preset.size();

                        // tl: Pos2, height: f32, _io_size: Vec2, num_ios: usize
                        let input_locs = graphics::calc_io_locs(
                            device.pos - Vec2::new(io_size.x, 0.0),
                            height,
                            device.data.num_inputs(),
                        );

                        device.input_locs = input_locs;

                        let output_locs = graphics::calc_io_locs(
                            device.pos + Vec2::new(width + io_size.x, 0.0),
                            height,
                            device.data.num_outputs(),
                        );

                        device.output_locs = output_locs;
                    }

                    let mut dead_links = Vec::new();
                    let scene_int =
                        graphics::show_scene(&mut ctx, &self.scene, &self.presets, &mut dead_links);
                    for (start, link_idx) in dead_links {
                        match start {
                            scene::LinkStart::SceneInput(input) => self
                                .scene
                                .inputs
                                .get_mut(&input)
                                .unwrap()
                                .links
                                .remove(link_idx),
                            scene::LinkStart::DeviceOutput(device, output) => {
                                self.scene.devices.get_mut(&device).unwrap().links[output]
                                    .remove(link_idx)
                            }
                        };
                    }

                    use graphics::SceneInteraction::*;
                    match scene_int {
                        None => (),
                        Some(Device(int)) => {
                            let device = self.scene.devices.get_mut(&int.sub).unwrap();

                            device.pos += int.int.drag;

                            if int.int.hovered && pressed_del {
                                self.scene.devices.remove(&int.sub);
                            }
                        }
                        Some(Input(SubInteraction { sub: id, int })) => {
                            if int.clicked {
                                let state = self.scene.get_input(id);
                                self.scene.set_input(id, !state);
                            } else if int.secondary_clicked {
                                self.link_start = Some(scene::LinkStart::SceneInput(id));
                            }
                            if int.hovered && pressed_del {
                                self.scene.inputs.remove(&id);
                            }
                        }
                        Some(Output(SubInteraction { sub: id, int })) => {
                            if int.clicked {
                                if let Some(link_start) = self.link_start.clone() {
                                    self.scene
                                        .add_link(link_start, scene::WriteTarget::SceneOutput(id));
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
                                        scene::WriteTarget::DeviceInput(device, input),
                                    );
                                    self.link_start = None;
                                } else {
                                    let state = self.scene.get_device_input(device, input);
                                    self.scene.set_device_input(device, input, !state);
                                }
                            }
                        }
                        Some(DeviceOutput(device, sub_int)) => {
                            let SubInteraction { sub: output, int } = sub_int;

                            if int.clicked {
                                self.link_start =
                                    Some(scene::LinkStart::DeviceOutput(device, output));
                            }
                        }
                    }

                    if let Some(link_start) = &self.link_start {
                        // TODO remove unwraps
                        if let Some(from) = self.scene.get_link_start_loc(&ctx, link_start.clone())
                        {
                            let state = match link_start {
                                scene::LinkStart::SceneInput(input) => self.scene.get_input(*input),
                                scene::LinkStart::DeviceOutput(device, output) => {
                                    self.scene.get_device_output(*device, *output)
                                }
                            };
                            graphics::show_link(&mut ctx, state, from, pointer_pos);
                        } else {
                            self.link_start = None;
                        };

                        if pressed_del {
                            self.link_start = None;
                        }
                    }

                    let graphics::Context { shapes, .. } = ctx;

                    painter.extend(shapes);
                });
            })
            .response
            .interact(Sense::click_and_drag());

        if response.clicked() {
            if pointer_pos.x < self.canvas_rect.min.x + self.input_space {
                self.scene.alloc_input(scene::Input::default());
            } else if pointer_pos.x > self.canvas_rect.max.x - self.output_space {
                self.scene.alloc_output(scene::Output::default());
            }
        }

        response.context_menu(|ui| {
            let Some(pos) = ctx.input().pointer.interact_pos() else {return };

            const LEFT_SP: f32 = 15.0;

            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.add_space(LEFT_SP);
                ui.label(RichText::new("Place device").strong());
            });
            ui.separator();

            let mut place_preset = None;
            // let mut del_preset = None;

            for (id, preset) in &self.presets {
                ui.horizontal(|ui| {
                    ui.add_space(LEFT_SP);

                    if ui.button(preset.name()).clicked() {
                        place_preset = Some(*id);
                        ui.close_menu();
                    }
                    // if ui.button("del").clicked() {
                    //     del_preset = Some(idx);
                    // }
                });
            }
            if let Some(preset) = place_preset {
                self.place_preset(preset, pos);
            }
            // if let Some(preset) = del_preset {
            //     self.presets.remove(preset);
            // }
        });

        ctx.request_repaint_after(core::time::Duration::from_millis(1000 / 60))
    }
}

fn main() {
    let native_options = eframe::NativeOptions::default();

    eframe::run_native(
        "Logic Sim",
        native_options,
        Box::new(|_cc| Box::new(App::new())),
    );
}
