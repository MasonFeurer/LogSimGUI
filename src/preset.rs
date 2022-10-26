pub mod chip;

use crate::{BitField, Color, DeviceVisuals, IntId, TruthTable};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Io {
    pub name: String,
    pub implicit: bool,
}
impl Io {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            implicit: false,
        }
    }
    pub fn from_names(implicit: bool, names: &[&str]) -> Vec<Self> {
        let mut result = Vec::with_capacity(names.len());
        for name in names {
            result.push(Self {
                name: String::from(*name),
                implicit,
            });
        }
        result
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CombGate {
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
pub enum PresetData {
    CombGate(CombGate),
    Chip(chip::Chip),
}
impl PresetData {
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
    pub fn inputs(&self) -> Vec<Io> {
        match self {
            Self::CombGate(e) => e.inputs.clone(),
            Self::Chip(e) => e.inputs.iter().map(|input| input.preset.clone()).collect(),
        }
    }
    pub fn outputs(&self) -> Vec<Io> {
        match self {
            Self::CombGate(e) => e.outputs.clone(),
            Self::Chip(e) => e.outputs.clone(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Preset {
    pub vis: DeviceVisuals,
    pub data: PresetData,
}

#[inline(always)]
fn key_index<K: PartialEq, V>(list: &[(K, V)], key: K) -> Option<usize> {
    list.iter().position(|(cmp_key, _)| *cmp_key == key)
}
#[inline(always)]
fn key_sort<K: PartialOrd + Ord, V>(list: &mut [(K, V)]) {
    list.sort_by(|(a_id, _), (b_id, _)| a_id.cmp(&b_id));
}

#[derive(Serialize, Deserialize)]
pub struct Cat {
    pub name: String,
    pub presets: Vec<(IntId, Preset)>,
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
    pub fn add_preset(&mut self, preset: Preset) -> IntId {
        let id = self.next_preset_id.get_inc();
        self.presets.push((id, preset));
        key_sort(&mut self.presets);
        id
    }
    pub fn remove_preset(&mut self, id: IntId) {
        let idx = key_index(&self.presets, id).unwrap();
        self.presets.remove(idx);
        key_sort(&mut self.presets);
    }

    pub fn get_preset(&self, id: IntId) -> Option<&Preset> {
        self.presets
            .iter()
            .find(|(cmp_id, _)| *cmp_id == id)
            .map(|(_, preset)| preset)
    }
    pub fn mut_preset(&mut self, id: IntId) -> Option<&mut Preset> {
        self.presets
            .iter_mut()
            .find(|(cmp_id, _)| *cmp_id == id)
            .map(|(_, preset)| preset)
    }
}

#[derive(Serialize, Deserialize)]
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
            name: String::from("Basic"),
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

    pub fn add_cat(&mut self, name: String) -> IntId {
        let id = self.next_cat_id.get_inc();
        self.cats.push((id, Cat::new(name)));
        key_sort(&mut self.cats);
        id
    }
    pub fn remove_cat(&mut self, id: IntId) {
        let idx = key_index(&self.cats, id).unwrap();
        self.cats.remove(idx);
        key_sort(&mut self.cats);
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

pub fn and_gate_preset() -> Preset {
    Preset {
        vis: DeviceVisuals {
            name: String::from("And"),
            color: Color::from_rgb(255, 0, 0),
        },
        data: PresetData::CombGate(CombGate {
            inputs: Io::from_names(true, &["a", "b"]),
            outputs: Io::from_names(true, &["out"]),
            table: and_truth_table(),
        }),
    }
}
pub fn not_gate_preset() -> Preset {
    Preset {
        vis: DeviceVisuals {
            name: String::from("Not"),
            color: Color::from_rgb(0, 255, 0),
        },
        data: PresetData::CombGate(CombGate {
            inputs: Io::from_names(true, &["in"]),
            outputs: Io::from_names(true, &["out"]),
            table: not_truth_table(),
        }),
    }
}

pub fn nand_gate_preset() -> Preset {
    Preset {
        vis: DeviceVisuals {
            name: String::from("Nand"),
            color: Color::from_rgb(0, 0, 255),
        },
        data: PresetData::CombGate(CombGate {
            inputs: Io::from_names(true, &["a", "b"]),
            outputs: Io::from_names(true, &["out"]),
            table: nand_truth_table(),
        }),
    }
}
pub fn nor_gate_preset() -> Preset {
    Preset {
        vis: DeviceVisuals {
            name: String::from("Nor"),
            color: Color::from_rgb(255, 255, 0),
        },
        data: PresetData::CombGate(CombGate {
            inputs: Io::from_names(true, &["a", "b"]),
            outputs: Io::from_names(true, &["out"]),
            table: nor_truth_table(),
        }),
    }
}
pub fn or_gate_preset() -> Preset {
    Preset {
        vis: DeviceVisuals {
            name: String::from("Or"),
            color: Color::from_rgb(0, 255, 255),
        },
        data: PresetData::CombGate(CombGate {
            inputs: Io::from_names(true, &["a", "b"]),
            outputs: Io::from_names(true, &["out"]),
            table: or_truth_table(),
        }),
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
