use crate::*;
use eframe::egui::*;

// TODO read tutuorial on how this thing works :>
// http://www.sunshine2k.de/coding/java/PointOnLine/PointOnLine.html
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

#[inline(always)]
pub fn state_color(state: bool) -> Color32 {
    if state {
        Color32::from_rgb(255, 0, 0)
    } else {
        Color32::from_rgb(150, 150, 150)
    }
}

pub struct Context<'a> {
    pub rect: Rect,
    pub pointer_pos: Pos2,
    pub shapes: Vec<Shape>,
    pub ui: &'a Ui,
    pub id_stack: Vec<Id>,
}
impl<'a> Context<'a> {
    pub fn new(ui: &'a Ui, first_id: Id, rect: Rect, pointer_pos: Pos2) -> Self {
        Self {
            rect,
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

fn show_io(ctx: &mut Context, def: &IoDef, preset: &preset::Io, state: bool) -> Interaction {
    let color = state_color(state);
    let rect = def.rect();

    let mut response = ctx.ui.interact(rect, ctx.id(), Sense::click_and_drag());
    if !preset.implicit {
        if !preset.name.trim().is_empty() {
            response = response.on_hover_text(&preset.name);
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

pub fn show_ios(
    ctx: &mut Context,
    id_step: &str,
    ios: &[(IoDef, preset::Io, bool)],
) -> Option<SubInteraction<usize>> {
    let mut int = None;

    for idx in 0..ios.len() {
        let (def, preset, state) = &ios[idx];

        ctx.push_id(|id| id.with(id_step).with(idx));
        let io_int = show_io(ctx, def, preset, *state);
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
    scene: &scene::Device,
    preset: &preset::Preset,
) -> DeviceInteraction {
    let size = scene.data.size().to_vec2();
    let rect = Rect::from_min_size(pos - size * 0.5, size);

    // ## interact with rect
    let response = ctx.ui.interact(rect, ctx.id(), Sense::drag());
    let mut int = DeviceInteraction::from_response(ctx, response);

    // ## show rect
    let stroke = if int.int.hovered {
        Stroke {
            width: 3.0,
            color: Color32::from_rgb(200, 200, 200),
        }
    } else {
        Stroke {
            width: 3.0,
            color: Color32::from_rgb(150, 150, 150),
        }
    };

    let color = preset.color();
    let rounding = Rounding::same(3.0);

    ctx.shapes.push(Shape::rect_filled(rect, rounding, *color));
    ctx.shapes.push(Shape::rect_stroke(rect, rounding, stroke));

    // ## show inputs
    let inputs_arg: Vec<_> = (0..preset.num_inputs())
        .into_iter()
        .map(|i| {
            (
                scene.get_input_def(i).unwrap(),
                preset.get_input(i).unwrap().clone(),
                scene.data.get_input(i).unwrap(),
            )
        })
        .collect();

    let io_sub_int = show_ios(ctx, ".input", &inputs_arg);
    if let Some(sub_int) = io_sub_int {
        int.input = Some(sub_int);
    }

    // ## show outputs
    let outputs_arg: Vec<_> = (0..preset.num_outputs())
        .into_iter()
        .map(|i| {
            (
                scene.get_output_def(i).unwrap(),
                preset.get_output(i).unwrap().clone(),
                scene.data.get_output(i).unwrap(),
            )
        })
        .collect();

    let io_sub_int = show_ios(ctx, ".output", &outputs_arg);
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
    pub device_int: Option<SubInteraction<IntId>>,
    pub finish_link: Option<LinkTarget<IntId>>,
    pub start_link: Option<LinkStart<IntId>>,
    pub toggle_input: Option<IntId>,
    pub toggle_device_input: Option<(IntId, usize)>,
}

#[derive(Debug)]
pub enum SceneInteraction {
    Input(SubInteraction<IntId>),
    Output(SubInteraction<IntId>),
    Device(SubInteraction<IntId>),
    DeviceInput(IntId, SubInteraction<usize>),
    DeviceOutput(IntId, SubInteraction<usize>),
}

pub fn show_scene(
    ctx: &mut Context,
    scene: &scene::Scene,
    presets: &preset::Presets,
    dead_links: &mut Vec<(LinkStart<IntId>, usize)>,
) -> Option<SceneInteraction> {
    let mut int = None;

    // IO COLUMN BARS
    let input_x = ctx.rect.min.x + scene::IO_COL_W;
    let output_x = ctx.rect.max.x - scene::IO_COL_W;
    let stroke = Stroke {
        width: 2.0,
        color: Color32::from_gray(100),
    };
    ctx.shapes.push(Shape::line_segment(
        [
            Pos2::new(input_x, ctx.rect.min.y),
            Pos2::new(input_x, ctx.rect.max.y),
        ],
        stroke,
    ));
    ctx.shapes.push(Shape::line_segment(
        [
            Pos2::new(output_x, ctx.rect.min.y),
            Pos2::new(output_x, ctx.rect.max.y),
        ],
        stroke,
    ));

    // DRAW DEVICES
    for (device_id, device) in &scene.devices {
        let device_preset = presets.get_preset(device.preset).unwrap();

        // DRAW DEVICE
        ctx.push_id(|prev_id| prev_id.with(".device_").with(device_id));
        let device_int = show_device(ctx, device.pos, device, device_preset);
        ctx.pop_id();

        // HANDLE DEVICE INTERACTIONS
        if !device_int.int.is_empty() {
            int = Some(SceneInteraction::Device(device_int.int.sub(*device_id)));
        }
        if let Some(input_int) = device_int.input {
            int = Some(SceneInteraction::DeviceInput(*device_id, input_int));
        }
        if let Some(output_int) = device_int.output {
            int = Some(SceneInteraction::DeviceOutput(*device_id, output_int));
        }

        // DRAW LINKS FROM DEVICE OUTPUTS
        for output_idx in 0..device.links.len() {
            for (link_idx, target) in device.links[output_idx].iter().enumerate() {
                let state = device.data.get_output(output_idx).unwrap();

                let link_start = device.get_output_def(output_idx).unwrap().tip_loc();

                let Some(target_def) = scene.get_link_target_def(target) else {
                	dead_links.push((LinkStart::DeviceOutput(*device_id, output_idx), link_idx));
                	continue
                };

                show_link(ctx, state, link_start, target_def.tip_loc());
            }
        }
    }

    // DRAW SCENE INPUTS
    for (id, WithLinks { item: input, links }) in &scene.inputs {
        ctx.push_id(|prev_id| prev_id.with(".input_").with(id));

        // DRAW INPUT
        let def = scene.get_input_def(input);
        let io_int = show_io(ctx, &def, &input.preset, input.state);
        ctx.pop_id();

        // HANDLE INPUT INTERACTION
        if !io_int.is_empty() {
            int = Some(SceneInteraction::Input(io_int.sub(*id)));
        }

        // DRAW LINKS FROM INPUT
        let link_start = def.tip_loc();
        for (link_idx, target) in links.iter().rev().enumerate() {
            let Some(target_def) = scene.get_link_target_def(target) else {
            	dead_links.push((LinkStart::Input(*id), link_idx));
        		continue
        	};

            show_link(ctx, input.state, link_start, target_def.tip_loc());
        }
    }

    // DRAW SCENE OUTPUTS
    for (id, output) in &scene.outputs {
        let def = scene.get_output_def(output);

        // DRAW OUTPUT
        ctx.push_id(|prev_id| prev_id.with(".output_").with(id));
        let io_int = show_io(ctx, &def, &output.preset, output.state);
        ctx.pop_id();

        // HANDLE OUTPUT INTERACTION
        if !io_int.is_empty() {
            int = Some(SceneInteraction::Output(io_int.sub(*id)));
        }
    }
    int
}
