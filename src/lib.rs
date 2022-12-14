#![feature(let_chains)]

pub mod app;
pub mod dev;
pub mod graphics;
pub mod integration;
pub mod old_data;
pub mod preset;
pub mod preset_placer;
pub mod scene;
pub mod settings;

use egui::{Color32, Pos2, Rect, Vec2};
use serde::{Deserialize, Serialize};

pub use app::{App, AppItem, CreateApp};
pub use integration::{FrameInput, FrameOutput, Keybind};
pub use preset::{DevicePreset, PresetData, Presets};
pub use scene::Scene;
pub use settings::Settings;

const ON_V: u8 = 200;
const OFF_V: u8 = 100;

#[rustfmt::skip]
pub const LINK_COLORS: &[[Color32; 2]] = &[
	[Color32::from_rgb(OFF_V, 0, 0), Color32::from_rgb(ON_V, 0, 0)],
    [Color32::from_rgb(OFF_V, OFF_V, OFF_V), Color32::from_rgb(ON_V, ON_V, ON_V)],
    [Color32::from_rgb(0, OFF_V, 0), Color32::from_rgb(0, ON_V, 0)],
    [Color32::from_rgb(0, 0, OFF_V), Color32::from_rgb(0, 0, ON_V)],
    [Color32::from_rgb(OFF_V, OFF_V, 0), Color32::from_rgb(ON_V, ON_V, 0)],
    [Color32::from_rgb(OFF_V, 0, OFF_V), Color32::from_rgb(ON_V, 0, ON_V)],
    [Color32::from_rgb(0, OFF_V, OFF_V), Color32::from_rgb(0, ON_V, ON_V)],
];
const NUM_LINK_COLORS: usize = LINK_COLORS.len();

pub fn rand_link_color() -> usize {
    let mut bytes = [0];
    _ = getrandom::getrandom(&mut bytes);
    let [byte] = bytes;
    // byte is in 0..=255
    // color should be in 0..=NUM_LINK_COLORS-1

    let norm = byte as f32 / 255.0;
    assert!(norm >= 0.0 && norm <= 1.0);
    (norm * (NUM_LINK_COLORS) as f32) as usize
}

#[inline(always)]
pub fn rand_id() -> u64 {
    let mut bytes = [0; 8];
    getrandom::getrandom(&mut bytes).unwrap();
    u64::from_le_bytes(bytes)
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Link {
    pub target: LinkTarget<u64>,
    pub anchors: Vec<Pos2>,
    pub color: usize,
}
impl Link {
    pub fn new(target: LinkTarget<u64>, color: usize, anchors: Vec<Pos2>) -> Self {
        Self {
            target,
            anchors,
            color,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BitField {
    pub data: u64,
    len: usize,
}
impl BitField {
    pub const fn empty(len: usize) -> Self {
        assert!(len <= 64);
        Self { len, data: 0 }
    }

    // NOTE: hot code!
    #[inline(always)]
    pub fn set(&mut self, pos: usize, state: bool) {
        debug_assert!(pos < self.len);
        self.data = (self.data & !(1 << pos as u64)) | ((state as u64) << pos);
    }
    // NOTE: hot code!
    #[inline(always)]
    pub fn get(&self, pos: usize) -> bool {
        debug_assert!(pos < self.len);
        ((self.data >> pos as u64) & 1) == 1
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TruthTable {
    pub num_inputs: usize,
    pub num_outputs: usize,
    pub map: Vec<u64>,
}
impl TruthTable {
    // NOTE: hot code!
    #[inline(always)]
    pub fn get(&self, input: usize) -> BitField {
        BitField {
            len: self.num_outputs,
            data: self.map[input],
        }
    }
}
use std::fmt;
impl fmt::Debug for TruthTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

pub struct ChangedOutputs {
    prev_output: u64,
    new_output: u64,
    len: usize,
    index: usize,
}
impl ChangedOutputs {
    #[inline(always)]
    pub const fn new(prev: BitField, new: BitField) -> Self {
        debug_assert!(prev.len == new.len);
        Self {
            prev_output: prev.data,
            new_output: new.data,
            len: prev.len,
            index: 0,
        }
    }
    #[inline(always)]
    pub const fn none() -> Self {
        Self {
            prev_output: 0,
            new_output: 0,
            len: 0,
            index: 0,
        }
    }

    #[inline(always)]
    pub fn next(&mut self) -> Option<(usize, bool)> {
        while self.index < self.len {
            let idx = self.index;
            let prev_bit = (self.prev_output >> idx as u64) & 1;
            let new_bit = (self.new_output >> idx as u64) & 1;
            self.index += 1;
            if prev_bit != new_bit {
                return Some((idx, new_bit == 1));
            }
        }
        None
    }
}
pub struct ChangedOutput {
    pub output: usize,
    pub state: bool,
}
