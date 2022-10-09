#![feature(never_type)]
#![feature(exhaustive_patterns)]

// TODO allow for changing IoLabel's in scene inputs/outputs
//  a button to the left, that opens a menu that allows for such edits

pub mod debug;
pub mod graphics;
pub mod preset;
pub mod scene;

use debug::good_debug;
use eframe::egui::*;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
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

pub trait IoAccess<T> {
    fn get_input(&self, input: usize) -> T;
    fn get_output(&self, output: usize) -> T;

    fn num_inputs(&self) -> usize;
    fn num_outputs(&self) -> usize;
}
impl<T> IoAccess<T> for ! {
    fn get_input(&self, _input: usize) -> T {
        unreachable!()
    }
    fn get_output(&self, _output: usize) -> T {
        unreachable!()
    }

    fn num_inputs(&self) -> usize {
        unreachable!()
    }
    fn num_outputs(&self) -> usize {
        unreachable!()
    }
}

#[derive(Clone, Debug)]
pub enum LinkTarget<T> {
    DeviceInput(T, usize),
    Output(T),
}
#[derive(Clone, Debug)]
pub enum LinkStart<T> {
    DeviceOutput(T, usize),
    Input(T),
}

#[derive(Clone, Debug)]
pub enum DeviceData<S, C, G> {
    CombGate(G),
    Chip(C),
    Light(S),
    Switch(S),
}
impl<S: Clone, C: IoAccess<S>, G: IoAccess<S>> DeviceData<S, C, G> {
    pub fn get_output(&self, output: usize) -> S {
        match self {
            Self::CombGate(e) => e.get_output(output),
            Self::Chip(e) => e.get_output(output),
            Self::Light(_) => panic!("a light doesnt have an output"),
            Self::Switch(state) => {
                assert_eq!(output, 0);
                state.clone()
            }
        }
    }
    pub fn get_input(&self, input: usize) -> S {
        match self {
            Self::CombGate(e) => e.get_input(input),
            Self::Chip(e) => e.get_input(input),
            Self::Light(state) => {
                assert_eq!(input, 0);
                state.clone()
            }
            Self::Switch(_) => panic!("a switch doesnt have an input"),
        }
    }

    pub fn num_inputs(&self) -> usize {
        match self {
            Self::CombGate(e) => e.num_inputs(),
            Self::Chip(e) => e.num_inputs(),
            Self::Light(_) => 1,
            Self::Switch(_) => 0,
        }
    }
    pub fn num_outputs(&self) -> usize {
        match self {
            Self::CombGate(e) => e.num_outputs(),
            Self::Chip(e) => e.num_outputs(),
            Self::Light(_) => 0,
            Self::Switch(_) => 1,
        }
    }
}

struct CreatePreset {
    name: String,
    color: [f32; 3],
    category: SimId,
    combinational: bool,
}
impl CreatePreset {
    pub fn new() -> Self {
        Self {
            name: format!("New Chip {}", fastrand::u16(10000..)),
            color: [1.0; 3],
            category: SimId(0),
            combinational: false,
        }
    }
}

struct App {
    input_space: f32,
    output_space: f32,
    canvas_rect: Rect,
    presets: preset::Presets,
    scene: scene::Scene,
    link_start: Option<LinkStart<SimId>>,
    paused: bool,
    speed: u32,
    create_preset: CreatePreset,
}
impl App {
    pub fn new() -> Self {
        Self {
            input_space: 40.0,
            output_space: 40.0,
            canvas_rect: Rect {
                min: Pos2::ZERO,
                max: Pos2::ZERO,
            },
            presets: preset::Presets::defaults(),
            scene: scene::Scene::new(),
            paused: false,
            speed: 1,
            link_start: None,
            create_preset: CreatePreset::new(),
        }
    }

    pub fn create(&mut self) {
        let mut create_preset = CreatePreset::new();
        std::mem::swap(&mut create_preset, &mut self.create_preset);

        let CreatePreset {
            name,
            color,
            category,
            combinational,
        } = create_preset;

        if combinational {
            unimplemented!("creating combination gates is not supported yet");
        }

        let chip = preset::chip::Chip::from_scene(&name, color, &self.scene);
        self.presets
            .add_preset(category, preset::DeviceData::Chip(chip));
        self.scene = scene::Scene::new();
    }

    pub fn place_preset(&mut self, id: SimId, pos: Pos2) {
        let preset = self.presets.get_preset(id).unwrap().clone();

        self.scene.alloc_preset(id, &preset, &self.presets, pos);
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
                if ui.button("Create Preset").clicked() {
                    self.create();
                }

                ui.label("name: ");
                ui.text_edit_singleline(&mut self.create_preset.name);

                ui.label("color: ");
                ui.color_edit_button_rgb(&mut self.create_preset.color);

                ui.label("combinational: ");
                ui.checkbox(&mut self.create_preset.combinational, "");
            });
        });

        TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Sim: ");

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
                    todo!()
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

                    let mut dead_devices = Vec::new();

                    use graphics::DEVICE_IO_SIZE;
                    for (device_id, device) in &mut self.scene.devices {
                        let Some(device_preset) = self.presets.get_preset(device.preset) else {
                        	dead_devices.push(*device_id);
                        	continue
                        };
                        let (width, height) = device_preset.size();

                        // tl: Pos2, height: f32, num_ios: usize
                        let input_locs = graphics::calc_io_locs(
                            device.pos - Vec2::new(DEVICE_IO_SIZE.x, 0.0),
                            height,
                            device.data.num_inputs(),
                        );

                        device.input_locs = input_locs;

                        let output_locs = graphics::calc_io_locs(
                            device.pos + Vec2::new(width + DEVICE_IO_SIZE.x, 0.0),
                            height,
                            device.data.num_outputs(),
                        );

                        device.output_locs = output_locs;
                    }
                    for dead_device in dead_devices {
                        self.scene.devices.remove(&dead_device);
                    }

                    let mut dead_links = Vec::new();
                    let scene_int =
                        graphics::show_scene(&mut ctx, &self.scene, &self.presets, &mut dead_links);
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
                                self.scene.write_input(id, !state);
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
                                    let state = self.scene.get_device_input(device, input);
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

                    if let Some(link_start) = &self.link_start {
                        if let Some(from) = self.scene.get_link_start_loc(&ctx, link_start.clone())
                        {
                            let state = match link_start {
                                LinkStart::Input(input) => self.scene.get_input(*input),
                                LinkStart::DeviceOutput(device, output) => {
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

            let mut place_preset = None;
            let mut del_preset = None;

            for (cat_id, presets) in self.presets.get_sorted() {
                let cat_name = self.presets.get_category_name(*cat_id).unwrap();
                ui.horizontal(|ui| {
                    ui.add_space(LEFT_SP);
                    ui.label(RichText::new(cat_name).strong());
                });
                ui.separator();
                ui.add_space(5.0);
                for preset_id in presets {
                    let preset = self.presets.get_preset(*preset_id).unwrap();
                    ui.horizontal(|ui| {
                        ui.add_space(LEFT_SP);

                        let stroke = Stroke {
                            width: 2.0,
                            color: preset.color().unwrap_or(Color32::WHITE),
                        };
                        let button = Button::new(preset.name()).stroke(stroke).ui(ui);
                        if button.clicked() {
                            place_preset = Some(*preset_id);
                            ui.close_menu();
                        }
                        if button.hovered() && pressed_del {
                            del_preset = Some(*preset_id);
                        }
                    });
                }
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

fn main() {
    let native_options = eframe::NativeOptions::default();

    eframe::run_native(
        "Logic Sim",
        native_options,
        Box::new(|_cc| Box::new(App::new())),
    );
}
