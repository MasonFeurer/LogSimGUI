use egui::{Color32, FontId, Rounding, Style, Visuals};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum Theme {
    Dark = 0,
    Light = 1,
}
impl Theme {
    pub fn visuals(self) -> Visuals {
        match self {
            Self::Dark => dark_mode_visuals(),
            Self::Light => Visuals::light(),
        }
    }

    pub fn set(self, style: &mut Style) {
        style.visuals = self.visuals();

        type Ts = egui::TextStyle;
        type Ff = egui::FontFamily;
        style.text_styles = [
            (Ts::Heading, FontId::new(30.0, Ff::Proportional)),
            (Ts::Body, FontId::new(18.0, Ff::Proportional)),
            (Ts::Monospace, FontId::new(14.0, Ff::Monospace)),
            (Ts::Button, FontId::new(18.0, Ff::Proportional)),
            (Ts::Small, FontId::new(10.0, Ff::Proportional)),
        ]
        .into();
    }
}

pub fn dark_mode_visuals() -> Visuals {
    let mut vis = Visuals::dark();
    vis.widgets.inactive.fg_stroke.color = Color32::WHITE;
    vis.widgets.hovered.fg_stroke.color = Color32::WHITE;
    vis.widgets.active.fg_stroke.color = Color32::WHITE;
    vis.widgets.noninteractive.fg_stroke.color = Color32::WHITE;

    let idle = Color32::from_rgb(100, 100, 100);
    let hovered = Color32::from_rgb(150, 150, 150);
    let pressed = Color32::from_rgb(200, 200, 200);

    vis.widgets.inactive.bg_stroke.color = idle;
    vis.widgets.inactive.bg_fill = idle;
    vis.widgets.inactive.rounding = Rounding::none();

    vis.widgets.hovered.bg_stroke.color = hovered;
    vis.widgets.hovered.bg_fill = hovered;
    vis.widgets.hovered.rounding = Rounding::none();

    vis.widgets.active.bg_stroke.color = pressed;
    vis.widgets.active.bg_fill = pressed;
    vis.widgets.active.rounding = Rounding::none();

    // vis.widgets.noninteractive.bg_stroke.color = Color32::YELLOW;
    // vis.widgets.noninteractive.bg_fill = Color32::YELLOW;
    // vis.widgets.noninteractive.rounding = Rounding::none();
    vis
}

#[derive(Serialize, Deserialize)]
pub struct Settings {
    // App
    pub theme: Theme,
    pub colorful_wires: bool,
    pub auto_link: bool,

    // Debug
    pub debug: bool,

    // Board
    pub board_color: Color32,
    pub board_io_pin_size: f32,
    pub board_io_col_color: Color32,
    pub board_io_col_w: f32,

    pub pin_colors: [Color32; 2],
    pub link_width: f32,
    pub link_colors: [Color32; 2],

    pub device_name_size: f32,
    pub device_pin_size: f32,
    pub device_min_pin_spacing: f32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            // App
            theme: Theme::Dark,
            colorful_wires: false,
            auto_link: false,

            // Debug
            debug: false,

            // Board
            board_color: Color32::from_rgba_premultiplied(20, 20, 20, 255),
            board_io_col_color: Color32::from_rgb(180, 180, 180),
            board_io_pin_size: 8.0,
            board_io_col_w: 40.0,

            pin_colors: [Color32::from_gray(100), Color32::from_rgb(255, 0, 0)],
            link_width: 4.0,
            link_colors: [Color32::from_gray(80), Color32::from_rgb(200, 0, 0)],

            device_name_size: 16.0,
            device_pin_size: 6.0,
            device_min_pin_spacing: 13.0,
        }
    }
}
impl Settings {
    #[inline(always)]
    pub fn pin_color(&self, state: bool) -> Color32 {
        self.pin_colors[state as usize]
    }
    #[inline(always)]
    pub fn link_color(&self, state: bool) -> Color32 {
        self.link_colors[state as usize]
    }
}
