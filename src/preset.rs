pub mod chip;

use crate::scene::Scene;
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
    pub fn from_scene(scene: &mut Scene) -> Result<Self, String> {
        // create truth table from scene
        let num_inputs = scene.inputs.len();
        let num_outputs = scene.outputs.len();

        let inputs = scene.inputs_sorted();
        let outputs = scene.outputs_sorted();

        let total_states = {
            let mut temp = if inputs.is_empty() { 0 } else { 1 };
            for _ in 0..inputs.len() {
                temp *= 2;
            }
            temp
        };
        let mut output_states = Vec::with_capacity(total_states);

        // I do not care to support combination gates with 32+ inputs
        // (32 inputs would have ~2 billion states)
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
            output_states.push(output.data);

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
pub enum PresetSource {
    Default,
    Scene(Option<Scene>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DevicePreset {
    pub name: String,
    pub cat: String,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Presets {
    presets: Vec<DevicePreset>,
    dirty: Vec<String>,
    removed: Vec<String>,
}
impl Presets {
    pub fn default() -> Self {
        let mut presets = Self::new(vec![]);
        presets.add_preset(and_gate_preset());
        presets.add_preset(not_gate_preset());
        presets
    }
    pub fn new(presets: Vec<DevicePreset>) -> Self {
        Self {
            presets,
            dirty: Vec::new(),
            removed: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn consume_dirty(&mut self) -> Vec<String> {
        let mut new = Vec::new();
        std::mem::swap(&mut self.dirty, &mut new);
        new
    }
    #[inline(always)]
    pub fn consume_removed(&mut self) -> Vec<String> {
        let mut new = Vec::new();
        std::mem::swap(&mut self.removed, &mut new);
        new
    }
    #[inline(always)]
    pub fn get(&self) -> &[DevicePreset] {
        &self.presets
    }

    pub fn merge(&mut self, presets: &[DevicePreset]) {
        for preset in presets {
            if let Some(idx) = self.get_preset_idx(&preset.name) {
                self.presets[idx] = preset.clone();
            } else {
                self.presets.push(preset.clone());
            }
            self.dirty.push(preset.name.clone());
        }
    }

    pub fn add_preset(&mut self, preset: DevicePreset) {
        self.dirty.push(preset.name.clone());
        if let Some(idx) = self.get_preset_idx(&preset.name) {
            self.presets[idx] = preset;
        } else {
            self.presets.push(preset);
        }
    }
    pub fn remove_preset(&mut self, name: &str) {
        let idx = self.get_preset_idx(name).unwrap();
        if matches!(self.presets[idx].src, PresetSource::Default) {
            println!("Can't remove default preset");
            return;
        }
        self.presets.remove(idx);
        self.removed.push(String::from(name));
    }

    #[inline(always)]
    pub fn get_preset_idx(&self, name: &str) -> Option<usize> {
        self.presets
            .iter()
            .position(|preset| preset.name.as_str() == name)
    }
    #[inline(always)]
    pub fn get_preset(&self, name: &str) -> Option<&DevicePreset> {
        self.presets
            .iter()
            .find(|preset| preset.name.as_str() == name)
    }
    #[inline(always)]
    pub fn mut_preset(&mut self, name: &str) -> Option<&mut DevicePreset> {
        self.presets
            .iter_mut()
            .find(|preset| preset.name.as_str() == name)
    }

    pub fn cats_sorted(&self) -> Vec<(&str, Vec<&DevicePreset>)> {
        let mut cats: Vec<(&str, Vec<&DevicePreset>)> = Vec::new();
        for preset in &self.presets {
            if let Some(cat) = cats
                .iter_mut()
                .find(|(name, _)| *name == preset.cat.as_str())
            {
                cat.1.push(preset);
            } else {
                cats.push((preset.cat.as_str(), vec![preset]));
            }
        }
        cats
    }
    pub fn cat_presets(&self, cat: &str) -> Vec<&DevicePreset> {
        let mut presets = Vec::new();
        for preset in &self.presets {
            if preset.cat.as_str() == cat {
                presets.push(preset);
            }
        }
        presets
    }
}

pub fn and_gate_preset() -> DevicePreset {
    DevicePreset {
        name: String::from("And"),
        cat: String::from("Basic"),
        color: [255, 0, 0, 255],
        data: PresetData::CombGate(CombGatePreset {
            inputs: [""; 2].map(str::to_owned).to_vec(),
            outputs: [""; 1].map(str::to_owned).to_vec(),
            table: and_truth_table(),
        }),
        src: PresetSource::Default,
    }
}
pub fn not_gate_preset() -> DevicePreset {
    DevicePreset {
        name: String::from("Not"),
        cat: String::from("Basic"),
        color: [0, 255, 0, 255],
        data: PresetData::CombGate(CombGatePreset {
            inputs: [""; 1].map(str::to_owned).to_vec(),
            outputs: [""; 1].map(str::to_owned).to_vec(),
            table: not_truth_table(),
        }),
        src: PresetSource::Default,
    }
}

pub fn and_truth_table() -> TruthTable {
    TruthTable {
        num_inputs: 2,
        num_outputs: 1,
        map: vec![
            0, // 00
            0, // 01
            0, // 10
            1, // 11
        ],
    }
}
pub fn not_truth_table() -> TruthTable {
    TruthTable {
        num_inputs: 1,
        num_outputs: 1,
        map: vec![
            1, // 0
            0, // 1
        ],
    }
}
