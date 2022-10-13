pub mod chip;

use crate::{BitField, Color, IntId, TruthTable};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Io {
    pub y_pos: f32, // normalized
    pub width: u8,
    pub name: String,
    pub implicit: bool,
}
impl Io {
    pub fn default_at(y_pos: f32) -> Self {
        Self {
            y_pos,
            width: 1,
            name: String::new(),
            implicit: false,
        }
    }
    pub fn from_names(implicit: bool, names: &[&str]) -> Vec<Self> {
        let mut result = Vec::with_capacity(names.len());
        let y_step = 1.0_f32 / (names.len() + 1) as f32;
        let mut y_pos = y_step;

        for name in names {
            result.push(Self {
                y_pos,
                width: 1,
                name: String::from(*name),
                implicit,
            });
            y_pos += y_step;
        }
        result
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CombGate {
    pub name: String,
    pub color: Color,
    pub inputs: Vec<Io>,
    pub outputs: Vec<Io>,
    pub table: TruthTable,
}
impl CombGate {
    #[inline(always)]
    fn num_inputs(&self) -> usize {
        self.table.num_inputs as usize
    }
    #[inline(always)]
    fn num_outputs(&self) -> usize {
        self.table.num_outputs as usize
    }

    #[inline(always)]
    fn get_input(&self, input: usize) -> Option<&Io> {
        self.inputs.get(input)
    }
    #[inline(always)]
    fn get_output(&self, output: usize) -> Option<&Io> {
        self.outputs.get(output)
    }
}

#[derive(Serialize, Deserialize)]
pub enum Preset {
    CombGate(CombGate),
    Chip(chip::Chip),
}
impl Preset {
    pub fn name(&self) -> &str {
        match self {
            Self::CombGate(e) => &e.name,
            Self::Chip(e) => &e.name,
        }
    }

    pub fn color(&self) -> &Color {
        match self {
            Self::CombGate(e) => &e.color,
            Self::Chip(e) => &e.color,
        }
    }

    pub fn num_inputs(&self) -> usize {
        match self {
            Self::CombGate(e) => e.num_inputs(),
            Self::Chip(e) => e.num_inputs(),
        }
    }
    pub fn num_outputs(&self) -> usize {
        match self {
            Self::CombGate(e) => e.num_outputs(),
            Self::Chip(e) => e.num_outputs(),
        }
    }

    pub fn get_input(&self, input: usize) -> Option<&Io> {
        match self {
            Self::CombGate(e) => e.get_input(input),
            Self::Chip(e) => e.get_input(input),
        }
    }
    pub fn get_output(&self, output: usize) -> Option<&Io> {
        match self {
            Self::CombGate(e) => e.get_output(output),
            Self::Chip(e) => e.get_output(output),
        }
    }

    #[inline(always)]
    pub fn get_input_loc(&self, input: usize) -> Option<f32> {
        Some(self.get_input(input)?.y_pos)
    }
    #[inline(always)]
    pub fn get_output_loc(&self, output: usize) -> Option<f32> {
        Some(self.get_output(output)?.y_pos)
    }
}

#[derive(Serialize, Deserialize)]
pub struct CatPreset {
    pub cat: IntId,
    pub preset: Preset,
}
#[derive(Serialize, Deserialize)]
pub struct Presets {
    pub cats: HashMap<IntId, String>,
    pub next_cat_id: IntId,

    pub presets: HashMap<IntId, CatPreset>,
    pub next_preset_id: IntId,
}
impl Presets {
    pub fn new() -> Self {
        Self {
            cats: HashMap::new(),
            next_cat_id: IntId(0),

            presets: HashMap::new(),
            next_preset_id: IntId(0),
        }
    }
    pub fn defaults() -> Self {
        let mut new = Self::new();
        new.add_defaults();
        new
    }

    pub fn get_preset(&self, id: IntId) -> Option<&Preset> {
        self.presets.get(&id).map(|e| &e.preset)
    }
    pub fn get_comb_gate(&self, id: IntId) -> Option<&CombGate> {
        if let Some(preset) = self.get_preset(id) {
            if let Preset::CombGate(comb_gate) = preset {
                Some(comb_gate)
            } else {
                None
            }
        } else {
            None
        }
    }
    pub fn get_cat(&self, id: IntId) -> Option<&str> {
        self.cats.get(&id).map(String::as_str)
    }

    pub fn add_preset(&mut self, cat: IntId, preset: Preset) -> IntId {
        let id = self.next_preset_id.get_inc();
        self.presets.insert(id, CatPreset { cat, preset });
        id
    }

    pub fn remove_cat(&mut self, cat: IntId) {
        self.cats.remove(&cat);
        let mut remove_presets = Vec::new();
        for (preset_id, preset) in &self.presets {
            if preset.cat == cat {
                remove_presets.push(*preset_id);
            }
        }
        for preset in remove_presets {
            self.presets.remove(&preset);
        }
    }
    pub fn remove_preset(&mut self, preset: IntId) {
        self.presets.remove(&preset);
    }
    pub fn add_cat(&mut self, cat: String) -> IntId {
        let id = self.next_cat_id.get_inc();
        self.cats.insert(id, cat);
        id
    }

    pub fn add_defaults(&mut self) {
        let cat_id = self.add_cat("Basic".to_owned());
        for preset in [
            Preset::CombGate(and_gate()),
            Preset::CombGate(not_gate()),
            Preset::CombGate(nand_gate()),
            Preset::CombGate(nor_gate()),
            Preset::CombGate(or_gate()),
        ] {
            self.add_preset(cat_id, preset);
        }
    }

    #[inline(always)]
    pub fn get_cats(&self) -> Vec<(IntId, String)> {
        let mut cats: Vec<_> = self
            .cats
            .iter()
            .map(|(id, name)| (*id, name.clone()))
            .collect();
        cats.sort_by(|(a_id, _), (b_id, _)| a_id.cmp(&b_id));
        cats
    }
    pub fn get_cat_presets(&self, cat: IntId) -> Vec<(IntId, &Preset)> {
        let mut result = Vec::new();
        for (preset_id, preset) in &self.presets {
            if preset.cat == cat {
                result.push((*preset_id, &preset.preset));
            }
        }
        result
    }
}

pub fn and_gate() -> CombGate {
    CombGate {
        name: String::from("And"),
        color: Color::from_rgb(255, 0, 0),
        inputs: Io::from_names(true, &["a", "b"]),
        outputs: Io::from_names(true, &["out"]),
        table: TruthTable {
            num_inputs: 2,
            num_outputs: 1,
            map: vec![
                BitField::single(0), // 00
                BitField::single(0), // 01
                BitField::single(0), // 10
                BitField::single(1), // 11
            ],
        },
    }
}
pub fn not_gate() -> CombGate {
    CombGate {
        name: String::from("Not"),
        color: Color::from_rgb(0, 255, 0),
        inputs: Io::from_names(true, &["in"]),
        outputs: Io::from_names(true, &["out"]),
        table: TruthTable {
            num_inputs: 1,
            num_outputs: 1,
            map: vec![
                BitField::single(1), // 0
                BitField::single(0), // 1
            ],
        },
    }
}

pub fn nand_gate() -> CombGate {
    CombGate {
        name: String::from("Nand"),
        color: Color::from_rgb(0, 0, 255),
        inputs: Io::from_names(true, &["a", "b"]),
        outputs: Io::from_names(true, &["out"]),
        table: TruthTable {
            num_inputs: 2,
            num_outputs: 1,
            map: vec![
                BitField::single(1), // 00
                BitField::single(1), // 01
                BitField::single(1), // 10
                BitField::single(0), // 11
            ],
        },
    }
}
pub fn nor_gate() -> CombGate {
    CombGate {
        name: String::from("Nor"),
        color: Color::from_rgb(255, 255, 0),
        inputs: Io::from_names(true, &["a", "b"]),
        outputs: Io::from_names(true, &["out"]),
        table: TruthTable {
            num_inputs: 2,
            num_outputs: 1,
            map: vec![
                BitField::single(1), // 00
                BitField::single(0), // 01
                BitField::single(0), // 10
                BitField::single(0), // 11
            ],
        },
    }
}
pub fn or_gate() -> CombGate {
    CombGate {
        name: String::from("Or"),
        color: Color::from_rgb(0, 255, 255),
        inputs: Io::from_names(true, &["a", "b"]),
        outputs: Io::from_names(true, &["out"]),
        table: TruthTable {
            num_inputs: 2,
            num_outputs: 1,
            map: vec![
                BitField::single(0), // 00
                BitField::single(1), // 01
                BitField::single(1), // 10
                BitField::single(1), // 11
            ],
        },
    }
}
