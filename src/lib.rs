#![feature(let_chains)]

pub mod app;
pub mod graphics;
pub mod integration;
pub mod preset;
pub mod preset_placer;
pub mod scene;
pub mod settings;

use egui::{Pos2, Rect, Vec2};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tinyrand::{Rand, Seeded, StdRand};

pub use app::{App, AppItem, CreateApp};
pub use integration::{FrameInput, FrameOutput, Keybind};
pub use preset::{DevicePreset, PresetData, Presets};
pub use scene::Scene;
pub use settings::Settings;

lazy_static::lazy_static! {
    pub static ref RAND: Mutex<StdRand> = Mutex::new(StdRand::seed(2538292));
}

#[inline(always)]
pub fn rand_id() -> u64 {
    RAND.lock().unwrap().next_u64()
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum LinkTarget<T> {
    DeviceInput(T, usize),
    Output(T),
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LinkStart<T> {
    DeviceOutput(T, usize),
    Input(T),
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DeviceInput<T>(pub T, pub usize);
impl<T: Copy> DeviceInput<T> {
    pub fn wrap(&self) -> LinkTarget<T> {
        LinkTarget::DeviceInput(self.0, self.1)
    }
}

pub enum NewLink<T> {
    InputToDeviceInput(T, DeviceInput<T>),
    DeviceOutputTo(T, usize, LinkTarget<T>),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BitField {
    pub data: u64,
    len: usize,
}
impl BitField {
    #[inline(always)]
    pub fn single(bit: u8) -> Self {
        assert!(bit == 0 || bit == 1);
        Self {
            len: 1,
            data: bit as u64,
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
            data |= (bits[i] as u64) << i;
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
        self.data = (self.data & !(1 << pos as u64)) | ((state as u64) << pos);
    }
    #[inline(always)]
    pub fn get(&self, pos: usize) -> bool {
        assert!(pos < self.len);
        ((self.data >> pos as u64) & 1) == 1
    }
    #[inline(always)]
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

#[derive(Clone, Serialize, Deserialize)]
pub struct TruthTable {
    pub num_inputs: usize,
    pub num_outputs: usize,
    pub map: Vec<u64>,
}
impl TruthTable {
    #[inline(always)]
    pub fn get(&self, input: usize) -> BitField {
        BitField {
            len: self.num_outputs,
            data: self.map[input],
        }
    }
}
impl std::fmt::Debug for TruthTable {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut f = f.debug_struct("TruthTable");
        let mut input = 0;
        for output in &self.map {
            f.field(
                &format!("{:01$b}", input, self.num_inputs),
                &format!("{:01$b}", *output, self.num_outputs),
            );
            input += 1;
        }
        f.finish()
    }
}