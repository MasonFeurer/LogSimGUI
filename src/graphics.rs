use crate::app::CreateLinks;
use crate::board::{Board, BoardItem, IoSel};
use crate::presets::DevicePreset;
use crate::settings::Settings;
use crate::*;
use egui::*;

const ON_V: u8 = 200;
const OFF_V: u8 = 100;

#[rustfmt::skip]
pub const LINK_COLORS: &[[Color32; 2]] = &[
    [Color32::from_rgb(OFF_V, 0, 0), Color32::from_rgb(ON_V, 0, 0)],
    [Color32::from_rgb(OFF_V, OFF_V, OFF_V), Color32::from_rgb(ON_V, ON_V, ON_V)],
    [Color32::from_rgb(0, OFF_V, 0), Color32::from_rgb(0, ON_V, 0)],
    [Color32::from_rgb(0, 0, OFF_V), Color32::from_rgb(0, 0, ON_V)],
    [Color32::from_rgb(OFF_V, OFF_V, 0), Color32::from_rgb(ON_V, ON_V, 0)],
    [Color32::from_rgb(OFF_V, 0, OFF_V), Color32::from_rgb(ON_V, 0, ON_V)],
    [Color32::from_rgb(0, OFF_V, OFF_V), Color32::from_rgb(0, ON_V, ON_V)],
];
pub const NUM_LINK_COLORS: usize = LINK_COLORS.len();

pub struct Spread {
    pub count: usize,
    pub counter: usize,
    pub value: f32,
    pub step: f32,
}
impl Spread {
    pub fn new(min: f32, max: f32, count: usize) -> Self {
        let step = (max - min) / (count + 1) as f32;
        let value = min + step;
        Self {
            count,
            counter: 0,
            value,
            step,
        }
    }
}
impl Iterator for Spread {
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        if self.counter >= self.count {
            return None;
        }
        let result = self.value;
        self.value += self.step;
        self.counter += 1;
        Some(result)
    }

    /// note: Doesn't update the iterator
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        if self.counter + n >= self.count {
            return None;
        }
        Some(self.value + self.step * n as f32)
    }
}

pub struct VerticalSpread(pub f32, pub Spread);
impl Iterator for VerticalSpread {
    type Item = Pos2;
    fn next(&mut self) -> Option<Self::Item> {
        self.1.next().map(|y| pos2(self.0, y))
    }

    /// note: Doesn't update the iterator
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.1.nth(n).map(|y| pos2(self.0, y))
    }
}

#[derive(Clone, Copy)]
pub struct Transform {
    pub scale: f32,
    pub offset: [f32; 2],
}
impl Transform {
    pub fn identity() -> Self {
        Self {
            scale: 1.0,
            offset: [0.0; 2],
        }
    }
}
impl std::ops::Mul<Pos2> for Transform {
    type Output = Pos2;
    fn mul(self, pos: Pos2) -> Pos2 {
        Pos2 {
            x: pos.x * self.scale + self.offset[0],
            y: pos.y * self.scale + self.offset[1],
        }
    }
}
impl std::ops::Mul<Vec2> for Transform {
    type Output = Vec2;
    fn mul(self, v: Vec2) -> Vec2 {
        v * self.scale
    }
}
impl std::ops::Mul<f32> for Transform {
    type Output = f32;
    fn mul(self, v: f32) -> f32 {
        v * self.scale
    }
}

#[derive(Clone)]
pub struct View {
    pub origin: Pos2,
    pub offset: Vec2,
    pub zoom: f32,
}
impl View {
    pub fn default() -> Self {
        Self {
            origin: Pos2::ZERO,
            offset: Vec2::ZERO,
            zoom: 100.0,
        }
    }

    pub fn zoom(&mut self, delta: f32, pos: Pos2) {
        let xs = (pos.x - self.offset.x) / self.scale();
        let ys = (pos.y - self.offset.y) / self.scale();
        self.zoom *= delta;

        const MIN_ZOOM: f32 = 10.0;
        const MAX_ZOOM: f32 = 400.0;

        self.zoom = f32::max(self.zoom, MIN_ZOOM);
        self.zoom = f32::min(self.zoom, MAX_ZOOM);

        self.offset.x = pos.x - xs * self.scale();
        self.offset.y = pos.y - ys * self.scale();
    }
    pub fn drag(&mut self, drag: Vec2) {
        self.offset += drag;
    }

    #[inline(always)]
    pub fn scale(&self) -> f32 {
        self.zoom / 100.0
    }

    pub fn create_transform(&self) -> Transform {
        let scale = self.scale();
        Transform {
            scale,
            offset: [
                self.origin.x * scale + self.origin.x + self.offset.x,
                self.origin.y * scale + self.origin.y + self.offset.y,
            ],
        }
    }
    pub fn create_inv_transform(&self) -> Transform {
        let scale = self.scale();
        Transform {
            scale: 1.0 / scale,
            offset: [
                -self.offset.x / scale - self.origin.x / scale + self.origin.x,
                -self.offset.y / scale - self.origin.y / scale + self.origin.y,
            ],
        }
    }
}

// http://www.sunshine2k.de/coding/java/PointOnLine/PointOnLine.html
pub fn project_point_onto_line(p: Pos2, line: (Pos2, Pos2)) -> Pos2 {
    let (v1, v2) = line;

    // get dot product of e1, e2
    let e1 = pos2(v2.x - v1.x, v2.y - v1.y);
    let e2 = pos2(p.x - v1.x, p.y - v1.y);
    let dot = e1.x * e2.x + e1.y * e2.y;

    // get squared length of e1
    let len_sq = e1.x * e1.x + e1.y * e1.y;

    let result_x = v1.x + (dot * e1.x) / len_sq;
    let result_y = v1.y + (dot * e1.y) / len_sq;
    pos2(result_x, result_y)
}
pub fn line_contains_point(line: (Pos2, Pos2), width: f32, point: Pos2) -> bool {
    let max_dist_sq = width * width;

    let projected = project_point_onto_line(point, line);

    let pp = projected - point;
    let dist_sq = (pp.x * pp.x + pp.y * pp.y).abs();

    let line_min_x = line.0.x.min(line.1.x);
    let line_max_x = line.0.x.max(line.1.x);
    let line_min_y = line.0.y.min(line.1.y);
    let line_max_y = line.0.y.max(line.1.y);

    dist_sq <= max_dist_sq
        && projected.x >= line_min_x
        && projected.x <= line_max_x
        && projected.y >= line_min_y
        && projected.y <= line_max_y
}

#[derive(Clone, Copy, Default)]
pub struct ShowStroke {
    pub color: [Color32; 2],
    pub width: [f32; 2],
}

pub struct Graphics<'a> {
    pub ctx: &'a Context,
    pub transform: Transform,
    pub pointer_pos: Pos2,
    shapes: Vec<Shape>,
}
impl<'a> Graphics<'a> {
    pub fn new(ctx: &'a Context, transform: Transform, pointer_pos: Pos2) -> Self {
        Self {
            ctx,
            transform,
            pointer_pos,
            shapes: Vec::new(),
        }
    }
    pub fn finish(self) -> Vec<Shape> {
        self.shapes
    }

    pub fn rect(
        &mut self,
        rect: Rect,
        rounding: f32,
        color: [Color32; 2],
        stroke: Option<ShowStroke>,
    ) -> bool {
        let rect = Rect {
            min: self.transform * rect.min,
            max: self.transform * rect.max,
        };

        let hovered = rect.contains(self.pointer_pos);

        let color = if hovered { color[1] } else { color[0] };
        let rounding = Rounding::same(rounding);
        self.shapes.push(Shape::rect_filled(rect, rounding, color));

        if let Some(ShowStroke { color, width }) = stroke {
            let color = if hovered { color[1] } else { color[0] };
            let width = if hovered { width[1] } else { width[0] };
            let stroke = Stroke { width, color };
            self.shapes.push(Shape::rect_stroke(rect, rounding, stroke));
        }
        hovered
    }

    pub fn rect_stroke(&mut self, rect: Rect, rounding: f32, stroke: Stroke) {
        let rect = Rect {
            min: self.transform * rect.min,
            max: self.transform * rect.max,
        };
        let rounding = Rounding::same(rounding);
        self.shapes.push(Shape::rect_stroke(rect, rounding, stroke));
    }

    pub fn line(&mut self, from: Pos2, to: Pos2, width: f32, stroke: ShowStroke) -> bool {
        let (from, to, width) = (
            self.transform * from,
            self.transform * to,
            self.transform * width,
        );

        let hovered = line_contains_point((from, to), width, self.pointer_pos);

        let ShowStroke { color, width } = stroke;
        let color = if hovered { color[1] } else { color[0] };
        let width = if hovered { width[1] } else { width[0] };
        let stroke = Stroke { width, color };

        self.shapes.push(Shape::line_segment([from, to], stroke));
        hovered
    }

    pub fn text(&mut self, pos: Pos2, size: f32, text: &str, color: Color32, align: Align2) {
        let (pos, size) = (self.transform * pos, self.transform * size);
        self.shapes.push(Shape::text(
            &self.ctx.fonts(),
            pos,
            align,
            text,
            FontId::proportional(size),
            color,
        ));
    }

    pub fn circle(
        &mut self,
        center: Pos2,
        radius: f32,
        color: [Color32; 2],
        stroke: Option<ShowStroke>,
    ) -> bool {
        let (center, radius) = (self.transform * center, self.transform * radius);
        let rect = Rect {
            min: center - Vec2::splat(radius),
            max: center + Vec2::splat(radius),
        };
        let hovered = rect.contains(self.pointer_pos);

        let color = if hovered { color[1] } else { color[0] };
        self.shapes
            .push(Shape::circle_filled(center, radius, color));

        if let Some(ShowStroke { color, width }) = stroke {
            let color = if hovered { color[1] } else { color[0] };
            let width = if hovered { width[1] } else { width[0] };
            let stroke = Stroke { width, color };
            self.shapes
                .push(Shape::circle_stroke(center, radius, stroke));
        }
        hovered
    }
}

// ---- SCENE GRAPHICS START HERE ----
pub fn device_output_locs(settings: &Settings, rect: Rect, count: usize) -> VerticalSpread {
    let x = rect.max.x + settings.device_pin_size * 0.5;
    VerticalSpread(x, Spread::new(rect.min.y, rect.max.y, count))
}
pub fn device_input_locs(settings: &Settings, rect: Rect, count: usize) -> VerticalSpread {
    let x = rect.min.x - settings.device_pin_size * 0.5;
    VerticalSpread(x, Spread::new(rect.min.y, rect.max.y, count))
}

pub fn link_target_pos(
    settings: &Settings,
    board: &Board,
    target: LinkTarget<u64>,
) -> Option<Pos2> {
    match target {
        LinkTarget::Output(id) => Some(Pos2 {
            x: board.rect.max.x - settings.board_io_col_w - settings.board_io_pin_size * 0.5,
            y: board.outputs.get(&id)?.io.y_pos,
        }),
        LinkTarget::DeviceInput(device_id, input) => {
            let device = board.devices.get(&device_id)?;
            let rect = Rect::from_min_size(device.pos, device_size(device, settings));
            device_input_locs(settings, rect, device.num_inputs()).nth(input)
        }
    }
}
pub fn link_start_pos(settings: &Settings, board: &Board, start: LinkStart<u64>) -> Option<Pos2> {
    match start {
        LinkStart::Input(id) => Some(Pos2 {
            x: board.rect.min.x + settings.board_io_col_w + settings.board_io_pin_size * 0.5,
            y: board.inputs.get(&id)?.io.y_pos,
        }),
        LinkStart::DeviceOutput(device_id, output) => {
            let device = board.devices.get(&device_id)?;
            let rect = Rect::from_min_size(device.pos, device_size(device, settings));
            device_output_locs(settings, rect, device.num_outputs()).nth(output)
        }
    }
}

pub fn calc_device_size(num_inputs: usize, num_outputs: usize, min_pin_spacing: f32) -> Vec2 {
    let num_io = num_inputs.max(num_outputs);
    let h = (num_io + 1) as f32 * min_pin_spacing;
    let w = h.max(70.0);
    vec2(w, h)
}
pub fn device_size(device: &board::Device, settings: &Settings) -> Vec2 {
    calc_device_size(
        device.num_inputs(),
        device.num_outputs(),
        settings.device_min_pin_spacing,
    )
}

pub const GROUP_COLOR: Color32 = Color32::from_gray(120);
pub const GROUP_HEADER_SIZE: f32 = 16.0;
pub const BULB_STROKE: Option<ShowStroke> = Some(ShowStroke {
    width: [0.0, 1.0],
    color: [Color32::from_gray(200); 2],
});

pub fn show_link(
    g: &mut Graphics,
    width: f32,
    state: bool,
    color: usize,
    from: Pos2,
    to: Pos2,
    anchors: &[Pos2],
) -> bool {
    let color = LINK_COLORS[color][state as usize];
    let stroke = ShowStroke {
        color: [color; 2],
        width: [width, width + 2.0],
    };
    let mut hovered = false;
    let mut points = vec![from];
    points.extend(anchors);
    points.push(to);

    for idx in 1..points.len() {
        let (from, to) = (points[idx - 1], points[idx]);
        if g.line(from, to, width, stroke) {
            hovered = true;
        }
    }
    hovered
}
pub fn show_pin(g: &mut Graphics, pos: Pos2, size: f32, color: Color32, name: &str) -> bool {
    let hovered = g.circle(
        pos,
        size,
        [color; 2],
        Some(ShowStroke {
            color: [Color32::WHITE; 2],
            width: [0.0, 1.0],
        }),
    );
    if !name.trim().is_empty() {
        // TODO show name popup
    }
    hovered
}

#[derive(Clone, Copy)]
pub enum DeviceItem {
    Device,
    Input(usize),
    Output(usize),
}
pub struct ShowDevice<'a> {
    inputs: BitField,
    outputs: BitField,
    preset: &'a DevicePreset,
    show_id: Option<u64>,
    alpha: Option<u8>,
}
pub fn show_device(
    g: &mut Graphics,
    settings: &Settings,
    pos: Pos2,
    size: Vec2,
    device: ShowDevice,
) -> Option<DeviceItem> {
    let color = {
        let [r, g, b, a]: [u8; 4] = device.preset.color.into();
        let a = device.alpha.unwrap_or(a);
        Color32::from_rgba_premultiplied(r, g, b, a)
    };
    let rect = Rect::from_min_size(pos, size);

    // --- Show rectangle ---
    let hovered = g.rect(
        rect,
        5.0,
        [color; 2],
        Some(ShowStroke {
            color: [Color32::from_rgb(200, 200, 200); 2],
            width: [1.0, 3.0],
        }),
    );
    let mut hovered = hovered.then(|| DeviceItem::Device);

    // --- Show name ---
    let name_color = match Rgba::from(color).intensity() {
        v if v > 0.5 => Color32::BLACK,
        _ => Color32::WHITE,
    };
    g.text(
        pos + size * 0.5,
        settings.device_name_size,
        &device.preset.name,
        name_color,
        Align2::CENTER_CENTER,
    );

    // --- Show input and output pins
    let input_locs = device_input_locs(settings, rect, device.inputs.len);
    for (index, pos) in input_locs.enumerate() {
        let state = device.inputs.get(index);
        let color = settings.pin_color(state);
        let name = &device.preset.data.input_names()[index];
        if show_pin(g, pos, settings.device_pin_size, color, name) {
            hovered = Some(DeviceItem::Input(index));
        }
    }
    let output_locs = device_output_locs(settings, rect, device.outputs.len);
    for (index, pos) in output_locs.enumerate() {
        let state = device.outputs.get(index);
        let color = settings.pin_color(state);
        let name = &device.preset.data.output_names()[index];
        if show_pin(g, pos, settings.device_pin_size, color, name) {
            hovered = Some(DeviceItem::Output(index));
        }
    }

    // --- Show ID ---
    if let Some(id) = device.show_id {
        g.text(
            pos + vec2(size.x * 0.5, -10.0),
            10.0,
            &format!("{}", id),
            Color32::from_gray(120),
            Align2::CENTER_CENTER,
        );
    }
    hovered
}

pub fn show_preset_device(g: &mut Graphics, settings: &Settings, pos: Pos2, preset: &DevicePreset) {
    let size = calc_device_size(
        preset.data.num_inputs(),
        preset.data.num_outputs(),
        settings.device_min_pin_spacing,
    );
    let show = ShowDevice {
        inputs: BitField::empty(preset.data.num_inputs()),
        outputs: BitField::empty(preset.data.num_outputs()),
        preset,
        show_id: None,
        alpha: Some(255 / 5),
    };
    show_device(g, settings, pos, size, show);
}

pub fn show_board_device(
    g: &mut Graphics,
    settings: &Settings,
    device: &board::Device,
    preset: &DevicePreset,
    show_id: Option<u64>,
) -> Option<DeviceItem> {
    let show = ShowDevice {
        inputs: device.data.input(),
        outputs: device.data.output(),
        preset,
        show_id,
        alpha: None,
    };
    let size = device_size(device, settings);
    show_device(g, settings, device.pos, size, show)
}

pub fn show_board(
    g: &mut Graphics,
    settings: &Settings,
    board: &board::Board,
    library: &Library,
    show_device_ids: bool,
) -> Option<BoardItem> {
    let mut result: Option<BoardItem> = None;
    let rect = board.rect;
    if rect.contains(g.pointer_pos) {
        result = Some(BoardItem::Board);
    }

    g.rect(rect, 5.0, [settings.board_color; 2], None);

    // --- Show links from devices ---
    for (device_id, device) in &board.devices {
        let size = device_size(device, settings);
        let device_rect = Rect::from_min_size(device.pos, size);

        let output_locs = device_output_locs(settings, device_rect, device.num_outputs());
        for (output_idx, output_loc) in output_locs.enumerate() {
            for (link_idx, link) in device.links[output_idx].iter().enumerate() {
                let state = device.data.output().get(output_idx);

                let target_pos = link_target_pos(settings, board, link.target).unwrap();
                let hovered = show_link(
                    g,
                    settings.link_width,
                    state,
                    link.color,
                    output_loc,
                    target_pos,
                    &link.anchors,
                );
                if hovered {
                    result = Some(BoardItem::DeviceOutputLink(
                        *device_id, output_idx, link_idx,
                    ));
                }
            }
        }
    }

    // --- Show links from inputs ---
    for (input_id, input) in &board.inputs {
        let start_pos = Pos2 {
            x: rect.min.x + settings.board_io_col_w + settings.board_io_pin_size,
            y: input.io.y_pos,
        };
        for (link_idx, link) in input.links.iter().enumerate() {
            let target_pos = link_target_pos(settings, board, link.target).unwrap();
            let hovered = show_link(
                g,
                settings.link_width,
                input.io.state,
                link.color,
                start_pos,
                target_pos,
                &link.anchors,
            );
            if hovered {
                result = Some(BoardItem::InputLink(*input_id, link_idx));
            }
        }
    }

    // --- Show devices ---
    for (device_id, device) in &board.devices {
        let show_id = show_device_ids.then(|| *device_id);
        let preset = library.get_preset(&device.preset).unwrap();
        let device_hovered = show_board_device(g, settings, device, preset, show_id);

        if let Some(device_item) = device_hovered {
            let board_item = match device_item {
                DeviceItem::Device => BoardItem::Device(*device_id),
                DeviceItem::Input(input) => BoardItem::DeviceInput(*device_id, input),
                DeviceItem::Output(output) => BoardItem::DeviceOutput(*device_id, output),
            };
            result = Some(board_item);
        }
    }

    // --- Show input and output columns ---
    let margin = Vec2::splat(5.0);
    let col_w = settings.board_io_col_w;
    let col_size = vec2(col_w, rect.height()) - margin * 2.0;
    let input_rect = Rect::from_min_size(rect.min + margin, col_size);
    let output_rect = Rect::from_min_size(rect.max - margin - col_size, col_size);
    let color = [settings.board_io_col_color; 2];

    if g.rect(input_rect, 5.0, color, None) {
        result = Some(BoardItem::InputCol);
    }
    if g.rect(output_rect, 5.0, color, None) {
        result = Some(BoardItem::OutputCol);
    }

    let show_io_bulb = move |g: &mut Graphics, state: bool, x: f32, y: f32| -> bool {
        g.circle(
            pos2(x, y),
            col_w * 0.5,
            [settings.pin_color(state); 2],
            BULB_STROKE,
        )
    };
    let show_io_decor = move |g: &mut Graphics, x: f32, y: f32| {
        let (x0, x1) = (x - col_w * 0.5, x + col_w * 0.5);
        let (y0, y1) = (y - col_w * 0.5, y + col_w * 0.5);
        let stroke = ShowStroke {
            color: [settings.board_io_col_color; 2],
            width: [4.0; 2],
        };
        g.line(pos2(x0, y0), pos2(x0, y1), 0.0, stroke);
        g.line(pos2(x1, y0), pos2(x1, y1), 0.0, stroke);
    };

    // --- Show input pins ---
    let pin_size = settings.board_io_pin_size;
    for (input_id, input) in &board.inputs {
        let input = &input.io;
        let (x, y) = (rect.min.x + col_w * 0.5, input.y_pos);

        let pin_pos = pos2(rect.min.x + col_w + pin_size * 0.5, y);
        let color = settings.pin_color(input.state);
        if show_pin(g, pin_pos, pin_size, color, &input.name) {
            result = Some(BoardItem::InputPin(*input_id));
        }
        if input.group_member.is_some() {
            show_io_decor(g, x, y);
        }
        if show_io_bulb(g, input.state, x, y) {
            result = Some(BoardItem::InputBulb(*input_id));
        }
    }

    // --- Show input group headers ---
    for (_, group) in &board.input_groups {
        let center = rect.min.x + col_w * 0.5;
        let text = group.display_value(group.field(board, IoSel::Input));
        let top_member_y = board.inputs.get(&group.members[0]).unwrap().io.y_pos;
        g.text(
            pos2(center, top_member_y - settings.board_io_col_w * 0.5),
            10.0,
            &text,
            Color32::WHITE,
            Align2::CENTER_BOTTOM,
        );
    }

    // --- Show output pins ---
    for (output_id, output) in &board.outputs {
        let output = &output.io;
        let (x, y) = (rect.max.x - col_w * 0.5, output.y_pos);

        let pin_pos = pos2(rect.max.x - col_w - pin_size * 0.5, y);
        let color = settings.pin_color(output.state);
        if show_pin(g, pin_pos, pin_size, color, &output.name) {
            result = Some(BoardItem::OutputPin(*output_id));
        }
        if output.group_member.is_some() {
            show_io_decor(g, x, y);
        }
        if show_io_bulb(g, output.state, x, y) {
            result = Some(BoardItem::OutputBulb(*output_id));
        }
    }

    // --- Show output group headers ---
    for (_group_id, _group) in &board.output_groups {}
    result
}

pub fn outline_devices(g: &mut Graphics, settings: &Settings, devices: &[u64], board: &Board) {
    for device_id in devices {
        let device = board.devices.get(device_id).unwrap();
        let (pos, size) = (device.pos, device_size(device, settings));
        let rect = Rect::from_min_size(pos, size);
        g.rect_stroke(rect, 2.0, Stroke::new(2.0, Color32::WHITE));
    }
}

pub fn show_create_links(
    g: &mut Graphics,
    settings: &Settings,
    board: &Board,
    links: &CreateLinks,
    target: Pos2,
) {
    let width = settings.link_width;
    let color = links.color;

    for idx in (0..links.starts.len()).rev() {
        let link_start = links.starts[idx].clone();
        let state = board.link_start_state(link_start).unwrap();
        let pos = link_start_pos(settings, board, link_start).unwrap();
        show_link(g, width, state, color, pos, target, &links.anchors);
    }
}

pub fn show_held_presets(
    g: &mut Graphics,
    settings: &Settings,
    library: &Library,
    mut pos: Pos2,
    presets: &[String],
) {
    if presets.len() > 1 {
        g.text(
            pos + vec2(30.0, 0.0),
            20.0,
            &format!("{}", presets.len()),
            Color32::WHITE,
            Align2::LEFT_CENTER,
        );
    }
    pos.y += 10.0;
    for name in presets {
        let preset = library.get_preset(name).unwrap();

        show_preset_device(g, settings, pos, preset);
        let size = calc_device_size(
            preset.data.num_inputs(),
            preset.data.num_outputs(),
            settings.device_min_pin_spacing,
        );
        pos.y += size.y;
    }
}
