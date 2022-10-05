use crate::{preset, scene, LinkStart, LinkTarget, Presets, SimId};
use eframe::egui::*;

// TODO read tutuorial on how this thing works :>
pub fn project_point_onto_line(p: Pos2, line: (Pos2, Pos2)) -> Pos2 {
    let (v1, v2) = line;

    // get dot product of e1, e2
    let e1 = Pos2::new(v2.x - v1.x, v2.y - v1.y);
    let e2 = Pos2::new(p.x - v1.x, p.y - v1.y);
    let dot = e1.x * e2.x + e1.y * e2.y;

    // get squared length of e1
    let len_sq = e1.x * e1.x + e1.y * e1.y;

    let result_x = v1.x + (dot * e1.x) / len_sq;
    let result_y = v1.y + (dot * e1.y) / len_sq;
    Pos2::new(result_x, result_y)
}
pub fn line_contains_point(line: (Pos2, Pos2), width: f32, point: Pos2) -> bool {
    let max_dist_sq = (width * 0.5) * (width * 0.5);

    let projected = project_point_onto_line(point, line);

    let pp = projected - point;
    let dist_sq = (pp.x * pp.x + pp.y * pp.y).abs();

    let line_min_x = line.0.x.min(line.1.x);
    let line_max_x = line.0.x.max(line.1.x);
    let line_min_y = line.0.y.min(line.1.y);
    let line_max_y = line.0.y.max(line.1.y);

    dist_sq <= max_dist_sq
        && point.x >= line_min_x
        && point.x <= line_max_x
        && point.y >= line_min_y
        && point.y <= line_max_y
}

pub const EDITOR_IO_SIZE: Vec2 = Vec2::new(30.0, 10.0);
pub const EDITOR_IO_SP: f32 = 20.0;

#[inline(always)]
pub fn state_color(state: bool) -> Color32 {
    if state {
        Color32::from_rgb(255, 0, 0)
    } else {
        Color32::from_rgb(150, 150, 150)
    }
}

pub struct Context<'a> {
    pub canvas_rect: Rect,
    pub pointer_pos: Pos2,
    pub shapes: Vec<Shape>,
    pub ui: &'a Ui,
    pub id_stack: Vec<Id>,
}
impl<'a> Context<'a> {
    pub fn new(ui: &'a Ui, first_id: Id, canvas_rect: Rect, pointer_pos: Pos2) -> Self {
        Self {
            canvas_rect,
            pointer_pos,
            shapes: Vec::new(),
            ui,
            id_stack: vec![first_id],
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

    pub fn any_click(&self) -> bool {
        self.ui.input().pointer.any_click()
    }
}

#[derive(Debug)]
pub struct Interaction {
    pub drag: Vec2,
    pub clicked: bool,
    pub secondary_clicked: bool,
    pub hovered: bool,
    pub contains_pointer: bool,
}
impl Interaction {
    pub fn new(ctx: &Context, response: Response) -> Self {
        Self {
            drag: response.drag_delta(),
            clicked: response.clicked(),
            hovered: response.hovered(),
            secondary_clicked: response.secondary_clicked(),
            contains_pointer: response.rect.contains(ctx.pointer_pos),
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.drag == Vec2::ZERO
            && !self.clicked
            && !self.secondary_clicked
            && !self.hovered
            && !self.contains_pointer
    }

    #[inline(always)]
    pub fn sub<T>(self, sub: T) -> SubInteraction<T> {
        SubInteraction { int: self, sub }
    }
}

#[derive(Debug)]
pub struct SubInteraction<T> {
    pub int: Interaction,
    pub sub: T,
}

fn show_io(
    ctx: &mut Context,
    size: Vec2,
    pos: Pos2,
    label: preset::IoLabel,
    state: bool,
) -> Interaction {
    let color = state_color(state);

    let rect = Rect::from_min_size(pos - Vec2::new(0.0, size.y * 0.5), size);
    let mut response = ctx.ui.interact(rect, ctx.id(), Sense::click_and_drag());
    if !label.implicit {
        if !label.name.trim().is_empty() {
            response = response.on_hover_text(label.name);
        }
    }
    let int = Interaction::new(ctx, response);

    let rounding = Rounding::none();

    ctx.shapes.push(Shape::rect_filled(rect, rounding, color));

    if int.hovered {
        let stroke = Stroke {
            width: 1.0,
            color: Color32::WHITE,
        };
        ctx.shapes.push(Shape::rect_stroke(rect, rounding, stroke));
    }
    int
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
    ios: &[(preset::IoLabel, bool)],
) -> Option<SubInteraction<usize>> {
    let mut int = None;

    let locs = calc_io_locs(tl, height, ios.len());

    for idx in 0..ios.len() {
        let (preset, state) = (ios[idx].0.clone(), ios[idx].1);

        ctx.push_id(|id| id.with(id_step).with(idx));
        let io_int = show_io(ctx, io_size, locs[idx], preset, state);
        ctx.pop_id();

        if !io_int.is_empty() {
            int = Some(io_int.sub(idx));
        }
    }
    int
}

pub struct DeviceInteraction {
    pub int: Interaction,
    pub input: Option<SubInteraction<usize>>,
    pub output: Option<SubInteraction<usize>>,
}
impl DeviceInteraction {
    #[inline(always)]
    pub fn from_int(int: Interaction) -> Self {
        Self {
            int,
            input: None,
            output: None,
        }
    }
    #[inline(always)]
    pub fn from_response(ctx: &Context, response: Response) -> Self {
        Self::from_int(Interaction::new(ctx, response))
    }
}

pub fn show_device(
    ctx: &mut Context,
    pos: Pos2,
    scene: &scene::DeviceData,
    preset: &preset::DeviceData,
) -> DeviceInteraction {
    let (width, height) = preset.size();

    let rect = Rect::from_min_size(pos, Vec2::new(width, height));

    // ## interact with rect
    let response = ctx.ui.interact(rect, ctx.id(), Sense::drag());
    let mut int = DeviceInteraction::from_response(ctx, response);

    // ## show rect
    let stroke = if int.int.hovered {
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

    let color = preset.color().unwrap_or(Color32::WHITE);
    let rounding = Rounding::same(3.0);

    ctx.shapes.push(Shape::rect_filled(rect, rounding, color));
    ctx.shapes.push(Shape::rect_stroke(rect, rounding, stroke));

    // ## show inputs
    let io_size = Vec2::new(20.0, 5.0);

    let preset_inputs: Vec<(preset::IoLabel, bool)> = (0..preset.num_inputs())
        .into_iter()
        .map(|i| (preset.get_input_label(i), scene.get_input(i)))
        .collect();

    let io_sub_int = show_ios(
        ctx,
        ".input",
        pos - Vec2::new(io_size.x, 0.0),
        height,
        io_size,
        &preset_inputs,
    );
    if let Some(sub_int) = io_sub_int {
        int.input = Some(sub_int);
    }

    // ## show outputs
    let preset_outputs: Vec<(preset::IoLabel, bool)> = (0..preset.num_outputs())
        .into_iter()
        .map(|i| (preset.get_output_label(i), scene.get_output(i)))
        .collect();

    let io_sub_int = show_ios(
        ctx,
        ".output",
        pos + Vec2::new(width, 0.0),
        height,
        io_size,
        &preset_outputs,
    );
    if let Some(sub_int) = io_sub_int {
        int.output = Some(sub_int);
    }
    int
}

pub fn show_link(ctx: &mut Context, state: bool, from: Pos2, to: Pos2) -> Interaction {
    let hovered = line_contains_point((from, to), 6.0, ctx.pointer_pos);

    let int = Interaction {
        drag: Vec2::ZERO,
        clicked: false,
        secondary_clicked: false,
        hovered,
        contains_pointer: hovered,
    };

    let color = state_color(state);
    let stroke = if hovered {
        Stroke { width: 6.0, color }
    } else {
        Stroke { width: 4.0, color }
    };
    ctx.shapes.push(Shape::line_segment([from, to], stroke));

    int
}

#[derive(Default)]
pub struct ShowSceneResult {
    pub device_int: Option<SubInteraction<SimId>>,
    pub finish_link: Option<LinkTarget<SimId>>,
    pub start_link: Option<LinkStart<SimId>>,
    pub toggle_input: Option<SimId>,
    pub toggle_device_input: Option<(SimId, usize)>,
}

#[derive(Debug)]
pub enum SceneInteraction {
    Input(SubInteraction<SimId>),
    Output(SubInteraction<SimId>),
    Device(SubInteraction<SimId>),
    DeviceInput(SimId, SubInteraction<usize>),
    DeviceOutput(SimId, SubInteraction<usize>),
}

pub fn show_scene(
    ctx: &mut Context,
    scene: &scene::Scene,
    presets: &Presets,
    dead_links: &mut Vec<(LinkStart<SimId>, usize)>,
) -> Option<SceneInteraction> {
    let mut int = None;

    for (device_id, scene_device) in &scene.devices {
        let device_preset = presets.get(&scene_device.preset).unwrap();

        ctx.push_id(|prev_id| prev_id.with(".device_").with(device_id));
        let device_int = show_device(ctx, scene_device.pos, &scene_device.data, device_preset);
        ctx.pop_id();

        if !device_int.int.is_empty() {
            int = Some(SceneInteraction::Device(device_int.int.sub(*device_id)));
        }
        if let Some(input_int) = device_int.input {
            int = Some(SceneInteraction::DeviceInput(*device_id, input_int));
        }
        if let Some(output_int) = device_int.output {
            int = Some(SceneInteraction::DeviceOutput(*device_id, output_int));
        }

        for output_idx in 0..scene_device.links.len() {
            for (link_idx, target) in scene_device.links[output_idx]
                .clone()
                .into_iter()
                .enumerate()
            {
                let state = scene_device.data.get_output(output_idx);

                let from = scene_device.output_locs[output_idx];

                let Some(target_loc) = scene.get_target_loc(ctx, target) else {
                	dead_links.push((LinkStart::DeviceOutput(*device_id, output_idx), link_idx));
                	continue
                };

                show_link(ctx, state, from, target_loc);
            }
        }
    }

    let input_locs = calc_io_unsized_locs(ctx.canvas_rect.min, scene.inputs.len(), EDITOR_IO_SP);

    for (input_idx, (input_id, input)) in scene.inputs.iter().enumerate() {
        ctx.push_id(|prev_id| prev_id.with(".input_").with(input_id));
        let io_int = show_io(
            ctx,
            EDITOR_IO_SIZE,
            input_locs[input_idx],
            input.label.clone(),
            input.state,
        );
        ctx.pop_id();

        if !io_int.is_empty() {
            int = Some(SceneInteraction::Input(io_int.sub(*input_id)));
        }

        let from = input_locs[input_idx] + Vec2::new(EDITOR_IO_SIZE.x, 0.0);
        for (link_idx, target) in input.links.clone().into_iter().rev().enumerate() {
            let Some(target_loc) = scene.get_target_loc(ctx, target) else {
            	dead_links.push((LinkStart::Input(*input_id), link_idx));
        		continue
        	};

            let _link_int = show_link(ctx, input.state, from, target_loc);
        }
    }

    let output_locs = calc_io_unsized_locs(
        Pos2::new(
            ctx.canvas_rect.max.x - EDITOR_IO_SIZE.x,
            ctx.canvas_rect.min.y,
        ),
        scene.outputs.len(),
        EDITOR_IO_SP,
    );
    for (idx, (id, output)) in scene.outputs.iter().enumerate() {
        ctx.push_id(|prev_id| prev_id.with(".output_").with(id));
        let io_int = show_io(
            ctx,
            EDITOR_IO_SIZE,
            output_locs[idx],
            output.label.clone(),
            output.state,
        );
        ctx.pop_id();

        if !io_int.is_empty() {
            int = Some(SceneInteraction::Output(io_int.sub(*id)));
        }
    }
    int
}
