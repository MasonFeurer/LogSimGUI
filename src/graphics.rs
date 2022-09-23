use crate::preset;
use crate::sim::{self, BoardWrite, BoardWriteTarget};
use crate::LinkStart;
use eframe::egui::*;

// tl: 210.0 37.0
// br: 589.7 562.4
pub const EDITOR_POS: Pos2 = Pos2::new(210.0, 37.0);
pub const EDITOR_SIZE: Vec2 = Vec2::new(590.0 - 210.0, 562.0 - 37.0);
pub const EDITOR_IO_SIZE: Vec2 = Vec2::new(30.0, 10.0);
pub const EDITOR_IO_SP: f32 = 20.0;

pub trait DeviceVisuals {
    fn name(&self) -> String;
    fn color(&self) -> Color32;
    fn size(&self) -> Vec2;
    fn inputs_locs(&self) -> Vec<Pos2>;
    fn output_locs(&self) -> Vec<Pos2>;
}

//
#[derive(Clone, Debug)]
pub struct SetInput {
    pub input: usize,
    pub state: bool,
}

pub struct Context<'a> {
    pub shapes: Vec<Shape>,
    pub ui: &'a Ui,
    pub id_stack: Vec<Id>,
    pub link_start: Option<LinkStart>,
}
impl<'a> Context<'a> {
    pub fn new(ui: &'a Ui, link_start: Option<LinkStart>, first_id: Id) -> Self {
        Self {
            shapes: Vec::new(),
            ui,
            id_stack: vec![first_id],
            link_start,
        }
    }

    pub fn push_id(&mut self, map: impl Fn(Id) -> Id) {
        self.id_stack.push(map(*self.id_stack.last().unwrap()));
    }
    pub fn pop_id(&mut self) {
        self.id_stack.pop();
    }
    pub fn id(&self) -> Id {
        *self.id_stack.last().unwrap()
    }
}

#[derive(Copy, Clone)]
pub struct DeviceDrag {
    pub device: usize,
    pub delta: Vec2,
}
#[derive(Clone)]
pub struct AddLink {
    pub device: usize,
    pub target: BoardWriteTarget,
}

#[derive(Default)]
pub struct ShowBoardContentResult {
    pub device_drag: Option<DeviceDrag>,
    pub set_input: Option<BoardWrite>,
    pub finish_link: Option<BoardWriteTarget>,
    pub start_link: Option<LinkStart>,
    pub toggle_input: Option<usize>,
}
#[derive(Default)]
pub struct ShowDeviceResult {
    pub drag: Option<Vec2>,
    pub set_input: Option<SetInput>,
    pub finish_link: Option<usize>,
    pub start_link: Option<usize>,
}
#[derive(Default)]
pub struct ShowIoResult {
    pub hovered: bool,
    pub clicked: bool,
}
#[derive(Default)]
pub struct WrappedShowIoResult {
    pub io: usize,
    pub hovered: bool,
    pub clicked: bool,
}

fn show_io(
    ctx: &mut Context,
    size: Vec2,
    pos: Pos2,
    io: (Option<preset::IoLabel>, bool),
) -> ShowIoResult {
    let mut result = ShowIoResult::default();

    let color = if io.1 {
        Color32::from_rgb(255, 0, 0)
    } else {
        Color32::from_rgb(150, 150, 150)
    };

    let rect = Rect::from_min_size(pos - Vec2::new(0.0, size.y * 0.5), size);
    let mut response = ctx.ui.interact(rect, ctx.id(), Sense::click_and_drag());
    if let Some(preset::IoLabel {
        implicit: false,
        name,
    }) = &io.0
    {
        if !name.trim().is_empty() {
            response = response.on_hover_text(name);
        }
    }

    let rounding = Rounding::none();

    ctx.shapes.push(Shape::rect_filled(rect, rounding, color));

    if response.hovered() {
        result.hovered = true;
        let stroke = Stroke {
            width: 1.0,
            color: Color32::WHITE,
        };
        ctx.shapes.push(Shape::rect_stroke(rect, rounding, stroke));
    }
    if response.clicked() {
        result.clicked = true;
    }
    result
}

pub fn calc_io_locs(top: Pos2, height: f32, num_ios: usize) -> Vec<Pos2> {
    let mut locs = Vec::with_capacity(num_ios);

    let sp = height / (num_ios + 1) as f32;

    let mut y = sp;
    for _ in 0..num_ios {
        let pos = top + Vec2::new(0.0, y);
        locs.push(pos);
        y += sp;
    }
    locs
}

pub fn calc_io_unsized_locs(top: Pos2, num_ios: usize, sp: f32) -> Vec<Pos2> {
    let mut locs = Vec::with_capacity(num_ios);

    let mut y = sp;
    for _ in 0..num_ios {
        let pos = top + Vec2::new(0.0, y);
        locs.push(pos);
        y += sp;
    }
    locs
}

pub fn show_ios(
    ctx: &mut Context,
    id_step: &str,
    tl: Pos2,
    height: f32,
    io_size: Vec2,
    ios: &[(Option<preset::IoLabel>, bool)],
) -> Option<WrappedShowIoResult> {
    let mut result = None;

    let locs = calc_io_locs(tl, height, ios.len());

    for i in 0..ios.len() {
        ctx.push_id(|id| id.with(id_step).with(i));

        let per_result = show_io(ctx, io_size, locs[i], ios[i].clone());

        ctx.pop_id();

        if per_result.clicked || per_result.hovered {
            result = Some(WrappedShowIoResult {
                io: i,
                hovered: per_result.hovered,
                clicked: per_result.clicked,
            });
        }
    }
    result
}

pub fn show_device(
    ctx: &mut Context,
    pos: Pos2,
    sim: &sim::Device,
    preset: &preset::Device,
) -> ShowDeviceResult {
    let mut result = ShowDeviceResult::default();

    let (width, height) = preset.size();

    let rect = Rect::from_min_size(pos, Vec2::new(width, height));

    // interact with rect
    let response = ctx.ui.interact(rect, ctx.id(), Sense::drag());
    let drag = response.drag_delta();
    if drag.x != 0.0 || drag.y != 0.0 {
        result.drag = Some(response.drag_delta());
    }

    // draw
    let stroke = if response.hovered() {
        Stroke {
            width: 3.0,
            color: Color32::from_rgb(255, 255, 255),
        }
    } else {
        Stroke {
            width: 3.0,
            color: Color32::from_rgb(150, 150, 150),
        }
    };

    let [r, g, b] = preset.color.map(|e| (e * 255.0) as u8);
    let color = Color32::from_rgb(r, g, b);
    let rounding = Rounding::same(3.0);

    ctx.shapes.push(Shape::rect_filled(rect, rounding, color));
    ctx.shapes.push(Shape::rect_stroke(rect, rounding, stroke));

    // show inputs & outputs
    let io_size = Vec2::new(20.0, 5.0);

    let preset_inputs: Vec<(Option<preset::IoLabel>, bool)> = (0..preset.num_inputs())
        .into_iter()
        .map(|i| (preset.get_input_label(i), sim.get_input(i)))
        .collect();

    let io_result = show_ios(
        ctx,
        ".input",
        pos - Vec2::new(io_size.x, 0.0),
        height,
        io_size,
        &preset_inputs,
    );

    if let Some(WrappedShowIoResult { io, clicked, .. }) = io_result {
        if clicked {
            if let Some(_) = ctx.link_start {
                result.finish_link = Some(io);
            } else {
                result.set_input = Some(SetInput {
                    input: io,
                    state: !sim.get_input(io),
                });
            }
        }
    }

    // output
    let preset_outputs: Vec<(Option<preset::IoLabel>, bool)> = (0..preset.num_outputs())
        .into_iter()
        .map(|i| (preset.get_output_label(i), sim.get_output(i)))
        .collect();

    let io_result = show_ios(
        ctx,
        ".output",
        pos + Vec2::new(width, 0.0),
        height,
        io_size,
        &preset_outputs,
    );

    if let Some(io_result) = io_result {
        if io_result.clicked {
            // println!("clicked on device output, starting link...");
            result.start_link = Some(io_result.io);
        }
    }
    result
}

pub fn show_link(ctx: &mut Context, state: bool, from: Pos2, to: Pos2) {
    let color = if state {
        Color32::from_rgb(255, 0, 0)
    } else {
        Color32::from_rgb(150, 150, 150)
    };
    let stroke = Stroke { width: 3.0, color };
    ctx.shapes.push(Shape::line_segment([from, to], stroke));
}

pub fn show_board_content(
    ctx: &mut Context,
    sim: &sim::Board,
    preset: &preset::Board,
) -> ShowBoardContentResult {
    let mut result = ShowBoardContentResult::default();

    for device_index in 0..sim.devices.len() {
        let device_sim = &sim.devices[device_index];
        let device_preset = &preset.devices[device_index];

        ctx.push_id(|id| id.with(".device_").with(device_index));
        let temp_result = show_device(
            ctx,
            device_preset.pos,
            &device_sim.device,
            &device_preset.device,
        );
        ctx.pop_id();

        if let Some(input) = temp_result.finish_link {
            result.finish_link = Some(BoardWriteTarget::DeviceInput(device_index, input));
        }
        if let Some(output) = temp_result.start_link {
            result.start_link = Some(LinkStart::DeviceOutput(device_index, output));
        }
        if let Some(set_input) = temp_result.set_input {
            result.set_input = Some(sim::BoardWrite {
                target: sim::BoardWriteTarget::DeviceInput(device_index, set_input.input),
                state: set_input.state,
            });
        }
        if let Some(drag) = temp_result.drag {
            result.device_drag = Some(DeviceDrag {
                device: device_index,
                delta: drag,
            });
        }

        for link in 0..device_sim.links.len() {
            for link_sim in &device_sim.links[link] {
                let state = device_sim.device.get_output(link);

                let from = device_preset.output_locs[link];
                let to = preset.get_target_loc(link_sim.target);

                show_link(ctx, state, from, to);
            }
        }
    }

    let input_locs = calc_io_unsized_locs(EDITOR_POS, sim.inputs.len(), EDITOR_IO_SP);

    for input in 0..sim.inputs.len() {
        let sim_input = &sim.inputs[input];
        let preset_input = &preset.inputs[input];

        ctx.push_id(|id| id.with(".input_base_").with(input));
        let io_base_result = show_io(
            ctx,
            EDITOR_IO_SIZE * Vec2::new(0.5, 1.0),
            input_locs[input],
            (Some(preset_input.label.clone()), sim_input.state),
        );
        ctx.pop_id();

        if io_base_result.clicked {
            result.toggle_input = Some(input);
        }

        ctx.push_id(|id| id.with(".input_tip_").with(input));
        let io_tip_result = show_io(
            ctx,
            EDITOR_IO_SIZE * Vec2::new(0.5, 1.0),
            input_locs[input] + Vec2::new(EDITOR_IO_SIZE.x * 0.5, 0.0),
            (Some(preset_input.label.clone()), sim_input.state),
        );
        ctx.pop_id();

        if io_tip_result.clicked {
            result.start_link = Some(LinkStart::BoardInput(input));
        }

        let from = input_locs[input];
        for link_target in &sim_input.links {
            let to = preset.get_target_loc(link_target.clone());

            show_link(ctx, sim_input.state, from, to);
        }
    }

    let output_locs = calc_io_unsized_locs(
        EDITOR_POS + Vec2::new(EDITOR_SIZE.x - EDITOR_IO_SIZE.x, 0.0),
        sim.outputs.len(),
        EDITOR_IO_SP,
    );
    for output in 0..sim.outputs.len() {
        let sim_output = &sim.outputs[output];
        let preset_output = &preset.outputs[output];

        ctx.push_id(|id| id.with(".output_").with(output));
        let io_result = show_io(
            ctx,
            EDITOR_IO_SIZE,
            output_locs[output],
            (Some(preset_output.label.clone()), sim_output.state),
        );
        ctx.pop_id();

        if io_result.clicked {
            if let Some(_) = ctx.link_start {
                result.finish_link = Some(sim::BoardWriteTarget::BoardOutput(output));
            }
        }
    }
    result
}
