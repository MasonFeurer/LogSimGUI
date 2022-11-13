use crate::preset::DevicePreset;
use crate::scene::{Group, Scene};
use crate::settings::Settings;
use crate::*;
use eframe::egui::*;

pub const GROUP_COLOR: Color = Color::from_gray(150);

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

    #[inline(always)]
    pub fn unmap_pos(&self, pos: Pos2) -> Pos2 {
        Pos2 {
            x: ((pos.x - self.offset.x - self.origin.x) / self.scale()) + self.origin.x,
            y: ((pos.y - self.offset.y - self.origin.y) / self.scale()) + self.origin.y,
        }
    }

    #[inline(always)]
    pub fn map_x(&self, x: f32) -> f32 {
        (x - self.origin.x) * self.scale() + self.origin.x + self.offset.x
    }
    #[inline(always)]
    pub fn map_y(&self, y: f32) -> f32 {
        (y - self.origin.y) * self.scale() + self.origin.y + self.offset.y
    }

    #[inline(always)]
    pub fn map_pos(&self, pos: Pos2) -> Pos2 {
        Pos2 {
            x: self.map_x(pos.x),
            y: self.map_y(pos.y),
        }
    }
    #[inline(always)]
    pub fn map_vec(&self, vec: Vec2) -> Vec2 {
        vec * self.scale()
    }

    #[inline(always)]
    pub fn scale(&self) -> f32 {
        self.zoom / 100.0
    }
}

pub fn response_has_priority(new: &Response, old: &Response) -> bool {
    match (
        old.drag_delta() != Vec2::ZERO,
        new.drag_delta() != Vec2::ZERO,
    ) {
        (true, _) => return true,
        (false, true) => return false,
        (false, false) => (),
    };
    old.clicked() || old.secondary_clicked()
}
pub fn response_has_interaction(rs: &Response) -> bool {
    rs.clicked() || rs.secondary_clicked() || rs.hovered() || rs.drag_delta() != Vec2::ZERO
}

pub fn handle_sub_response<T>(rs: &mut Option<(Response, T)>, sub_rs: Response, item: T) {
    if !response_has_interaction(&sub_rs) {
        return;
    }
    let replace = match rs {
        Some(rs) => response_has_priority(&sub_rs, &rs.0),
        None => true,
    };
    if replace {
        *rs = Some((sub_rs.clone(), item))
    }
}

// :SHOW STRUCTS
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

#[derive(Clone, Copy)]
pub struct ShowStroke {
    pub color: [Color; 2],
    pub width: [f32; 2],
}

// :GRAPHICS
pub struct Graphics<'a> {
    pub ui: &'a mut Ui,
    pub rect: Rect,
    pub pointer_pos: Pos2,

    pub shapes: Vec<Shape>,
    pub next_id: u32,
}
impl<'a> Graphics<'a> {
    pub fn new(ui: &'a mut Ui, rect: Rect, pointer_pos: Pos2) -> Self {
        Self {
            ui,
            rect,
            pointer_pos,

            shapes: Vec::new(),
            next_id: 2848,
        }
    }

    #[inline(always)]
    pub fn create_response(&mut self, rect: Rect, hovered: bool, sense: Sense) -> Response {
        let id = Id::new(self.next_id);
        self.next_id += 53;
        self.ui.interact_with_hovered(rect, hovered, id, sense)
    }

    pub fn rect(
        &mut self,
        rect: Rect,
        rounding: f32,
        color: [Color; 2],
        stroke: Option<ShowStroke>,
    ) -> Response {
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
        self.create_response(rect, hovered, Sense::click_and_drag())
    }

    pub fn static_rect(
        &mut self,
        rect: Rect,
        rounding: f32,
        color: [Color; 2],
        stroke: Option<ShowStroke>,
    ) {
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
    }

    pub fn line(&mut self, from: Pos2, to: Pos2, width: f32, stroke: ShowStroke) -> Response {
        let min_x = f32::min(from.x, to.x);
        let max_x = f32::max(from.x, to.x);
        let min_y = f32::min(from.y, to.y);
        let max_y = f32::max(from.y, to.y);
        let rect = Rect::from_min_max(Pos2::new(min_x, min_y), Pos2::new(max_x, max_y));

        let hovered = line_contains_point((from, to), width, self.pointer_pos);

        let ShowStroke { color, width } = stroke;
        let color = if hovered { color[1] } else { color[0] };
        let width = if hovered { width[1] } else { width[0] };
        let stroke = Stroke { width, color };

        self.shapes.push(Shape::line_segment([from, to], stroke));
        self.create_response(rect, hovered, Sense::click_and_drag())
    }

    pub fn text(&mut self, pos: Pos2, size: f32, text: &str, color: Color, align: Align2) {
        self.shapes.push(Shape::text(
            &self.ui.fonts(),
            pos,
            align,
            text,
            FontId::monospace(size),
            color,
        ));
    }

    pub fn circle(
        &mut self,
        center: Pos2,
        radius: f32,
        color: [Color; 2],
        stroke: Option<ShowStroke>,
    ) -> Response {
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
        self.create_response(rect, hovered, Sense::click_and_drag())
    }
}

// :SCENE GRAPHICS
pub fn show_link(g: &mut Graphics, s: &Settings, state: bool, from: Pos2, to: Pos2) -> Response {
    g.line(
        from,
        to,
        10.0,
        ShowStroke {
            color: [s.power_color(state); 2],
            width: [3.0, 5.0],
        },
    )
}
pub fn show_pin(g: &mut Graphics, s: &Settings, pin: &Pin, state: bool) -> Response {
    let color = s.power_color(state);
    let mut rs = g.rect(
        pin.rect(s),
        0.0,
        [color; 2],
        Some(ShowStroke {
            color: [Color::GREEN, Color::from_gray(255)],
            width: [0.0, 1.0],
        }),
    );
    if !pin.name.trim().is_empty() {
        rs = rs.on_hover_text(pin.name);
    }
    rs
}
pub fn show_group(
    g: &mut Graphics,
    s: &Settings,
    group: &Group,
    states: &[bool],
    center: f32,
) -> Response {
    let col_w = s.scene_pin_col_w;
    let text = Scene::group_value(group, states);
    let size = 20.0;
    let rs = g.rect(
        Rect::from_min_size(
            Pos2::new(center - col_w * 0.5, group.bottom),
            Vec2::new(col_w, size),
        ),
        0.0,
        [GROUP_COLOR; 2],
        Some(ShowStroke {
            color: [Color::TRANSPARENT, Color::WHITE],
            width: [1.0, 1.0],
        }),
    );
    g.text(
        Pos2::new(center, group.bottom + size * 0.5),
        size,
        &text,
        Color::WHITE,
        Align2::CENTER_CENTER,
    );
    rs
}

#[derive(Clone, Copy)]
pub enum DeviceItem {
    Device,
    Input(usize),
    Output(usize),
}

pub fn show_device_preset(
    g: &mut Graphics,
    s: &Settings,
    view: &View,
    pos: Pos2,
    preset: &DevicePreset,
) {
    const ALPHA: u8 = 40;
    let color = {
        let [r, g, b, _] = preset.color;
        Color::from_rgba_unmultiplied(r, g, b, ALPHA)
    };
    let size = view.map_vec(preset.size(s));

    let _ = g.rect(
        Rect::from_min_size(pos, size),
        5.0,
        [color; 2],
        Some(ShowStroke {
            color: [Color32::from_rgb(200, 200, 200); 2],
            width: [1.0, 3.0],
        }),
    );

    let name_color = if Rgba::from(color).intensity() > 0.5 {
        Color::from_rgba_unmultiplied(0, 0, 0, ALPHA)
    } else {
        Color::from_rgba_unmultiplied(255, 255, 255, ALPHA)
    };
    let _ = g.text(
        pos + size * 0.5,
        s.device_name_font_size * view.scale(),
        &preset.name,
        name_color,
        Align2::CENTER_CENTER,
    );
}

pub fn show_device(
    g: &mut Graphics,
    s: &Settings,
    view: &View,
    id: IntId,
    device: &scene::Device,
) -> Option<(Response, DeviceItem)> {
    let pos = device.pos(view);
    let size = device.size(s, view);
    let color = device.color;

    let mut result: Option<(Response, DeviceItem)> = None;
    let rect_rs = g.rect(
        Rect::from_min_size(pos, size),
        5.0,
        [color; 2],
        Some(ShowStroke {
            color: [Color32::from_rgb(200, 200, 200); 2],
            width: [1.0, 3.0],
        }),
    );
    if response_has_interaction(&rect_rs) {
        result = Some((rect_rs, DeviceItem::Device));
    }

    let name_color = if Rgba::from(color).intensity() > 0.5 {
        Color::BLACK
    } else {
        Color::WHITE
    };
    g.text(
        pos + size * 0.5,
        s.device_name_font_size * view.scale(),
        &device.name,
        name_color,
        Align2::CENTER_CENTER,
    );

    // ## show inputs
    let input_pins = device.input_pins(s, view);
    for i in 0..input_pins.len() {
        let state = device.data.input().get(i);
        let pin_rs = show_pin(g, s, &input_pins[i], state);
        handle_sub_response(&mut result, pin_rs, DeviceItem::Input(i));
    }

    // ## show outputs
    let output_pins = device.output_pins(s, view);
    for i in 0..output_pins.len() {
        let state = device.data.output().get(i);
        let pin_rs = show_pin(g, s, &output_pins[i], state);
        handle_sub_response(&mut result, pin_rs, DeviceItem::Output(i));
    }

    if s.show_device_id {
        g.text(
            pos + Vec2::new(size.x * 0.5, -10.0),
            10.0,
            &format!("{}", id.0),
            Color::WHITE,
            Align2::CENTER_CENTER,
        );
    }
    result
}

#[derive(Clone, Copy, Debug)]
pub enum SceneItem {
    Device(IntId),
    DeviceInput(IntId, usize),
    DeviceOutput(IntId, usize),
    DeviceOutputLink(IntId, usize, usize),
    InputPin(IntId),
    InputBulb(IntId),
    InputLink(IntId, usize),
    InputGroup(IntId),
    OutputPin(IntId),
    OutputBulb(IntId),
    OutputGroup(IntId),
}

pub fn show_scene(
    g: &mut Graphics,
    s: &Settings,
    view: &View,
    scene: &scene::Scene,
    dead_links: &mut Vec<(LinkStart<IntId>, usize)>,
) -> Option<(Response, SceneItem)> {
    let mut result: Option<(Response, SceneItem)> = None;

    // DRAW DEVICES
    for (device_id, device) in &scene.devices {
        // DRAW DEVICE
        let device_rs = show_device(g, s, view, *device_id, device);

        if let Some((rs, device_item)) = device_rs {
            let scene_item = match device_item {
                DeviceItem::Device => SceneItem::Device(*device_id),
                DeviceItem::Input(input) => SceneItem::DeviceInput(*device_id, input),
                DeviceItem::Output(output) => SceneItem::DeviceOutput(*device_id, output),
            };
            handle_sub_response(&mut result, rs, scene_item);
        }

        // DRAW LINKS FROM DEVICE OUTPUTS
        for output_idx in 0..device.links.len() {
            for (link_idx, target) in device.links[output_idx].iter().enumerate() {
                let state = device.data.output().get(output_idx);

                let link_start = device.output_pins(s, view)[output_idx].tip(s);

                let Some(pin) = scene.get_link_target_pin(target, s, view) else {
                	dead_links.push((LinkStart::DeviceOutput(*device_id, output_idx), link_idx));
                	continue;
                };

                let link_rs = show_link(g, s, state, link_start, pin.tip(s));

                let scene_item = SceneItem::DeviceOutputLink(*device_id, output_idx, link_idx);
                handle_sub_response(&mut result, link_rs, scene_item);
            }
        }
    }

    // IO COLUMN BARS
    let col_w = s.scene_pin_col_w;
    let output_col_x = g.rect.max.x - col_w;
    let input_col_x = g.rect.min.x + col_w;

    let (y0, y1) = (g.rect.min.y, g.rect.max.y);

    let stroke = ShowStroke {
        color: [Color32::from_gray(100), Color32::WHITE],
        width: [2.0, 2.0],
    };
    let _ = g.line(
        Pos2::new(input_col_x, y0),
        Pos2::new(input_col_x, y1),
        2.0,
        stroke,
    );
    let _ = g.line(
        Pos2::new(output_col_x, y0),
        Pos2::new(output_col_x, y1),
        2.0,
        stroke,
    );

    // DRAW SCENE INPUTS
    for (input_id, input) in &scene.inputs {
        // DRAW INPUT
        let pin = scene.input_pin(input, s, view);
        let pin_rs = show_pin(g, s, &pin, input.state);

        let scene_item = SceneItem::InputPin(*input_id);
        handle_sub_response(&mut result, pin_rs, scene_item);

        if input.group_member.is_some() {
            let _ = g.static_rect(
                Rect::from_min_size(
                    pin.origin - Vec2::new(col_w, col_w * 0.5),
                    Vec2::splat(col_w),
                ),
                0.0,
                [GROUP_COLOR; 2],
                None,
            );
        }

        let scene_item = SceneItem::InputBulb(*input_id);
        let bulb_rs = g.circle(
            pin.origin - Vec2::new(col_w * 0.5, 0.0),
            col_w * 0.5,
            [s.power_color(input.state); 2],
            Some(ShowStroke {
                color: [Color::from_gray(200), Color::from_gray(255)],
                width: [1.0, 2.0],
            }),
        );
        handle_sub_response(&mut result, bulb_rs, scene_item);

        // DRAW LINKS FROM INPUT
        let link_start = pin.tip(s);
        for (link_idx, target) in input.links.iter().enumerate() {
            let Some(pin) = scene.get_link_target_pin(&target.wrap(), s, view) else {
            	dead_links.push((LinkStart::Input(*input_id), link_idx));
        		continue
        	};

            let link_rs = show_link(g, s, input.state, link_start, pin.tip(s));
            let scene_item = SceneItem::InputLink(*input_id, link_idx);
            handle_sub_response(&mut result, link_rs, scene_item);
        }
    }

    // DRAW INPUT GROUPS
    for (group_id, group) in &scene.input_groups {
        let center = g.rect.min.x + col_w * 0.5;
        let mut states = Vec::with_capacity(group.members.len());
        for member_id in &group.members {
            states.push(scene.inputs.get(member_id).unwrap().state);
        }
        let group_rs = show_group(g, s, group, &states, center);
        let scene_item = SceneItem::InputGroup(*group_id);
        handle_sub_response(&mut result, group_rs, scene_item);
    }

    // DRAW SCENE OUTPUTS
    for (output_id, output) in &scene.outputs {
        let pin = scene.output_pin(output, s, view);
        let pin_rs = show_pin(g, s, &pin, output.state);

        let scene_item = SceneItem::OutputPin(*output_id);
        handle_sub_response(&mut result, pin_rs, scene_item);

        let scene_item = SceneItem::OutputBulb(*output_id);
        let bulb_rs = g.circle(
            pin.origin + Vec2::new(col_w * 0.5, 0.0),
            col_w * 0.5,
            [s.power_color(output.state); 2],
            Some(ShowStroke {
                color: [Color::from_gray(200), Color::from_gray(255)],
                width: [1.0, 2.0],
            }),
        );
        handle_sub_response(&mut result, bulb_rs, scene_item);
    }

    // DRAW OUTPUT GROUPS
    for (group_id, group) in &scene.output_groups {
        let center = g.rect.max.x - col_w * 0.5;
        let mut states = Vec::with_capacity(group.members.len());
        for member_id in &group.members {
            states.push(scene.outputs.get(member_id).unwrap().state);
        }
        let group_rs = show_group(g, s, group, &states, center);
        let scene_item = SceneItem::OutputGroup(*group_id);
        handle_sub_response(&mut result, group_rs, scene_item);
    }
    result
}
