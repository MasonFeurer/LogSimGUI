use crate::preset::DevicePreset;
use crate::scene::{Device, Group, IoSel, Scene, SceneItem};
use crate::settings::Settings;
use crate::*;
use egui::*;

#[inline(always)]
pub fn scene_input_view_y(scene: &Scene, id: u64, v: &View) -> Option<f32> {
    Some(map_io_y(v, scene.inputs.get(&id)?.io.y_pos))
}
#[inline(always)]
pub fn scene_output_view_y(scene: &Scene, id: u64, v: &View) -> Option<f32> {
    Some(map_io_y(v, scene.outputs.get(&id)?.io.y_pos))
}
#[inline(always)]
pub fn unmap_io_y(v: &View, y: f32) -> f32 {
    (y - v.offset.y) / v.scale()
}
#[inline(always)]
pub fn map_io_y(v: &View, y: f32) -> f32 {
    y * v.scale() + v.offset.y
}

pub fn spread_y<T, F: Fn(f32) -> T>(min: f32, max: f32, count: usize, f: F) -> Vec<T> {
    let mut out = Vec::with_capacity(count);
    let step = (max - min) / (count + 1) as f32;
    let mut temp_y = min + step;
    for _ in 0..count {
        out.push(f(temp_y));
        temp_y += step;
    }
    out
}

pub fn device_output_links(s: &Settings, v: &View, device: Rect, count: usize) -> Vec<Pos2> {
    spread_y(device.min.y, device.max.y, count, |y| Pos2 {
        x: device.max.x + s.device_pin_size.x * v.scale() * 0.5,
        y,
    })
}
pub fn device_input_links(s: &Settings, v: &View, device: Rect, count: usize) -> Vec<Pos2> {
    spread_y(device.min.y, device.max.y, count, |y| Pos2 {
        x: device.min.x - s.device_pin_size.x * v.scale() * 0.5,
        y,
    })
}
pub fn device_input_pins(s: &Settings, v: &View, device: Rect, count: usize) -> Vec<Rect> {
    let size = s.device_pin_size * v.scale();
    spread_y(device.min.y, device.max.y, count, |y| Rect {
        min: Pos2::new(device.min.x - size.x, y - size.y * 0.5),
        max: Pos2::new(device.min.x, y + size.y * 0.5),
    })
}
pub fn device_output_pins(s: &Settings, v: &View, device: Rect, count: usize) -> Vec<Rect> {
    let size = s.device_pin_size * v.scale();
    spread_y(device.min.y, device.max.y, count, |y| Rect {
        min: Pos2::new(device.max.x, y - size.y * 0.5),
        max: Pos2::new(device.max.x + size.x, y + size.y * 0.5),
    })
}

#[inline(always)]
pub fn device_pos(device: &Device, v: &View) -> Pos2 {
    v.map_pos(device.pos)
}

#[inline(always)]
pub fn device_size(device: &Device, s: &Settings, v: &View) -> Vec2 {
    v.map_vec(s.device_size(device.num_inputs(), device.num_outputs(), &device.name))
}

pub fn link_target_pos(
    s: &Settings,
    v: &View,
    scene: &Scene,
    target: LinkTarget<u64>,
) -> Option<Pos2> {
    match target {
        LinkTarget::Output(id) => {
            let y = scene_output_view_y(scene, id, v)?;
            let x = scene.rect.max.x
                - s.scene_pin_col_w * v.scale()
                - s.scene_pin_size.x * v.scale() * 0.5;
            Some(Pos2 { x, y })
        }
        LinkTarget::DeviceInput(device_id, input) => {
            let device = scene.devices.get(&device_id)?;
            let rect = Rect::from_min_size(device_pos(device, v), device_size(device, s, v));
            Some(device_input_links(s, v, rect, device.num_inputs())[input])
        }
    }
}
pub fn link_start_pos(
    s: &Settings,
    v: &View,
    scene: &Scene,
    start: LinkStart<u64>,
) -> Option<Pos2> {
    match start {
        LinkStart::Input(id) => {
            let y = scene_input_view_y(scene, id, v)?;
            let x = scene.rect.min.x
                + s.scene_pin_col_w * v.scale()
                + s.scene_pin_size.x * v.scale() * 0.5;
            Some(Pos2 { x, y })
        }
        LinkStart::DeviceOutput(device_id, output) => {
            let device = scene.devices.get(&device_id)?;
            let rect = Rect::from_min_size(device_pos(device, v), device_size(device, s, v));
            Some(device_output_links(s, v, rect, device.num_outputs())[output])
        }
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
    #[inline(always)]
    pub fn inv_scale(&self) -> f32 {
        100.0 / self.zoom
    }
}

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

#[derive(Clone, Copy)]
pub struct ShowStroke {
    pub color: [Color32; 2],
    pub width: [f32; 2],
}

pub struct Graphics<'a> {
    pub ui: &'a mut Ui,
    pub pointer_pos: Pos2,

    pub shapes: Vec<Shape>,
    pub next_id: u32,
}
impl<'a> Graphics<'a> {
    pub fn new(ui: &'a mut Ui, pointer_pos: Pos2) -> Self {
        Self {
            ui,
            pointer_pos,

            shapes: Vec::new(),
            next_id: 7387,
        }
    }

    #[inline(always)]
    pub fn create_response(&mut self, rect: Rect, hovered: bool, sense: Sense) -> Response {
        let id = Id::new(self.next_id);
        self.next_id += 1;
        self.ui.interact_with_hovered(rect, hovered, id, sense)
    }

    pub fn rect(
        &mut self,
        rect: Rect,
        rounding: f32,
        color: [Color32; 2],
        stroke: Option<ShowStroke>,
    ) -> bool {
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

    pub fn line(&mut self, from: Pos2, to: Pos2, width: f32, stroke: ShowStroke) -> bool {
        let hovered = line_contains_point((from, to), width, self.pointer_pos);

        let ShowStroke { color, width } = stroke;
        let color = if hovered { color[1] } else { color[0] };
        let width = if hovered { width[1] } else { width[0] };
        let stroke = Stroke { width, color };

        self.shapes.push(Shape::line_segment([from, to], stroke));
        hovered
    }

    pub fn text(&mut self, pos: Pos2, size: f32, text: &str, color: Color32, align: Align2) {
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
        color: [Color32; 2],
        stroke: Option<ShowStroke>,
    ) -> bool {
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

pub fn show_link(
    g: &mut Graphics,
    s: &Settings,
    v: &View,
    state: bool,
    color: usize,
    from: Pos2,
    to: Pos2,
) -> bool {
    let width = s.link_width * v.scale();
    let color = LINK_COLORS[color][state as usize];

    let hovered = g.line(
        from,
        to,
        s.link_width,
        ShowStroke {
            color: [color; 2],
            width: [width, width + 2.0],
        },
    );
    hovered
}
pub fn show_pin<T: Into<RichText>>(
    g: &mut Graphics,
    s: &Settings,
    rect: Rect,
    state: bool,
    hover_text: T,
) -> bool {
    let hover_text = hover_text.into();
    let color = s.pin_color(state);
    let hovered = g.rect(rect, 0.0, [color; 2], PIN_STROKE);
    if !hover_text.text().trim().is_empty() {
        g.create_response(rect, hovered, Sense::hover())
            .on_hover_text(hover_text);
    }
    hovered
}
pub fn show_group_header(
    g: &mut Graphics,
    col_w: f32,
    group: &Group,
    field: BitField,
    center: f32,
    top: f32,
) -> bool {
    let text = group.display_value(field);
    let size = GROUP_HEADER_SIZE;
    let rect = Rect::from_min_size(Pos2::new(center - col_w * 0.5, top), Vec2::new(col_w, size));
    let hovered = g.rect(
        rect,
        0.0,
        [GROUP_COLOR; 2],
        Some(ShowStroke {
            color: [Color32::TRANSPARENT, Color32::WHITE],
            width: [1.0, 1.0],
        }),
    );
    g.text(
        Pos2::new(center, top + size * 0.5),
        size,
        &text,
        Color32::WHITE,
        Align2::CENTER_CENTER,
    );
    hovered
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
        Color32::from_rgba_unmultiplied(r, g, b, ALPHA)
    };
    let size = view.map_vec(preset.size(s));

    g.rect(
        Rect::from_min_size(pos, size),
        5.0,
        [color; 2],
        Some(ShowStroke {
            color: [Color32::from_rgb(200, 200, 200); 2],
            width: [1.0, 3.0],
        }),
    );

    let name_color = if Rgba::from(color).intensity() > 0.5 {
        Color32::from_rgba_unmultiplied(0, 0, 0, ALPHA)
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, ALPHA)
    };
    g.text(
        pos + size * 0.5,
        s.device_name_font_size * view.scale(),
        &preset.name,
        name_color,
        Align2::CENTER_CENTER,
    );
}

#[derive(Clone, Copy)]
pub enum DeviceItem {
    Device,
    Input(usize),
    Output(usize),
}
pub fn show_device(
    g: &mut Graphics,
    s: &Settings,
    view: &View,
    device: &scene::Device,
    show_id: Option<u64>,
) -> Option<DeviceItem> {
    let pos = device_pos(device, view);
    let size = device_size(device, s, view);
    let color = device.color;
    let rect = Rect::from_min_size(pos, size);
    let mut hovered: Option<DeviceItem> = None;

    // --- Show rectangle ---
    let rect_hovered = g.rect(
        rect,
        5.0,
        [color; 2],
        Some(ShowStroke {
            color: [Color32::from_rgb(200, 200, 200); 2],
            width: [1.0, 3.0],
        }),
    );
    if rect_hovered {
        hovered = Some(DeviceItem::Device);
    }

    // --- Show name ---
    let name_color = if Rgba::from(color).intensity() > 0.5 {
        Color32::BLACK
    } else {
        Color32::WHITE
    };
    g.text(
        pos + size * 0.5,
        s.device_name_font_size * view.scale(),
        &device.name,
        name_color,
        Align2::CENTER_CENTER,
    );

    // --- Show input and output pins
    let input_pins = device_input_pins(s, view, rect, device.num_inputs());
    for i in 0..input_pins.len() {
        let state = device.data.input().get(i);
        if show_pin(g, s, input_pins[i], state, &device.input_names[i]) {
            hovered = Some(DeviceItem::Input(i));
        }
    }
    let output_pins = device_output_pins(s, view, rect, device.num_outputs());
    for i in 0..output_pins.len() {
        let state = device.data.output().get(i);
        if show_pin(g, s, output_pins[i], state, &device.output_names[i]) {
            hovered = Some(DeviceItem::Output(i));
        }
    }

    // --- Show ID ---
    if let Some(id) = show_id {
        g.text(
            pos + Vec2::new(size.x * 0.5, -10.0),
            10.0,
            &format!("{}", id),
            Color32::from_gray(120),
            Align2::CENTER_CENTER,
        );
    }
    hovered
}

pub const GROUP_COLOR: Color32 = Color32::from_gray(120);
pub const GROUP_HEADER_SIZE: f32 = 16.0;
pub const PIN_STROKE: Option<ShowStroke> = Some(ShowStroke {
    width: [0.0, 1.0],
    color: [Color32::from_gray(200); 2],
});
pub const BULB_STROKE: Option<ShowStroke> = Some(ShowStroke {
    width: [0.0, 1.0],
    color: [Color32::from_gray(200); 2],
});

pub fn show_scene(
    g: &mut Graphics,
    s: &Settings,
    view: &View,
    scene: &scene::Scene,
    dead_links: &mut Vec<(LinkStart<u64>, usize)>,
    output_link_err: bool,
    show_device_ids: bool,
) -> Option<SceneItem> {
    let mut result: Option<SceneItem> = None;
    let rect = scene.rect;

    // --- Show links from devices ---
    // If we are hovering a device input or output pin,
    // that should take priority for interaction
    let interact_w_link = match result {
        Some(SceneItem::DeviceInput(..)) => false,
        Some(SceneItem::DeviceOutput(..)) => false,
        _ => true,
    };
    for (device_id, device) in &scene.devices {
        let device_rect =
            Rect::from_min_size(device_pos(device, view), device_size(device, s, view));

        let links = device_output_links(s, view, device_rect, device.num_outputs());
        for output_idx in 0..device.links.len() {
            for (link_idx, link) in device.links[output_idx].iter().enumerate() {
                let state = device.data.output().get(output_idx);
                let link_start = links[output_idx];

                let Some(target_pos) = link_target_pos(s, view, scene, link.target) else {
                	dead_links.push((LinkStart::DeviceOutput(*device_id, output_idx), link_idx));
                	continue;
                };
                let hovered = show_link(g, s, view, state, link.color, link_start, target_pos);
                if hovered && interact_w_link {
                    result = Some(SceneItem::DeviceOutputLink(
                        *device_id, output_idx, link_idx,
                    ));
                }
            }
        }
    }

    // --- Show links from inputs ---
    for (input_id, input) in &scene.inputs {
        let y = scene_input_view_y(scene, *input_id, view).unwrap();
        let link_start = Pos2 {
            x: rect.min.x + s.scene_pin_col_w * view.scale() + s.scene_pin_size.x * view.scale(),
            y,
        };
        for (link_idx, link) in input.links.iter().enumerate() {
            let Some(target_pos) = link_target_pos(s, view, scene, link.target) else {
            	dead_links.push((LinkStart::Input(*input_id), link_idx));
            	continue;
            };
            let hovered = show_link(
                g,
                s,
                view,
                input.io.state,
                link.color,
                link_start,
                target_pos,
            );
            if hovered && interact_w_link {
                result = Some(SceneItem::InputLink(*input_id, link_idx));
            }
        }
    }

    // --- Show devices ---
    for (device_id, device) in &scene.devices {
        let show_id = show_device_ids.then(|| *device_id);
        let device_rs = show_device(g, s, view, device, show_id);

        if let Some(device_item) = device_rs {
            let scene_item = match device_item {
                DeviceItem::Device => SceneItem::Device(*device_id),
                DeviceItem::Input(input) => SceneItem::DeviceInput(*device_id, input),
                DeviceItem::Output(output) => SceneItem::DeviceOutput(*device_id, output),
            };
            result = Some(scene_item);
        }
    }

    // --- Show input and output columns ---
    let col_w = s.scene_pin_col_w * view.scale();
    let col_size = Vec2::new(col_w, rect.height());
    let input_rect = Rect::from_min_size(rect.min, col_size);
    let output_rect = Rect::from_min_size(rect.max - col_size, col_size);
    let color = [g.ui.visuals().faint_bg_color; 2];

    g.rect(input_rect, 0.0, color, None);
    g.rect(output_rect, 0.0, color, None);

    let show_io_bulb = move |g: &mut Graphics, state: bool, x: f32, y: f32| -> bool {
        g.circle(
            Pos2::new(x, y),
            col_w * 0.5,
            [s.pin_color(state); 2],
            BULB_STROKE,
        )
    };
    let show_io_decor = move |g: &mut Graphics, x: f32, y: f32| {
        let (x0, x1) = (x - col_w * 0.5, x + col_w * 0.5);
        let (y0, y1) = (y - col_w * 0.5, y + col_w * 0.5);
        let stroke = ShowStroke {
            color: [GROUP_COLOR; 2],
            width: [2.0; 2],
        };
        g.line(Pos2::new(x0, y0), Pos2::new(x0, y1), 0.0, stroke);
        g.line(Pos2::new(x1, y0), Pos2::new(x1, y1), 0.0, stroke);
    };

    // --- Show input pins ---
    let pin_size = s.scene_pin_size * view.scale();
    for (input_id, input) in &scene.inputs {
        let input = &input.io;
        let y = scene_input_view_y(scene, *input_id, view).unwrap();
        let x = rect.min.x + col_w * 0.5;

        let pin_rect = Rect::from_min_size(
            Pos2::new(rect.min.x + col_w, y - pin_size.y * 0.5),
            pin_size,
        );
        if show_pin(g, s, pin_rect, input.state, &input.name) {
            result = Some(SceneItem::InputPin(*input_id));
        }
        if input.group_member.is_some() {
            show_io_decor(g, x, y);
        }
        if show_io_bulb(g, input.state, x, y) {
            result = Some(SceneItem::InputBulb(*input_id));
        }
    }

    // --- Show input group headers ---
    for (group_id, group) in &scene.input_groups {
        let center = rect.min.x + col_w * 0.5;
        let field = scene.input_field();
        let header_top = map_io_y(&view, group.bottom_y(IoSel::Input, scene)) + col_w * 0.5;
        if show_group_header(g, col_w, group, field, center, header_top) {
            result = Some(SceneItem::InputGroup(*group_id));
        }
    }

    // --- Show output pins ---
    for (output_id, output) in &scene.outputs {
        let output = &output.io;
        let y = scene_output_view_y(scene, *output_id, view).unwrap();
        let x = rect.max.x - col_w * 0.5;

        let pin_rect = Rect::from_min_size(
            Pos2::new(rect.max.x - col_w - pin_size.x, y - pin_size.y * 0.5),
            pin_size,
        );
        let hover_text = if output_link_err {
            RichText::new("Can't link from scene input to scene output").color(Color32::RED)
        } else {
            RichText::new(&output.name)
        };
        if show_pin(g, s, pin_rect, output.state, hover_text) {
            result = Some(SceneItem::OutputPin(*output_id));
        }
        if output.group_member.is_some() {
            show_io_decor(g, x, y);
        }
        if show_io_bulb(g, output.state, x, y) {
            result = Some(SceneItem::OutputBulb(*output_id));
        }
    }

    // --- Show output group headers ---
    for (group_id, group) in &scene.output_groups {
        let center = rect.max.x - col_w * 0.5;
        let field = scene.output_field();
        let header_top = map_io_y(view, group.bottom_y(IoSel::Output, scene)) + col_w * 0.5;
        if show_group_header(g, col_w, group, field, center, header_top) {
            result = Some(SceneItem::OutputGroup(*group_id));
        }
    }
    result
}
