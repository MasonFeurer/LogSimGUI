#![feature(let_chains)]

// TODO (big) : allow for changing Io config (name, width, implicit?) in scene inputs/outputs
//  a button next to the io, that opens a menu for such edits

pub mod app;
pub mod debug;
pub mod graphics;
pub mod preset;
pub mod scene;

use serde::{Deserialize, Serialize};

pub use eframe::egui::{Color32 as Color, Pos2, Rect, Vec2};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeviceVisuals {
    pub name: String,
    pub color: Color,
}

#[derive(Debug, Clone)]
pub enum NewLink<T> {
    InputToDeviceInput(T, DeviceInput<T>),
    DeviceOutputTo(T, usize, LinkTarget<T>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInput<T>(pub T, pub usize);
impl<T: Copy> DeviceInput<T> {
    pub fn wrap(&self) -> LinkTarget<T> {
        LinkTarget::DeviceInput(self.0, self.1)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum IoDir {
    Left,
    Right,
}
#[derive(Debug, Clone, Copy)]
pub enum IoSize {
    Small,
    Large,
}

#[derive(Debug, Clone)]
pub struct IoDef {
    pub pos: Pos2,
    pub size: IoSize,
    pub dir: IoDir,
}
impl IoDef {
    #[inline(always)]
    fn real_size(&self, settings: &graphics::Settings) -> Vec2 {
        match self.size {
            IoSize::Small => settings.small_io_size,
            IoSize::Large => settings.large_io_size,
        }
    }

    pub fn rect(&self, settings: &graphics::Settings) -> Rect {
        let size = self.real_size(settings);
        let (y0, y1) = (self.pos.y - size.y * 0.5, self.pos.y + size.y * 0.5);

        match self.dir {
            IoDir::Left => Rect {
                min: Pos2::new(self.pos.x - size.x, y0),
                max: Pos2::new(self.pos.x, y1),
            },
            IoDir::Right => Rect {
                min: Pos2::new(self.pos.x, y0),
                max: Pos2::new(self.pos.x + size.x, y1),
            },
        }
    }

    #[inline(always)]
    pub fn tip_loc(&self, settings: &graphics::Settings) -> Pos2 {
        let size = self.real_size(settings);
        match self.dir {
            IoDir::Left => Pos2::new(self.pos.x - size.x, self.pos.y),
            IoDir::Right => Pos2::new(self.pos.x + size.x, self.pos.y),
        }
    }
}

// damn those derives tho
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IntId(u32);
impl IntId {
    #[inline(always)]
    pub fn new() -> Self {
        Self(fastrand::u32(..))
    }
    #[inline(always)]
    pub fn get_inc(&mut self) -> Self {
        let r = *self;
        self.0 += 1;
        r
    }
}
impl std::hash::Hash for IntId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.0, state)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BitField {
    pub len: u8,
    pub data: u32,
}
impl BitField {
    #[inline(always)]
    pub fn single(bit: u8) -> Self {
        Self {
            len: 1,
            data: bit as u32,
        }
    }
    #[inline(always)]
    pub fn empty(len: u8) -> Self {
        Self { len, data: 0 }
    }
    pub fn from_bits(bits: &[u8]) -> Self {
        assert!(bits.len() <= 32);
        let mut data = 0;
        for i in 0..bits.len() {
            data |= (bits[i] as u32) << i;
        }
        Self {
            len: bits.len() as u8,
            data,
        }
    }

    #[inline(always)]
    pub fn set(&mut self, pos: u8, state: bool) {
        assert!(pos < self.len);
        self.data = (self.data & !(1 << pos as u32)) | ((state as u32) << pos);
    }
    #[inline(always)]
    pub fn get(&self, pos: u8) -> bool {
        assert!(pos < self.len);
        ((self.data >> pos as u32) & 1) == 1
    }

    pub fn bits(self) -> Vec<bool> {
        let mut bits = Vec::with_capacity(self.len as usize);
        for i in 0..self.len {
            bits.push(self.get(i));
        }
        bits
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruthTable {
    pub num_inputs: u8,
    pub num_outputs: u8,
    pub map: Vec<BitField>,
}
impl TruthTable {
    #[inline(always)]
    pub fn get(&self, input: usize) -> BitField {
        self.map[input]
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum LinkTarget<T> {
    DeviceInput(T, usize),
    Output(T),
}
#[derive(Clone, Debug)]
pub enum LinkStart<T> {
    DeviceOutput(T, usize),
    Input(T),
}

fn main() {
    let native_options = eframe::NativeOptions::default();

    eframe::run_native(
        "Logic Gate Sim",
        native_options,
        Box::new(|_cc| Box::new(app::App::new())),
    );
}
