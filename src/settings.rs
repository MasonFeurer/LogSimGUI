use crate::preset::{DevicePreset, Presets};
use crate::scene::Scene;
use eframe::egui::*;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{fmt, fs, io, process};

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub dark_mode: bool,
    pub high_contrast: bool,
    pub dev_options: bool,
    pub power_on_color: [u8; 4],
    pub power_off_color: [u8; 4],

    pub scene_pin_col_w: f32,
    pub scene_pin_size: [f32; 2],
    pub link_width: f32,

    pub device_name_font_size: f32,
    pub device_pin_size: [f32; 2],
    pub device_min_pin_spacing: f32,
}
impl Settings {
    pub fn default() -> Self {
        Self {
            dark_mode: true,
            high_contrast: true,
            dev_options: false,
            power_on_color: [255, 0, 0, 255],
            power_off_color: [100, 100, 100, 255],

            scene_pin_col_w: 40.0,
            scene_pin_size: [15.0, 10.0],
            link_width: 4.0,

            device_name_font_size: 16.0,
            device_pin_size: [15.0, 10.0],
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
    pub fn power_color(&self, state: bool) -> Color32 {
        let [r, g, b, _] = match state {
            true => self.power_on_color,
            false => self.power_off_color,
        };
        Color32::from_rgb(r, g, b)
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

#[derive(Clone)]
pub struct Err {
    not_found: bool,
    path: String,
    msg: String,
}
impl Err {
    #[inline(always)]
    fn io_err<P: AsRef<Path>>(path: P, err: io::Error) -> Self {
        Self {
            not_found: false,
            path: format!("{}", path.as_ref().display()),
            msg: format!("{:?}", err.kind()),
        }
    }
    #[inline(always)]
    fn new<P: AsRef<Path>, M: fmt::Debug>(path: P, msg: M) -> Self {
        Self {
            not_found: false,
            path: format!("{}", path.as_ref().display()),
            msg: format!("\"{msg:?}\""),
        }
    }
    #[inline(always)]
    fn not_found(mut self) -> Self {
        self.not_found = true;
        self
    }

    #[inline(always)]
    fn context(mut self, ctx: &str) -> Self {
        self.msg = format!("{} : {}", ctx, self.msg);
        self
    }

    #[inline(always)]
    pub fn log(self) {
        println!("{} ({:?})", self.msg, self.path);
    }
    pub fn panic(self) -> ! {
        panic!("{} ({:?})", self.msg, self.path);
    }
}

pub trait ErrResult<T> {
    fn log_err(self);
    fn unwrap_res_or<F: Fn() -> T>(self, f: F) -> T;
}
impl<T> ErrResult<T> for Result<T, Err> {
    fn log_err(self) {
        if let Err(err) = self {
            err.log();
        }
    }
    fn unwrap_res_or<F: Fn() -> T>(self, f: F) -> T {
        match self {
            Ok(v) => return v,
            Err(err) if !err.not_found => err.log(),
            Err(_) => {}
        }
        f()
    }
}

pub fn config_dir() -> PathBuf {
    let mut buf = dirs::config_dir().unwrap_or(PathBuf::new());
    buf.push("LogSimGUI");
    buf
}

pub fn open_file<T: AsRef<Path>>(path: &T) -> Result<fs::File, Err> {
    match fs::File::open(path) {
        Ok(file) => Ok(file),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            Err(Err::io_err(path, err).not_found())
        }
        Err(err) => Err(Err::io_err(path, err)),
    }
}
pub fn write_file<P: AsRef<Path>>(path: &P) -> Result<fs::File, Err> {
    match fs::File::create(path) {
        Ok(file) => Ok(file),
        Err(err) => Err(Err::io_err(path, err)),
    }
}

pub fn save_str(name: &str, s: &str) -> Result<(), Err> {
    let path = config_path(name);
    let mut file = write_file(&path)?;
    file.write_all(s.as_bytes())
        .map_err(|err| Err::io_err(path, err))
}
pub fn save_ron<P: AsRef<Path>, T: Serialize>(path: &P, value: &T) -> Result<(), Err> {
    let mut file = write_file(path)?;
    let string = ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::new())
        .map_err(|err| Err::new(path, err))?;
    file.write_all(string.as_bytes())
        .map_err(|err| Err::new(path, err))
}
pub fn save_data<P: AsRef<Path>, T: Serialize>(path: &P, value: &T) -> Result<(), Err> {
    let mut file = write_file(path)?;
    let bytes = bincode::serialize(value).map_err(|err| Err::new(path, err))?;
    file.write_all(&bytes).map_err(|err| Err::new(path, err))
}

pub fn load_ron<P, T>(path: &P) -> Result<T, Err>
where
    P: AsRef<Path>,
    T: for<'de> serde::de::Deserialize<'de>,
{
    let mut file = open_file(path)?;
    match ron::de::from_reader::<_, T>(&mut file) {
        Ok(v) => Ok(v),
        Err(_) => Err(Err::new(path, "Invalid RON")),
    }
}
pub fn load_data<P, T>(path: &P) -> Result<T, Err>
where
    P: AsRef<Path>,
    T: for<'de> serde::de::Deserialize<'de>,
{
    let mut file = open_file(path)?;
    match bincode::deserialize_from::<_, T>(&mut file) {
        Ok(v) => Ok(v),
        Err(_) => Err(Err::new(path, "Invalid data")),
    }
}

pub fn config_path(name: &str) -> String {
    let mut buf = config_dir();
    let config_path = buf.to_str().unwrap();
    match std::fs::create_dir(&buf) {
        Ok(_) => {}
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
        Err(err) => eprintln!("Failed to create config directory {config_path:?} : {err:?}"),
    }
    buf.push(name);
    String::from(buf.to_str().unwrap())
}

pub fn save_presets(presets: &mut Presets) -> Result<(), Err> {
    let mut path_buf = config_dir();
    path_buf.push("presets");
    let _ = std::fs::create_dir(&path_buf);

    let removed = presets.consume_removed();
    let dirty = presets.consume_dirty();
    for name in removed {
        path_buf.push(format!("{}.data", name));
        let _ = std::fs::remove_file(&path_buf);
        path_buf.pop();
    }
    for name in dirty {
        path_buf.push(format!("{}.data", name));
        let preset = presets.get_preset(&name).unwrap();
        save_data(&path_buf, preset)?;
        path_buf.pop();
    }
    let index: Vec<_> = presets
        .get()
        .iter()
        .map(|preset| format!("{}.data", preset.name))
        .collect();
    path_buf.push("__index.ron");
    save_ron(&path_buf, &index)
}
pub fn load_presets_at(path: &mut PathBuf) -> Result<Vec<DevicePreset>, Err> {
    let mut presets = Vec::new();
    let path_str = String::from(path.to_str().unwrap());

    let add_ctx = |e: Err| e.context(&format!("Failed to load presets at {path_str:?}"));

    path.push("__index.ron");
    let index: Vec<String> = load_ron(path).map_err(add_ctx)?;
    path.pop();
    for entry in index {
        path.push(entry);
        let preset: DevicePreset = load_data(path).map_err(add_ctx)?;
        presets.push(preset);
        path.pop();
    }
    Ok(presets)
}
pub fn load_presets() -> Result<Presets, Err> {
    let mut path_buf = config_dir();
    path_buf.push("presets");
    let presets = load_presets_at(&mut path_buf)?;
    Ok(Presets::new(presets))
}
pub fn save_settings(settings: &Settings) -> Result<(), Err> {
    save_ron(&config_path("settings.ron"), settings)
}
pub fn load_settings() -> Result<Settings, Err> {
    load_ron(&config_path("settings.ron"))
}
pub fn save_scene(scene: &Scene) -> Result<(), Err> {
    save_data(&config_path("scene.data"), scene)
}
pub fn load_scene() -> Result<Scene, Err> {
    load_data(&config_path("scene.data"))
}

pub fn reveal_dir(dir: &str) -> Result<(), String> {
    #[allow(unused_variables)]
    let cmd = "open";
    #[cfg(target_os = "windows")]
    let cmd = "explorer";
    #[cfg(target_os = "linux")]
    let cmd = "xdg-open";
    let output = process::Command::new(cmd).arg(dir).output().unwrap();
    if output.stderr.is_empty() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}
