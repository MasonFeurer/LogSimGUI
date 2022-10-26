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

pub struct Settings {
    pub io_col_w: f32,
    pub small_io_size: Vec2,
    pub large_io_size: Vec2,
    pub device_io_spacing: f32,
    pub device_io_sep: f32,
    pub device_name_font_size: f32,
    pub device_name_hover_text: bool,
}
impl Settings {
    pub fn default() -> Self {
        Self {
            io_col_w: 40.0,
            small_io_size: Vec2::new(15.0, 8.0),
            large_io_size: Vec2::new(30.0, 10.0),
            device_io_spacing: 5.0,
            device_io_sep: 10.0,
            device_name_font_size: 30.0,
            device_name_hover_text: false,
        }
    }
}

pub const DEVICE_NAME_CHAR_W: f32 = 18.0;
pub const IO_STROKE: Option<ShowStroke> = Some(ShowStroke {
    color: [Color::WHITE; 2],
    width: [0.0, 1.0],
});

pub fn io_presets_height(presets: &[preset::Io]) -> f32 {
    pub const DEVICE_IO_SPACING: f32 = 15.0;

    let mut height = 0.0;
    for _ in presets {
        height += DEVICE_IO_SPACING;
    }
    height + DEVICE_IO_SPACING
}

pub struct Context<'a, 'b> {
    pub rect: Rect,
    pub pointer_pos: Pos2,
    pub shapes: Vec<Shape>,
    pub ui: &'a Ui,
    pub id_stack: Vec<String>,
    pub id: Id,
    pub settings: &'b Settings,
}
impl<'a, 'b> Context<'a, 'b> {
    pub fn new(ui: &'a Ui, settings: &'b Settings, rect: Rect, pointer_pos: Pos2) -> Self {
        Self {
            rect,
            pointer_pos,
            shapes: Vec::new(),
            ui,
            id_stack: Vec::new(),
            id: Id::null(),
            settings,
        }
    }

    pub fn push_id<T: std::fmt::Display>(&mut self, v: T) {
        self.id_stack.push(format!("{}", v));
        self.id = Id::new(&self.id_stack);
    }
    pub fn pop_id(&mut self) {
        self.id_stack.pop();
        self.id = Id::new(&self.id_stack);
    }

    pub fn any_click(&self) -> bool {
        self.ui.input().pointer.any_click()
    }
}

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

    pub fn should_handle(&self) -> bool {
        self.drag != Vec2::ZERO || self.clicked || self.secondary_clicked || self.hovered
    }

    #[inline(always)]
    pub fn sub<T>(self, sub: T) -> SubInteraction<T> {
        SubInteraction { int: self, sub }
    }
}

pub struct SubInteraction<T> {
    pub int: Interaction,
    pub sub: T,
}

#[derive(Clone)]
pub struct ShowStroke {
    pub color: [Color; 2],
    pub width: [f32; 2],
}
#[derive(Clone)]
pub struct ShowRect<'a> {
    pub rect: Rect,
    pub rounding: f32,
    pub color: [Color; 2],
    pub stroke: Option<ShowStroke>,
    pub hover_text: Option<&'a str>,
}

pub fn show_rect(ctx: &mut Context, rect: ShowRect) -> Interaction {
    let ShowRect {
        rect,
        rounding,
        color,
        stroke,
        hover_text,
    } = rect;

    let mut response = ctx.ui.interact(rect, ctx.id, Sense::click_and_drag());
    if let Some(text) = hover_text && !text.trim().is_empty() {
		response = response.on_hover_text(text);
    }
    let int = Interaction::new(ctx, response);

    let color = if int.hovered { color[1] } else { color[0] };
    let rounding = Rounding::same(rounding);
    ctx.shapes.push(Shape::rect_filled(rect, rounding, color));

    if let Some(ShowStroke { color, width }) = stroke {
        let color = if int.hovered { color[1] } else { color[0] };
        let width = if int.hovered { width[1] } else { width[0] };
        let stroke = Stroke { width, color };
        ctx.shapes.push(Shape::rect_stroke(rect, rounding, stroke));
    }
    int
}
pub fn show_rects(ctx: &mut Context, rects: &[ShowRect]) -> Option<SubInteraction<usize>> {
    let mut result_int = None;

    for (idx, rect) in rects.iter().enumerate() {
        ctx.push_id(idx);
        let int = show_rect(ctx, rect.clone());
        if int.should_handle() {
            result_int = Some(int.sub(idx));
        }
        ctx.pop_id();
    }
    result_int
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

pub fn show_device(ctx: &mut Context, device: &scene::Device) -> DeviceInteraction {
    let (num_inputs, num_outputs) = (device.data.num_inputs(), device.data.num_outputs());

    let hover_text = if ctx.settings.device_name_hover_text {
        Some(device.vis.name.as_str())
    } else {
        None
    };
    let int = show_rect(
        ctx,
        ShowRect {
            rect: device.rect(),
            rounding: 5.0,
            color: [device.vis.color; 2],
            stroke: Some(ShowStroke {
                color: [Color32::from_rgb(200, 200, 200); 2],
                width: [1.0, 3.0],
            }),
            hover_text,
        },
    );

    let name_color = if Rgba::from(device.vis.color).intensity() > 0.5 {
        Color::BLACK
    } else {
        Color::WHITE
    };
    ctx.shapes.push(Shape::text(
        &ctx.ui.fonts(),
        device.pos + device.size * 0.5,
        Align2::CENTER_CENTER,
        &device.vis.name,
        FontId::monospace(ctx.settings.device_name_font_size),
        name_color,
    ));
    let mut int = DeviceInteraction::from_int(int);

    // ## show inputs
    let mut show_inputs = Vec::with_capacity(num_inputs);
    for i in 0..num_inputs {
        let state = device.data.get_input(i).unwrap();
        let preset = &device.input_presets[i];
        let hover_text = (!preset.implicit).then(|| preset.name.as_str());

        show_inputs.push(ShowRect {
            rect: device.input_defs[i].rect(ctx.settings),
            rounding: 0.0,
            color: [state_color(state); 2],
            stroke: IO_STROKE,
            hover_text,
        });
    }

    ctx.push_id("_input_");
    let inputs_sub_int = show_rects(ctx, &show_inputs);
    ctx.pop_id();

    if let Some(sub_int) = inputs_sub_int {
        int.input = Some(sub_int);
    }

    // ## show inputs
    let mut show_outputs = Vec::with_capacity(num_outputs);
    for i in 0..num_outputs {
        let state = device.data.get_output(i).unwrap();
        let preset = &device.input_presets[i];
        let hover_text = (!preset.implicit).then(|| preset.name.as_str());

        show_outputs.push(ShowRect {
            rect: device.output_defs[i].rect(ctx.settings),
            rounding: 0.0,
            color: [state_color(state); 2],
            stroke: IO_STROKE,
            hover_text,
        });
    }

    ctx.push_id("_output_");
    let outputs_sub_int = show_rects(ctx, &show_outputs);
    ctx.pop_id();

    if let Some(sub_int) = outputs_sub_int {
        int.output = Some(sub_int);
    }
    int
}

pub fn show_link(ctx: &mut Context, state: bool, from: Pos2, to: Pos2) -> Interaction {
    let hovered = line_contains_point((from, to), 10.0, ctx.pointer_pos);

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
    dead_links: &mut Vec<(LinkStart<IntId>, usize)>,
) -> Option<SceneInteraction> {
    ctx.push_id("scene");
    let mut int = None;

    // IO COLUMN BARS
    let input_x = ctx.rect.min.x + ctx.settings.io_col_w;
    let output_x = ctx.rect.max.x - ctx.settings.io_col_w;
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
    ctx.push_id("_device_");
    for (device_id, device) in &scene.devices {
        // DRAW DEVICE
        ctx.push_id(device_id.0);
        let device_int = show_device(ctx, device);
        ctx.pop_id();

        // HANDLE DEVICE INTERACTIONS
        if device_int.int.should_handle() {
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

                let link_start = device
                    .output_defs
                    .get(output_idx)
                    .unwrap()
                    .tip_loc(ctx.settings);

                let Some(target_def) = scene.get_link_target_def(target) else {
                	dead_links.push((LinkStart::DeviceOutput(*device_id, output_idx), link_idx));
                	continue
                };

                show_link(ctx, state, link_start, target_def.tip_loc(ctx.settings));
            }
        }
    }
    ctx.pop_id();

    // DRAW SCENE INPUTS
    ctx.push_id("_input_");
    for (id, input) in &scene.inputs {
        ctx.push_id(id.0);

        // DRAW INPUT
        let def = scene.get_input_def(input);
        let io_int = show_rect(
            ctx,
            ShowRect {
                rect: def.rect(ctx.settings),
                rounding: 0.0,
                color: [state_color(input.state); 2],
                stroke: IO_STROKE,
                hover_text: None,
            },
        );
        ctx.pop_id();

        // HANDLE INPUT INTERACTION
        if io_int.should_handle() {
            int = Some(SceneInteraction::Input(io_int.sub(*id)));
        }

        // DRAW LINKS FROM INPUT
        let link_start = def.tip_loc(ctx.settings);
        for (link_idx, target) in input.links.iter().rev().enumerate() {
            let Some(target_def) = scene.get_link_target_def(&target.wrap()) else {
            	dead_links.push((LinkStart::Input(*id), link_idx));
        		continue
        	};

            show_link(
                ctx,
                input.state,
                link_start,
                target_def.tip_loc(ctx.settings),
            );
        }
    }
    ctx.pop_id();

    // DRAW SCENE OUTPUTS
    ctx.push_id("_output_");
    for (id, output) in &scene.outputs {
        let def = scene.get_output_def(output);

        // DRAW OUTPUT
        ctx.push_id(id.0);
        let io_int = show_rect(
            ctx,
            ShowRect {
                rect: def.rect(ctx.settings),
                rounding: 0.0,
                color: [state_color(output.state); 2],
                stroke: IO_STROKE,
                hover_text: None,
            },
        );
        ctx.pop_id();

        // HANDLE OUTPUT INTERACTION
        if io_int.should_handle() {
            int = Some(SceneInteraction::Output(io_int.sub(*id)));
        }
    }
    ctx.pop_id();
    ctx.pop_id();
    int
}
