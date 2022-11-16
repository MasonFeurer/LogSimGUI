pub mod chip;

use crate::settings::Settings;
use crate::{BitField, TruthTable};
pub use chip::ChipPreset;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CombGatePreset {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub table: TruthTable,
}
impl CombGatePreset {
    pub fn from_scene(scene: &mut crate::scene::Scene) -> Result<Self, String> {
        // create truth table from scene
        let num_inputs = scene.inputs.len();
        let num_outputs = scene.outputs.len();

        let mut inputs: Vec<_> = scene.inputs.keys().cloned().collect();
        let mut outputs: Vec<_> = scene.outputs.keys().cloned().collect();
        inputs.sort_by(|a, b| {
            let a_y = scene.inputs.get(a).unwrap().y_pos;
            let b_y = scene.inputs.get(b).unwrap().y_pos;
            a_y.partial_cmp(&b_y).unwrap()
        });
        outputs.sort_by(|a, b| {
            let a_y = scene.outputs.get(a).unwrap().y_pos;
            let b_y = scene.outputs.get(b).unwrap().y_pos;
            a_y.partial_cmp(&b_y).unwrap()
        });

        let mut total_states = if inputs.is_empty() { 0 } else { 1 };
        for _ in 0..inputs.len() {
            total_states *= 2;
        }
        let mut output_states = Vec::with_capacity(total_states);

        println!("total states: {}", total_states);

        // I do not care to support combination gates with 32+ inputs
        let mut input: u32 = 0;
        while (input >> num_inputs as u32) == 0 {
            // set inputs
            for i in 0..num_inputs {
                let state = ((input >> i as u32) & 1) == 1;
                scene.set_input(inputs[i], state);
            }

            // execute queued writes
            let mut total_updates = 0;
            while scene.write_queue.0.len() > 0 {
                total_updates += 1;
                scene.update();
                if total_updates > 1000 {
                    return Err("Has a loop, or is too big".to_owned());
                }
            }

            // store output
            let mut output = BitField::empty(num_outputs);
            for i in 0..num_outputs {
                let state = scene.outputs.get(&outputs[i]).unwrap().state;
                output.set(i, state);
            }
            output_states.push(output);

            input += 1;
        }

        let inputs = inputs
            .into_iter()
            .map(|id| scene.inputs.get(&id).unwrap().name.clone())
            .collect();
        let outputs = outputs
            .into_iter()
            .map(|id| scene.outputs.get(&id).unwrap().name.clone())
            .collect();
        Ok(Self {
            inputs,
            outputs,
            table: TruthTable {
                num_inputs,
                num_outputs,
                map: output_states,
            },
        })
    }
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

    pub fn inputs(&self) -> &[String] {
        match self {
            Self::CombGate(e) => &e.inputs,
            Self::Chip(e) => &e.inputs,
        }
    }
    pub fn outputs(&self) -> &[String] {
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
    pub presets: Vec<(u64, DevicePreset)>,
    pub next_preset_id: u64,
}
impl Cat {
    pub fn new(name: String) -> Self {
        Self {
            name,
            presets: Vec::new(),
            next_preset_id: 0,
        }
    }
    pub fn add_preset(&mut self, preset: DevicePreset) -> u64 {
        if let Some(idx) = self.presets.iter().position(|(_, b)| b.name == preset.name) {
            self.presets[idx].1 = preset;
            self.presets[idx].0
        } else {
            let id = self.next_preset_id;
            self.next_preset_id += 1;
            self.presets.push((id, preset));
            key_sort(&mut self.presets);
            id
        }
    }
    pub fn remove_preset(&mut self, id: u64) {
        let idx = key_index(&self.presets, id).unwrap();
        self.presets.remove(idx);
        key_sort(&mut self.presets);
    }

    pub fn get_preset(&self, id: u64) -> Option<&DevicePreset> {
        self.presets
            .iter()
            .find(|(cmp_id, _)| *cmp_id == id)
            .map(|(_, preset)| preset)
    }
    pub fn mut_preset(&mut self, id: u64) -> Option<&mut DevicePreset> {
        self.presets
            .iter_mut()
            .find(|(cmp_id, _)| *cmp_id == id)
            .map(|(_, preset)| preset)
    }
}

const DEFAULT_CAT: &'static str = "Basic";

#[derive(Debug, Serialize, Deserialize)]
pub struct Presets {
    pub cats: Vec<(u64, Cat)>,
    pub next_cat_id: u64,
}
impl Presets {
    pub fn default() -> Self {
        let mut cat = Cat {
            name: String::from(DEFAULT_CAT),
            presets: Vec::new(),
            next_preset_id: 0,
        };
        for preset in [and_gate_preset(), not_gate_preset()] {
            let id = cat.next_preset_id;
            cat.next_preset_id += 1;
            cat.presets.push((id, preset));
        }
        Self {
            cats: vec![(0, cat)],
            next_cat_id: 1,
        }
    }
    pub fn merge(&mut self, other: &Self) {
        for cat in &other.cats {
            let Some(cat_idx) = self.cats.iter().position(|(_, n)| n.name == cat.1.name) else {
            	// cat doesn't already exist, just add it
            	self.cats.push((self.next_cat_id, cat.1.clone()));
            	self.next_cat_id += 1;
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

    pub fn add_cat(&mut self, name: String) -> Option<u64> {
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

        let id = self.next_cat_id;
        self.next_cat_id += 1;
        self.cats.push((id, Cat::new(name)));
        key_sort(&mut self.cats);
        Some(id)
    }
    pub fn remove_cat(&mut self, id: u64) -> bool {
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

    pub fn get_cat(&self, id: u64) -> Option<&Cat> {
        self.cats
            .iter()
            .find(|(cmp_id, _)| *cmp_id == id)
            .map(|(_, cat)| cat)
    }
    pub fn mut_cat(&mut self, id: u64) -> Option<&mut Cat> {
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
            inputs: [""; 2].map(str::to_owned).to_vec(),
            outputs: [""; 1].map(str::to_owned).to_vec(),
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
            inputs: [""; 1].map(str::to_owned).to_vec(),
            outputs: [""; 1].map(str::to_owned).to_vec(),
            table: not_truth_table(),
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
