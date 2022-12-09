use log_sim_gui::old_data::OldDevicePreset;
use log_sim_gui::preset::{Change, DevicePreset, Presets};
use log_sim_gui::scene::Scene;
use log_sim_gui::settings::Settings;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::{fmt, fs, io, process};

pub enum Encoding {
    Ron,
    Data,
}
impl Encoding {
    pub fn file_matches<P: AsRef<Path>>(&self, path: &P) -> bool {
        let end = match self {
            &Self::Ron => ".ron",
            &Self::Data => ".data",
        };
        let Some(file_name) = path.as_ref().file_name() else {
        	return false;
        };
        file_name.to_str().unwrap().ends_with(end)
    }
}

#[derive(Clone)]
pub struct Err {
    // If the error was because of a not-found file
    not_found: bool,
    path: String,
    msg: String,
}
impl Err {
    fn io_err<P: AsRef<Path>>(path: P, err: io::Error) -> Self {
        Self {
            not_found: false,
            path: format!("{}", path.as_ref().display()),
            msg: format!("{:?}", err.kind()),
        }
    }
    fn new<P: AsRef<Path>, M: fmt::Debug>(path: P, msg: M) -> Self {
        Self {
            not_found: false,
            path: format!("{}", path.as_ref().display()),
            msg: format!("{msg:?}"),
        }
    }

    fn not_found(mut self, value: bool) -> Self {
        self.not_found = value;
        self
    }

    pub fn context(mut self, ctx: &str) -> Self {
        self.msg = format!("{} : {}", ctx, self.msg);
        self
    }
    pub fn log(self) {
        println!("{} ({:?})", self.msg, self.path);
    }
}

macro_rules! err {
	($path:expr,$($t:tt)*) => {{
		let msg = format_args!($($t)*).to_string();
		Err {
            not_found: false,
            path: format!("{}", $path.as_ref().display()),
            msg: format!("{msg:?}"),
        }
	}};
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
        // If the resource was not found, use the default
        // (we don't care if it wasn't found because that's
        // what happens when you run the app for the first time)
        // If reading the resource returned an error, log it
        // Otherwise, use the loaded resource
        match self {
            Ok(v) => return v,
            Err(err) if !err.not_found => err.log(),
            Err(_) => {}
        }
        f()
    }
}

pub fn save<P, T>(path: &P, value: &T, encoding: Encoding) -> Result<(), Err>
where
    P: AsRef<Path>,
    T: Serialize,
{
    let bytes = match encoding {
        Encoding::Ron => ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::new())
            .unwrap()
            .into_bytes(),
        Encoding::Data => bincode::serialize(value).unwrap(),
    };
    fs::write(path, bytes).map_err(|err| Err::new(path, err))
}
pub fn load<P, T>(path: &P, encoding: Encoding) -> Result<T, Err>
where
    P: AsRef<Path>,
    T: for<'de> serde::de::Deserialize<'de>,
{
    let bytes = fs::read(path).map_err(|err| Err::io_err(path, err))?;
    match encoding {
        Encoding::Ron => {
            ron::de::from_bytes::<T>(&bytes).map_err(|_| Err::new(path, "Invalid RON"))
        }
        Encoding::Data => {
            bincode::deserialize::<T>(&bytes).map_err(|_| Err::new(path, "Invalid data"))
        }
    }
}

pub fn persist_dir() -> PathBuf {
    let mut buf = dirs::config_dir().unwrap_or(PathBuf::new());
    buf.push("LogSimGUI");
    match fs::create_dir(&buf) {
        Ok(_) => {}
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
        Err(err) => Err::io_err(&buf, err)
            .context("Failed to create config directory")
            .log(),
    }
    buf
}

pub fn save_presets(presets: &mut Presets) -> Result<(), Err> {
    let mut path = persist_dir();
    path.push("presets");
    match fs::create_dir(&path) {
        Ok(_) => {}
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
        Err(err) => {
            return Err(Err::io_err(&path, err).context("Failed to create presets directory"))
        }
    }

    let changes = presets.consume_changes();
    for change in changes {
        match change {
            Change::Added(name) | Change::Modified(name) => {
                path.push(format!("{}.data", name));
                let preset = presets.get_preset(&name).unwrap();
                save(&path, preset, Encoding::Data)?;
                path.pop();
            }
            Change::Removed(name) => {
                path.push(format!("{}.data", name));
                let _ = fs::remove_file(&path);
                path.pop();
            }
        }
    }
    Ok(())
}
pub fn read_dir<P: AsRef<Path>, F: Fn(&PathBuf) -> bool>(
    path: &P,
    cond: F,
) -> Result<Vec<PathBuf>, Err> {
    let map_err = |err: io::Error| {
        err!(path, "Failed to read directory").not_found(err.kind() == io::ErrorKind::NotFound)
    };
    let mut results = Vec::new();
    for entry in fs::read_dir(path).map_err(map_err)? {
        let entry = entry.unwrap();
        if cond(&entry.path()) {
            results.push(entry.path());
        }
    }
    Ok(results)
}
// returns the Device preset and if it is dirty (should be re-saved)
pub fn load_preset<P: AsRef<Path>>(path: &P, presets: &mut Presets) -> Result<(), Err> {
    let add_ctx = |err: Err| err.context("Failed to load preset");

    let preset: Result<DevicePreset, _> = load(path, Encoding::Data).map_err(add_ctx);
    let old_preset: Result<OldDevicePreset, _> = load(path, Encoding::Data);

    match (preset, old_preset) {
        (Ok(preset), _) => {
            presets.add_preset(preset, false);
        }
        (_, Ok(old_preset)) => {
            presets.add_preset(old_preset.update(), true);
        }
        (Err(err), _) => return Err(err),
    }
    Ok(())
}
pub fn load_presets_in<P: AsRef<Path>>(path: &P) -> Result<Presets, Err> {
    let mut presets = Presets::new();

    let cond = |f: &PathBuf| Encoding::Data.file_matches(f);
    let add_ctx = |err: Err| err.context("Failed to load presets");

    for entry in read_dir(path, cond).map_err(add_ctx)? {
        load_preset(&entry, &mut presets)?;
    }
    Ok(presets)
}
pub fn load_presets() -> Result<Presets, Err> {
    let mut path = persist_dir();
    path.push("presets");
    load_presets_in(&path)
}

pub fn save_settings(settings: &Settings) -> Result<(), Err> {
    let mut path = persist_dir();
    path.push("settings.ron");
    save(&path, settings, Encoding::Ron)
}
pub fn load_settings() -> Result<Settings, Err> {
    let mut path = persist_dir();
    path.push("settings.ron");
    load(&path, Encoding::Ron)
}

pub fn save_scene(scene: &Scene) -> Result<(), Err> {
    let mut path = persist_dir();
    path.push("scene.data");
    save(&path, scene, Encoding::Data)
}
pub fn load_scene() -> Result<Scene, Err> {
    let mut path = persist_dir();
    path.push("scene.data");
    load(&path, Encoding::Data)
}

pub fn reveal_dir<P: AsRef<Path>>(path: &P) -> Result<(), Err> {
    let path = path.as_ref().to_str().unwrap();
    #[allow(unused_variables)]
    let cmd = "open";
    #[cfg(target_os = "windows")]
    let cmd = "explorer";
    #[cfg(target_os = "linux")]
    let cmd = "xdg-open";
    let output = process::Command::new(cmd).arg(path).output().unwrap();
    if output.stderr.is_empty() {
        Ok(())
    } else {
        Err(Err::new(
            &path,
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ))
    }
}
