use crate::preset::Presets;
use crate::Color;
use eframe::egui::*;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::io;
use std::path::PathBuf;
use std::process::Command;

fn fmt_err<E: Debug>(err: E) -> String {
    format!("{err:?}")
}

pub fn config_dir() -> Option<PathBuf> {
    let mut config_dir = dirs::config_dir()?;
    config_dir.push("logic-sim-gui");
    Some(config_dir)
}
pub fn save_config(name: &str, bytes: &[u8]) -> io::Result<()> {
    let mut path = config_dir().unwrap();
    let _ = std::fs::create_dir(&path);
    path.push(name);
    std::fs::write(path, bytes)
}
pub fn load_config(name: &str) -> io::Result<Vec<u8>> {
    let mut path = config_dir().unwrap();
    let _ = std::fs::create_dir(&path);
    path.push(name);
    std::fs::read(path)
}

pub fn reveal_dir(dir: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(target_os = "windows")]
    let cmd = "explorer";
    #[cfg(target_os = "linux")]
    let cmd = "xdg-open";
    let output = Command::new(cmd).arg(dir).output().unwrap();
    if output.stderr.is_empty() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}

pub fn encode_settings(settings: &Settings) -> Vec<u8> {
    serde_json::to_string(settings).unwrap().as_bytes().to_vec()
}
pub fn decode_settings(bytes: &[u8]) -> Result<Settings, String> {
    let json = String::from_utf8_lossy(bytes).to_owned();
    serde_json::from_str(&json).map_err(fmt_err)
}

pub fn encode_presets(presets: &Presets) -> Vec<u8> {
    bincode::serialize(presets, bincode::Infinite).unwrap()
}
pub fn decode_presets(bytes: &[u8]) -> Result<Presets, String> {
    let presets: Result<Presets, _> = bincode::deserialize(&bytes);
    match presets {
        Ok(presets) => {
            // loaded presets must contain a cat of ID 0
            for (cat_id, _) in &presets.cats {
                if cat_id.0 == 0 {
                    return Ok(presets);
                }
            }
            // at this point, we have determined that there isn't a cat of ID 0
            println!("error loading presets: corrupted presets");
            return Err("corrupted presets".to_owned());
        }
        Err(err) => Err(fmt_err(&err)),
    }
}

pub fn save_settings(settings: &Settings) {
    let bytes = encode_settings(settings);
    if let Err(err) = save_config("settings.json", &bytes) {
        eprintln!("error saving settings: {err}");
    }
}
pub fn load_settings() -> Option<Settings> {
    let bytes = match load_config("settings.json") {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("error loading settings: {err}");
            return None;
        }
    };
    match decode_settings(&bytes) {
        Ok(settings) => Some(settings),
        Err(err) => {
            eprintln!("error loading settings: {err}");
            None
        }
    }
}

pub fn save_presets(presets: &Presets) {
    let bytes = encode_presets(presets);
    if let Err(err) = save_config("presets", &bytes) {
        eprintln!("error saving presets: {err}");
    }
}
pub fn load_presets() -> Option<Presets> {
    let bytes = match load_config("presets") {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("error loading presets: {err}");
            return None;
        }
    };
    match decode_presets(&bytes) {
        Ok(presets) => Some(presets),
        Err(err) => {
            eprintln!("error loading presets: {err}");
            None
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub dark_mode: bool,
    pub high_contrast: bool,
    pub power_on_color: [u8; 4],
    pub power_off_color: [u8; 4],

    pub scene_pin_col_w: f32,
    pub scene_pin_size: [f32; 2],

    pub device_name_font_size: f32,
    pub device_pin_size: [f32; 2],
    pub device_min_pin_spacing: f32,

    pub show_device_id: bool,
    pub show_write_queue: bool,

    pub preset_picker_pos: [f32; 2],
}
impl Settings {
    pub fn default() -> Self {
        Self {
            dark_mode: true,
            high_contrast: true,
            power_on_color: [255, 0, 0, 255],
            power_off_color: [100, 100, 100, 255],

            scene_pin_col_w: 40.0,
            scene_pin_size: [15.0, 8.0],

            device_name_font_size: 16.0,
            device_pin_size: [15.0, 8.0],
            device_min_pin_spacing: 15.0,

            show_device_id: false,
            show_write_queue: false,
            preset_picker_pos: [50.0, 50.0],
        }
    }

    #[inline(always)]
    pub fn visuals(&self) -> Visuals {
        if self.dark_mode {
            let mut vis = Visuals::dark();
            if self.high_contrast {
                let color = Color::WHITE;
                vis.widgets.inactive.fg_stroke.color = color;
                vis.widgets.noninteractive.fg_stroke.color = color;
                vis.widgets.hovered.fg_stroke.color = color;
                vis.widgets.active.fg_stroke.color = color;
            }
            vis
        } else {
            Visuals::light()
        }
    }

    #[inline(always)]
    pub fn power_color(&self, state: bool) -> Color {
        let [r, g, b, _] = match state {
            true => self.power_on_color,
            false => self.power_off_color,
        };
        Color::from_rgb(r, g, b)
    }
    pub fn device_size(&self, num_inputs: usize, num_outputs: usize, name: &str) -> Vec2 {
        let w = name.len() as f32 * self.device_name_font_size;
        let h = f32::max(
            self.device_pins_height(num_inputs),
            self.device_pins_height(num_outputs),
        );
        Vec2::new(w, h)
    }
    pub fn device_pins_height(&self, count: usize) -> f32 {
        (count as f32 + 1.0) * self.device_min_pin_spacing
    }
}
