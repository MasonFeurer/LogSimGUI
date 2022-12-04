use egui::{Color32, Pos2, Vec2, Visuals};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Settings {
    // App
    pub dark_mode: bool,
    pub high_contrast: bool,
    pub dev_options: bool,
    pub preset_placer_pos: Pos2,

    // Scene IO
    pub scene_pin_col_w: f32,
    pub scene_pin_size: Vec2,
    pub pin_color: [Color32; 2],

    // Scene links
    pub link_width: f32,
    pub link_color: [Color32; 2],

    // Scene devices
    pub device_name_font_size: f32,
    pub device_pin_size: Vec2,
    pub device_min_pin_spacing: f32,
}
impl Settings {
    pub fn default() -> Self {
        Self {
            // App
            dark_mode: true,
            high_contrast: true,
            dev_options: false,
            preset_placer_pos: Pos2::new(100.0, 100.0),
            pin_color: [Color32::from_gray(100), Color32::from_rgb(255, 0, 0)],

            // Scene IO
            scene_pin_col_w: 40.0,
            scene_pin_size: Vec2::new(15.0, 10.0),

            // Scene links
            link_width: 4.0,
            link_color: [Color32::from_gray(80), Color32::from_rgb(200, 0, 0)],

            // Scene devices
            device_name_font_size: 16.0,
            device_pin_size: Vec2::new(15.0, 10.0),
            device_min_pin_spacing: 14.0,
        }
    }

    #[inline(always)]
    pub fn visuals(&self) -> Visuals {
        if self.dark_mode {
            let mut vis = Visuals::dark();
            if self.high_contrast {
                let color = Color32::WHITE;
                vis.widgets.inactive.fg_stroke.color = color;
                vis.widgets.noninteractive.fg_stroke.color = color;
                vis.widgets.hovered.fg_stroke.color = color;
                vis.widgets.active.fg_stroke.color = color;
            }
            vis
        } else {
            let mut vis = Visuals::light();
            if self.high_contrast {
                let color = Color32::BLACK;
                vis.widgets.inactive.fg_stroke.color = color;
                vis.widgets.noninteractive.fg_stroke.color = color;
                vis.widgets.hovered.fg_stroke.color = color;
                vis.widgets.active.fg_stroke.color = color;
            }
            vis
        }
    }

    #[inline(always)]
    pub fn pin_color(&self, state: bool) -> Color32 {
        self.pin_color[state as usize]
    }
    #[inline(always)]
    pub fn link_color(&self, state: bool) -> Color32 {
        self.link_color[state as usize]
    }
    pub fn device_size(&self, num_inputs: usize, num_outputs: usize, name: &str) -> Vec2 {
        let w = name.len() as f32 * self.device_name_font_size;
        let h = f32::max(
            self.device_pins_height(num_inputs),
            self.device_pins_height(num_outputs),
        );
        Vec2::new(w, h)
    }
    #[inline(always)]
    pub fn device_pins_height(&self, count: usize) -> f32 {
        (count as f32 + 1.0) * self.device_min_pin_spacing
    }
}
