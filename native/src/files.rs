use logsim::{
    board::Board,
    old_data::OldDevicePreset,
    presets::{Change, DevicePreset, Library},
    settings::Settings,
};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::{fs, io, process};

pub fn save_settings(settings: &Settings) -> Result<(), FileErr> {
    save(&config_path("settings.ron"), Encoding::Ron, settings)
}
pub fn load_settings() -> Result<Settings, FileErr> {
    load(&config_path("settings.ron"), Encoding::Ron)
}

pub fn save_board(board: &Board) -> Result<(), FileErr> {
    save(&config_path("board.data"), Encoding::Data, board)
}
pub fn load_board() -> Result<Board, FileErr> {
    load(&config_path("board.data"), Encoding::Data)
}

pub fn save_library(library: &mut Library) -> Result<(), FileErr> {
    save_presets(&config_path("presets"), library)
}
pub fn load_library() -> Result<Library, FileErr> {
    load_presets(&config_path("presets"))
}

pub fn reveal_config_dir() -> Result<(), FileErr> {
    reveal_dir(&config_dir())
}

pub fn config_dir() -> PathBuf {
    let mut buf = dirs::config_dir().unwrap_or(PathBuf::new());
    buf.push("LogSimGUI");
    match fs::create_dir(&buf) {
        Ok(_) => {}
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
        Err(err) => FileErr::io(&buf, err)
            .context("Failed to create config directory")
            .log(),
    }
    buf
}
pub fn config_path(config_file: &str) -> PathBuf {
    let mut path_buf = config_dir();
    path_buf.push(config_file);
    path_buf
}

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

pub fn save<P, T>(path: &P, encoding: Encoding, value: &T) -> Result<(), FileErr>
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
    fs::write(path, bytes).map_err(|err| FileErr::io(path, err))
}
pub fn load<P, T>(path: &P, encoding: Encoding) -> Result<T, FileErr>
where
    P: AsRef<Path>,
    T: for<'de> serde::de::Deserialize<'de>,
{
    let bytes: Vec<_> = fs::read(path).map_err(|err| FileErr::io(path, err))?;
    match encoding {
        Encoding::Ron => {
            ron::de::from_bytes::<T>(&bytes).map_err(|_| FileErr::new(path, "Invalid RON"))
        }
        Encoding::Data => {
            bincode::deserialize::<T>(&bytes).map_err(|_| FileErr::new(path, "Invalid data"))
        }
    }
}
pub fn read_dir<P: AsRef<Path>, F: Fn(&PathBuf) -> bool>(
    path: &P,
    cond: F,
) -> Result<Vec<PathBuf>, FileErr> {
    let map_err = |err: io::Error| FileErr::io(path, err).context("Failed to read directory");
    let mut results = Vec::new();
    for entry in fs::read_dir(path).map_err(map_err)? {
        let entry = entry.unwrap();
        if cond(&entry.path()) {
            results.push(entry.path());
        }
    }
    Ok(results)
}

pub fn save_presets<P: AsRef<Path>>(path: &P, presets: &mut Library) -> Result<(), FileErr> {
    match fs::create_dir(&path) {
        Ok(_) => {}
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
        Err(err) => Err(FileErr::io(&path, err).context("Failed to create presets directory"))?,
    }

    let mut buf = PathBuf::from(path.as_ref());
    let changes = presets.consume_changes();
    for (preset, change) in changes {
        match change {
            Change::Added | Change::Modified => {
                buf.push(format!("{}.data", preset));
                let Some(preset) = presets.get_preset(&preset) else {
                	continue;
                };
                save(&buf, Encoding::Data, preset).log_err();
                buf.pop();
            }
            Change::Removed => {
                buf.push(format!("{}.data", preset));
                _ = fs::remove_file(&buf);
                buf.pop();
            }
        }
    }
    Ok(())
}
pub fn load_preset<P: AsRef<Path>>(path: &P, presets: &mut Library) -> Result<(), FileErr> {
    let add_ctx = |err: FileErr| err.context("Failed to load preset");

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
pub fn load_presets<P: AsRef<Path>>(path: &P) -> Result<Library, FileErr> {
    let mut presets = Library::new();

    let cond = |f: &PathBuf| Encoding::Data.file_matches(f);
    let add_ctx = |err: FileErr| err.context("Failed to load presets");

    for entry in read_dir(path, cond).map_err(add_ctx)? {
        load_preset(&entry, &mut presets)?;
    }
    Ok(presets)
}

pub fn reveal_dir<P: AsRef<Path>>(path: &P) -> Result<(), FileErr> {
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
        let err_msg = String::from_utf8_lossy(&output.stderr).into_owned();
        Err(FileErr::new(&path, err_msg))
    }
}

pub struct FileErr {
    // If the error was because of a missing resource
    not_found: bool,
    path: String,
    msg: String,
}
impl FileErr {
    pub fn io<P: AsRef<Path>>(path: P, err: std::io::Error) -> Self {
        Self {
            not_found: err.kind() != std::io::ErrorKind::NotFound,
            path: format!("{}", path.as_ref().display()),
            msg: format!("{:?}", err.kind()),
        }
    }
    pub fn new<P: AsRef<Path>, M: std::fmt::Debug>(path: P, msg: M) -> Self {
        Self {
            not_found: false,
            path: format!("{}", path.as_ref().display()),
            msg: format!("{msg:?}"),
        }
    }

    pub fn context(mut self, ctx: &str) -> Self {
        self.msg = format!("{} : {}", ctx, self.msg);
        self
    }
    pub fn log(self) {
        println!("{} ({:?})", self.msg, self.path);
    }
}

pub trait FileErrResult<T> {
    fn log_err(self) -> Option<T>;
}
impl<T> FileErrResult<T> for Result<T, FileErr> {
    fn log_err(self) -> Option<T> {
        match self {
            Self::Err(err) if !err.not_found => {
                err.log();
                None
            }
            Self::Err(_) => None,
            Self::Ok(ok) => Some(ok),
        }
    }
}
