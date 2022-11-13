pub mod chip;

use crate::settings::Settings;
use crate::{BitField, IntId, TruthTable};
pub use chip::ChipPreset;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct PinPreset {
    pub name: String,
}
impl PinPreset {
    pub fn unnamed(count: usize) -> Vec<Self> {
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            result.push(Self {
                name: String::new(),
            });
        }
        result
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CombGatePreset {
    pub inputs: Vec<PinPreset>,
    pub outputs: Vec<PinPreset>,
    pub table: TruthTable,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PresetData {
    CombGate(CombGatePreset),
    Chip(ChipPreset),
}
impl PresetData {
    pub fn num_inputs(&self) -> usize {
        match self {
            Self::CombGate(e) => e.inputs.len(),
            Self::Chip(e) => e.inputs.len(),
        }
    }
    pub fn num_outputs(&self) -> usize {
        match self {
            Self::CombGate(e) => e.outputs.len(),
            Self::Chip(e) => e.outputs.len(),
        }
    }

    pub fn get_input(&self, input: usize) -> &PinPreset {
        match self {
            Self::CombGate(e) => &e.inputs[input],
            Self::Chip(e) => &e.inputs[input],
        }
    }
    pub fn get_output(&self, output: usize) -> &PinPreset {
        match self {
            Self::CombGate(e) => &e.outputs[output],
            Self::Chip(e) => &e.outputs[output],
        }
    }

    pub fn inputs(&self) -> &[PinPreset] {
        match self {
            Self::CombGate(e) => &e.inputs,
            Self::Chip(e) => &e.inputs,
        }
    }
    pub fn outputs(&self) -> &[PinPreset] {
        match self {
            Self::CombGate(e) => &e.outputs,
            Self::Chip(e) => &e.outputs,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DevicePreset {
    pub name: String,
    pub color: [u8; 4],
    pub data: PresetData,
    pub src: PresetSource,
}
impl DevicePreset {
    #[inline(always)]
    pub fn size(&self, settings: &Settings) -> eframe::egui::Vec2 {
        settings.device_size(self.data.num_inputs(), self.data.num_outputs(), &self.name)
    }
}

#[inline(always)]
fn key_index<K: PartialEq, V>(list: &[(K, V)], key: K) -> Option<usize> {
    list.iter().position(|(cmp_key, _)| *cmp_key == key)
}
#[inline(always)]
fn key_sort<K: PartialOrd + Ord, V>(list: &mut [(K, V)]) {
    list.sort_by(|(a_id, _), (b_id, _)| a_id.cmp(&b_id));
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cat {
    pub name: String,
    pub presets: Vec<(IntId, DevicePreset)>,
    pub next_preset_id: IntId,
}
impl Cat {
    pub fn new(name: String) -> Self {
        Self {
            name,
            presets: Vec::new(),
            next_preset_id: IntId(0),
        }
    }
    pub fn add_preset(&mut self, preset: DevicePreset) -> IntId {
        if let Some(idx) = self.presets.iter().position(|(_, b)| b.name == preset.name) {
            self.presets[idx].1 = preset;
            self.presets[idx].0
        } else {
            let id = self.next_preset_id.get_inc();
            self.presets.push((id, preset));
            key_sort(&mut self.presets);
            id
        }
    }
    pub fn remove_preset(&mut self, id: IntId) {
        let idx = key_index(&self.presets, id).unwrap();
        self.presets.remove(idx);
        key_sort(&mut self.presets);
    }

    pub fn get_preset(&self, id: IntId) -> Option<&DevicePreset> {
        self.presets
            .iter()
            .find(|(cmp_id, _)| *cmp_id == id)
            .map(|(_, preset)| preset)
    }
    pub fn mut_preset(&mut self, id: IntId) -> Option<&mut DevicePreset> {
        self.presets
            .iter_mut()
            .find(|(cmp_id, _)| *cmp_id == id)
            .map(|(_, preset)| preset)
    }
}

const DEFAULT_CAT: &'static str = "Basic";

#[derive(Debug, Serialize, Deserialize)]
pub struct Presets {
    pub cats: Vec<(IntId, Cat)>,
    pub next_cat_id: IntId,
}
impl Presets {
    pub fn new() -> Self {
        Self {
            cats: Vec::new(),
            next_cat_id: IntId(0),
        }
    }
    pub fn default() -> Self {
        let mut cat = Cat {
            name: String::from(DEFAULT_CAT),
            presets: Vec::new(),
            next_preset_id: IntId(0),
        };
        for preset in [
            and_gate_preset(),
            not_gate_preset(),
            nand_gate_preset(),
            nor_gate_preset(),
            or_gate_preset(),
        ] {
            let id = cat.next_preset_id.get_inc();
            cat.presets.push((id, preset));
        }
        Self {
            cats: vec![(IntId(0), cat)],
            next_cat_id: IntId(1),
        }
    }
    pub fn merge(&mut self, other: &Self) {
        for cat in &other.cats {
            let Some(cat_idx) = self.cats.iter().position(|(_, n)| n.name == cat.1.name) else {
            	// cat doesn't already exist, just add it
            	self.cats.push((self.next_cat_id.get_inc(), cat.1.clone()));
            	continue;
            };
            // cat exists, so we must merge them

            for preset in &cat.1.presets {
                let idx = self.cats[cat_idx]
                    .1
                    .presets
                    .iter()
                    .position(|(_, n)| n.name == preset.1.name);
                if let Some(idx) = idx {
                    // preset already exists, override it
                    self.cats[cat_idx].1.presets[idx].1 = preset.1.clone();
                } else {
                    // preset is new, so add it
                    self.cats[cat_idx].1.add_preset(preset.1.clone());
                }
            }
        }
    }

    pub fn add_cat(&mut self, name: String) -> Option<IntId> {
        if name.trim().is_empty() {
            return None;
        }
        if self
            .cats
            .iter()
            .find(|(_, b)| b.name.as_str() == name)
            .is_some()
        {
            eprintln!("can't re-use category names");
            return None;
        }

        let id = self.next_cat_id.get_inc();
        self.cats.push((id, Cat::new(name)));
        key_sort(&mut self.cats);
        Some(id)
    }
    pub fn remove_cat(&mut self, id: IntId) -> bool {
        if self.cats.len() == 1 {
            eprintln!("can't remove last category");
            return false;
        }

        let idx = key_index(&self.cats, id).unwrap();

        if self.cats[idx].1.name.as_str() == DEFAULT_CAT {
            eprintln!("can't remove default category");
            return false;
        }
        if self.cats[idx].1.presets.len() != 0 {
            eprintln!("can't remove a category with presets in it");
            return false;
        }

        self.cats.remove(idx);
        key_sort(&mut self.cats);
        true
    }

    pub fn get_cat(&self, id: IntId) -> Option<&Cat> {
        self.cats
            .iter()
            .find(|(cmp_id, _)| *cmp_id == id)
            .map(|(_, cat)| cat)
    }
    pub fn mut_cat(&mut self, id: IntId) -> Option<&mut Cat> {
        self.cats
            .iter_mut()
            .find(|(cmp_id, _)| *cmp_id == id)
            .map(|(_, cat)| cat)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PresetSource {
    BuiltIn,
    Scene(Option<crate::scene::SceneLayout>),
    // Code(), // design chips with some coding language?
}

pub fn and_gate_preset() -> DevicePreset {
    DevicePreset {
        name: String::from("And"),
        color: [255, 0, 0, 255],
        data: PresetData::CombGate(CombGatePreset {
            inputs: PinPreset::unnamed(2),
            outputs: PinPreset::unnamed(1),
            table: and_truth_table(),
        }),
        src: PresetSource::BuiltIn,
    }
}
pub fn not_gate_preset() -> DevicePreset {
    DevicePreset {
        name: String::from("Not"),
        color: [0, 255, 0, 255],
        data: PresetData::CombGate(CombGatePreset {
            inputs: PinPreset::unnamed(1),
            outputs: PinPreset::unnamed(1),
            table: not_truth_table(),
        }),
        src: PresetSource::BuiltIn,
    }
}

pub fn nand_gate_preset() -> DevicePreset {
    DevicePreset {
        name: String::from("Nand"),
        color: [0, 0, 255, 255],
        data: PresetData::CombGate(CombGatePreset {
            inputs: PinPreset::unnamed(2),
            outputs: PinPreset::unnamed(1),
            table: nand_truth_table(),
        }),
        src: PresetSource::BuiltIn,
    }
}
pub fn nor_gate_preset() -> DevicePreset {
    DevicePreset {
        name: String::from("Nor"),
        color: [255, 255, 0, 255],
        data: PresetData::CombGate(CombGatePreset {
            inputs: PinPreset::unnamed(2),
            outputs: PinPreset::unnamed(1),
            table: nor_truth_table(),
        }),
        src: PresetSource::BuiltIn,
    }
}
pub fn or_gate_preset() -> DevicePreset {
    DevicePreset {
        name: String::from("Or"),
        color: [0, 255, 255, 255],
        data: PresetData::CombGate(CombGatePreset {
            inputs: PinPreset::unnamed(2),
            outputs: PinPreset::unnamed(1),
            table: or_truth_table(),
        }),
        src: PresetSource::BuiltIn,
    }
}

pub fn and_truth_table() -> TruthTable {
    TruthTable {
        num_inputs: 2,
        num_outputs: 1,
        map: vec![
            BitField::single(0), // 00
            BitField::single(0), // 01
            BitField::single(0), // 10
            BitField::single(1), // 11
        ],
    }
}
pub fn not_truth_table() -> TruthTable {
    TruthTable {
        num_inputs: 1,
        num_outputs: 1,
        map: vec![
            BitField::single(1), // 0
            BitField::single(0), // 1
        ],
    }
}

pub fn nand_truth_table() -> TruthTable {
    TruthTable {
        num_inputs: 2,
        num_outputs: 1,
        map: vec![
            BitField::single(1), // 00
            BitField::single(1), // 01
            BitField::single(1), // 10
            BitField::single(0), // 11
        ],
    }
}
pub fn nor_truth_table() -> TruthTable {
    TruthTable {
        num_inputs: 2,
        num_outputs: 1,
        map: vec![
            BitField::single(1), // 00
            BitField::single(0), // 01
            BitField::single(0), // 10
            BitField::single(0), // 11
        ],
    }
}
pub fn or_truth_table() -> TruthTable {
    TruthTable {
        num_inputs: 2,
        num_outputs: 1,
        map: vec![
            BitField::single(0), // 00
            BitField::single(1), // 01
            BitField::single(1), // 10
            BitField::single(1), // 11
        ],
    }
}
