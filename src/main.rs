#![feature(let_chains)]

// TODO (small) : impl speed adjustment

// TODO (big) : allow for changing Io config (name, width, implicit?) in scene inputs/outputs
//  a button next to the io, that opens a menu for such edits

pub mod app;
pub mod debug;
pub mod graphics;
pub mod preset;
pub mod scene;

use serde::{Deserialize, Serialize};

pub use eframe::egui::{Color32 as Color, Pos2, Rect, Vec2};

#[derive(Debug, Clone)]
pub struct IoDef {
    pub y: f32,
    pub h: f32,
    pub base_x: f32,
    pub tip_x: f32,
}
impl IoDef {
    pub fn rect(&self) -> Rect {
        let (y0, y1) = (self.y - self.h * 0.5, self.y + self.h * 0.5);

        if self.base_x < self.tip_x {
            Rect::from_min_max(Pos2::new(self.base_x, y0), Pos2::new(self.tip_x, y1))
        } else {
            Rect::from_min_max(Pos2::new(self.tip_x, y0), Pos2::new(self.base_x, y1))
        }
    }

    #[inline(always)]
    pub fn tip_loc(&self) -> Pos2 {
        Pos2::new(self.tip_x, self.y)
    }
    #[inline(always)]
    pub fn base_loc(&self) -> Pos2 {
        Pos2::new(self.base_x, self.y)
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
    pub fn get(&self, input: BitField) -> BitField {
        self.map[input.data as usize]
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WithLinks<T, L> {
    pub item: T,
    pub links: Vec<LinkTarget<L>>,
}
impl<T, L: Clone> WithLinks<T, L> {
    #[inline(always)]
    pub fn none(item: T) -> Self {
        Self {
            item,
            links: Vec::new(),
        }
    }
    #[inline(always)]
    pub fn map_item<N>(&self, map: impl FnOnce(&T) -> N) -> WithLinks<N, L> {
        WithLinks {
            item: map(&self.item),
            links: self.links.clone(),
        }
    }
}

fn main() {
    let native_options = eframe::NativeOptions::default();

    eframe::run_native(
        "Logic Gate Sim",
        native_options,
        Box::new(|_cc| Box::new(app::App::new())),
    );
}
