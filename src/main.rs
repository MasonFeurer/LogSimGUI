#![feature(let_chains)]

pub mod app;
pub mod graphics;
pub mod preset;
pub mod scene;
pub mod settings;

use crate::preset::PinPreset;
use crate::settings::Settings;
pub use eframe::egui::{Color32 as Color, Pos2, Rect, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeviceVisuals {
    pub name: String,
    pub color: [u8; 4],
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

#[derive(Clone)]
pub struct Pin<'a> {
    pub origin: Pos2,
    pub left: bool,
    pub large: bool,
    pub name: &'a str,
}
impl<'a> Pin<'a> {
    #[inline(always)]
    pub fn size(&self, settings: &Settings) -> Vec2 {
        match self.large {
            true => settings.scene_pin_size.into(),
            false => settings.device_pin_size.into(),
        }
    }
    #[inline(always)]
    pub fn rect(&self, settings: &Settings) -> Rect {
        let size = self.size(settings);
        let min = match self.left {
            true => self.origin - Vec2::new(size.x, size.y * 0.5),
            false => self.origin - Vec2::new(0.0, size.y * 0.5),
        };
        Rect::from_min_size(min, size)
    }
    #[inline(always)]
    pub fn tip(&self, settings: &Settings) -> Pos2 {
        let (origin, size) = (self.origin, self.size(settings));
        match self.left {
            false => Pos2::new(origin.x + size.x, origin.y),
            true => Pos2::new(origin.x - size.x, origin.y),
        }
    }

    pub fn spread_presets(
        presets: &'a [PinPreset],
        pos: Pos2,
        h: f32,
        left: bool,
        large: bool,
    ) -> Vec<Self> {
        let mut out = Vec::with_capacity(presets.len());
        let step = h / (presets.len() + 1) as f32;

        let mut temp_y = pos.y + step;
        for i in 0..presets.len() {
            out.push(Self {
                origin: Pos2::new(pos.x, temp_y),
                left,
                large,
                name: presets[i].name.as_str(),
            });
            temp_y += step;
        }
        out
    }
}

pub fn spread_values(count: usize, from: f32, to: f32) -> Vec<f32> {
    let mut out = vec![f32::from_bits(0); count];

    let dist = (from - to).abs();
    let step = dist / (count + 1) as f32;

    let mut temp = from + step;
    for i in 0..count {
        out[i] = temp;
        temp += step;
    }
    out
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
    len: usize,
    pub data: u32,
}
impl BitField {
    #[inline(always)]
    pub fn single(bit: u8) -> Self {
        assert!(bit == 0 || bit == 1);
        Self {
            len: 1,
            data: bit as u32,
        }
    }
    #[inline(always)]
    pub fn empty(len: usize) -> Self {
        assert!(len <= 32);
        Self { len, data: 0 }
    }
    pub fn from_bits(bits: &[u8]) -> Self {
        assert!(bits.len() <= 32);
        let mut data = 0;
        for i in 0..bits.len() {
            data |= (bits[i] as u32) << i;
        }
        Self {
            len: bits.len(),
            data,
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn set(&mut self, pos: usize, state: bool) {
        assert!(pos < self.len);
        self.data = (self.data & !(1 << pos as u32)) | ((state as u32) << pos);
    }
    #[inline(always)]
    pub fn get(&self, pos: usize) -> bool {
        assert!(pos < self.len);
        ((self.data >> pos as u32) & 1) == 1
    }
    pub fn any_on(&self) -> bool {
        self.data.count_ones() > 0
    }

    pub fn bits(self) -> Vec<bool> {
        let mut bits = Vec::with_capacity(self.len);
        for i in 0..self.len {
            bits.push(self.get(i));
        }
        bits
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruthTable {
    pub num_inputs: usize,
    pub num_outputs: usize,
    pub map: Vec<BitField>,
}
impl TruthTable {
    #[inline(always)]
    pub fn get(&self, input: usize) -> BitField {
        self.map[input]
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
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
