#![feature(let_else)]

pub mod graphics;
pub mod preset;
pub mod sim;

use eframe::egui::*;

pub type Color = [u8; 3];
pub type Pos = [f32; 2];

#[derive(Copy, Clone, Debug)]
pub enum LinkStart {
    BoardInput(usize),
    DeviceOutput(usize, usize),
}

fn main() {
    let native_options = eframe::NativeOptions::default();

    eframe::run_native(
        "Logic Sim",
        native_options,
        Box::new(|_cc| Box::new(App::new())),
    );
}

struct App {
    presets: Vec<preset::Device>,
    board: sim::Board,
    board_name: String,
    board_color: [f32; 3],
    board_preset: preset::Board,
    combinational: bool,
    link_start: Option<LinkStart>,
    paused: bool,
    speed: u32,
    hovering_device: Option<usize>,
}
impl App {
    pub fn new() -> Self {
        let presets = preset::default_presets().to_vec();
        Self {
            presets,
            board: sim::Board::new(),
            board_name: String::from("new board"),
            board_color: [1.0; 3],
            board_preset: preset::Board::new(),
            combinational: true,
            paused: false,
            speed: 1,
            link_start: None,
            hovering_device: None,
        }
    }

    pub fn create(&mut self) {
        let mut board_preset = preset::Board::new();
        std::mem::swap(&mut self.board_preset, &mut board_preset);

        self.presets.push(preset::Device {
            name: self.board_name.clone(),
            color: self.board_color,
            data: preset::DeviceData::Board(board_preset),
        });

        self.board_name = String::new();
        self.board_preset = preset::Board::new();
        self.board = sim::Board::new();
    }

    pub fn place_preset(&mut self, preset: usize, pos: Pos2) {
        let preset_device = &self.presets[preset];

        let num_inputs = preset_device.num_inputs();
        let num_outputs = preset_device.num_outputs();
        let sim_device = preset_device.sim();

        self.board.devices.push(sim::BoardDevice {
            device: sim_device,
            links: vec![Vec::new(); num_inputs],
        });

        self.board_preset.devices.push(preset::BoardDevice {
            device: preset_device.clone(),
            pos,
            links: vec![Vec::new(); num_inputs],
            input_locs: vec![Pos2::ZERO; num_inputs],
            output_locs: vec![Pos2::ZERO; num_outputs],
        });
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _win_frame: &mut eframe::Frame) {
        if !self.paused {
            self.board.exec_writes(&mut Vec::new());
        }

        ctx.set_visuals(Visuals::dark());
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.board_name);

                ui.color_edit_button_rgb(&mut self.board_color);

                ui.checkbox(&mut self.combinational, "combinational");

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
                    self.board.exec_writes(&mut Vec::new());
                }

                ui.add(Slider::new(&mut self.speed, 0..=999));

                if ui.button("clear").clicked() {
                    self.board = sim::Board::new();
                    self.board_preset = preset::Board::new();
                }

                if ui.button("debug").clicked() {
                    println!("sim: {:#?}\n", self.board);
                    println!("preset: {:#?}", self.board_preset);
                }
            });
        });

        SidePanel::left("left_panel").show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                if ui.button("+").clicked() {
                    self.board.inputs.push(sim::BoardInput::default());
                    self.board_preset.inputs.push(preset::BoardInput::default());
                }
                if ui.button("-").clicked() {
                    self.board.inputs.pop();
                    self.board_preset.inputs.pop();
                }
                ui.heading("input");
            });
            ui.separator();

            ScrollArea::vertical().show(ui, |ui| {
                for input in &mut self.board_preset.inputs {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut input.label.implicit, "")
                            .on_hover_text("implicit");
                        ui.text_edit_singleline(&mut input.label.name);
                    });
                }
            });
        });

        SidePanel::right("right_panel").show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                if ui.button("+").clicked() {
                    self.board.outputs.push(sim::BoardOutput::default());
                    self.board_preset
                        .outputs
                        .push(preset::BoardOutput::default());
                }
                if ui.button("-").clicked() {
                    self.board.outputs.pop();
                    self.board_preset.outputs.pop();
                }
                ui.heading("output");
            });
            ui.separator();

            ScrollArea::vertical().show(ui, |ui| {
                for output in &mut self.board_preset.outputs {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut output.label.implicit, "")
                            .on_hover_text("implicit");
                        ui.text_edit_singleline(&mut output.label.name);
                    });
                }
            });
        });

        let response = CentralPanel::default()
            .show(ctx, |ui| {
                Frame::canvas(ui.style()).show(ui, |ui| {
                    let (_response, painter) = ui.allocate_painter(
                        Vec2::new(ui.available_width(), ui.available_height()),
                        Sense::hover(),
                    );

                    let mut ctx = graphics::Context::new(ui, self.link_start, Id::new("board"));

                    let io_size = Vec2::new(20.0, 5.0);
                    for device in &mut self.board_preset.devices {
                        let (width, height) = device.device.size();

                        // tl: Pos2, height: f32, _io_size: Vec2, num_ios: usize
                        let input_locs = graphics::calc_io_locs(
                            device.pos - Vec2::new(io_size.x, 0.0),
                            height,
                            device.device.num_inputs(),
                        );

                        device.input_locs = input_locs;

                        let output_locs = graphics::calc_io_locs(
                            device.pos + Vec2::new(width + io_size.x, 0.0),
                            height,
                            device.device.num_outputs(),
                        );

                        device.output_locs = output_locs;
                    }

                    let result =
                        graphics::show_board_content(&mut ctx, &self.board, &self.board_preset);
                    if let Some(link_start) = result.start_link {
                        println!("frame create a link start: {:?}", link_start);
                        self.link_start = Some(link_start);
                    }

                    if let Some(link_target) = result.finish_link {
                        let link_start = self.link_start.take().unwrap();

                        println!("linked from {:?} to {:?}", link_start, link_target);

                        let state = match link_start {
                            LinkStart::DeviceOutput(device, output) => {
                                let link = sim::BoardLink {
                                    output: output,
                                    target: link_target,
                                };
                                self.board.devices[device].links[output].push(link);
                                self.board_preset.devices[device].links[output].push(link);
                                self.board.devices[device].device.get_output(output)
                            }
                            LinkStart::BoardInput(input) => {
                                self.board.inputs[input].links.push(link_target);
                                self.board_preset.inputs[input].links.push(link_target);
                                self.board.inputs[input].state
                            }
                        };

                        self.board.queue_write(sim::BoardWrite {
                            target: link_target,
                            state,
                        });
                    }

                    if let Some(graphics::DeviceDrag { device, delta }) = result.device_drag {
                        self.board_preset.devices[device].pos += delta;
                    }

                    if let Some(write) = result.set_input {
                        self.board.queue_write(write);
                    }
                    if let Some(input) = result.toggle_input {
                        self.board.inputs[input].state ^= true;

                        for link_target in self.board.inputs[input].links.clone() {
                            self.board.queue_write(sim::BoardWrite {
                                state: self.board.inputs[input].state,
                                target: link_target,
                            });
                        }
                    }

                    let graphics::Context { shapes, .. } = ctx;

                    painter.extend(shapes);
                });
            })
            .response;

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

            for (idx, preset) in self.presets.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.add_space(LEFT_SP);

                    if ui.button(&preset.name).clicked() {
                        place_preset = Some(idx);
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
