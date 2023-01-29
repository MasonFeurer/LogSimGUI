pub mod chip;

use crate::board::Board;
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
    pub fn from_board(board: &mut Board) -> Result<Self, &'static str> {
        let original_board = board.clone();

        if board.inputs.len() > 64 {
            return Err("Too many inputs (max is 64)");
        }
        if board.outputs.len() > 64 {
            return Err("Too many outputs (max is 64)");
        }

        // create truth table from board
        let num_inputs = board.inputs.len();
        let num_outputs = board.outputs.len();
        let total_states: u64 = 1 << num_inputs;

        let inputs = board.inputs_sorted();
        let outputs = board.outputs_sorted();

        let mut output_states = Vec::with_capacity(total_states as usize);
        let mut input_state: u64 = 0;
        while input_state < total_states {
            // set inputs
            for i in 0..num_inputs {
                let state = ((input_state >> i as u64) & 1) == 1;
                board.set_input(inputs[i], state);
            }

            // execute queued writes
            let mut total_updates = 0;
            while board.write_queue.len() > 0 {
                if total_updates > 1000 {
                    return Err("Has a loop or is too big");
                }
                board.update();
                total_updates += 1;
            }

            // store output
            let mut output = BitField::empty(num_outputs);
            for i in 0..num_outputs {
                let state = board.outputs.get(&outputs[i]).unwrap().io.state;
                output.set(i, state);
            }
            output_states.push(output.data);

            input_state += 1;
        }

        let inputs = inputs
            .into_iter()
            .map(|id| board.inputs.get(&id).unwrap().io.name.clone())
            .collect();
        let outputs = outputs
            .into_iter()
            .map(|id| board.outputs.get(&id).unwrap().io.name.clone())
            .collect();
        *board = original_board;
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
pub enum BuiltinPreset {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PresetData {
    CombGate(CombGatePreset),
    Chip(ChipPreset),
    Builtin(BuiltinPreset),
}
impl PresetData {
    pub fn num_inputs(&self) -> usize {
        match self {
            Self::CombGate(e) => e.inputs.len(),
            Self::Chip(e) => e.inputs.len(),
            Self::Builtin(_) => todo!(),
        }
    }
    pub fn num_outputs(&self) -> usize {
        match self {
            Self::CombGate(e) => e.outputs.len(),
            Self::Chip(e) => e.outputs.len(),
            Self::Builtin(_) => todo!(),
        }
    }

    pub fn input_names(&self) -> &[String] {
        match self {
            Self::CombGate(e) => &e.inputs,
            Self::Chip(e) => &e.inputs,
            Self::Builtin(_) => todo!(),
        }
    }
    pub fn output_names(&self) -> &[String] {
        match self {
            Self::CombGate(e) => &e.outputs,
            Self::Chip(e) => &e.outputs,
            Self::Builtin(_) => todo!(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PresetSource {
    Default,
    Builtin,
    Board(Board),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DevicePreset {
    pub name: String,
    pub cat: String,
    pub color: [u8; 4],
    pub data: PresetData,
    pub src: PresetSource,
}

#[derive(Debug)]
pub enum Change {
    Removed,
    Added,
    Modified,
}

#[derive(Debug)]
pub struct Library {
    presets: Vec<DevicePreset>,
    changes: Vec<(String, Change)>,
}
impl Default for Library {
    fn default() -> Self {
        Self::new()
    }
}
impl Library {
    pub fn new() -> Self {
        Self {
            presets: default_presets().to_vec(),
            changes: Vec::new(),
        }
    }

    pub fn consume_changes(&mut self) -> Vec<(String, Change)> {
        let mut new = Vec::new();
        std::mem::swap(&mut self.changes, &mut new);
        new
    }
    pub fn preset_names(&self) -> Vec<String> {
        self.presets
            .iter()
            .map(|preset| preset.name.clone())
            .collect()
    }

    pub fn add_presets(&mut self, presets: &[DevicePreset]) {
        for preset in presets {
            self.add_preset(preset.clone(), true);
        }
    }
    pub fn add_preset(&mut self, preset: DevicePreset, save: bool) {
        let name = preset.name.clone();

        let change = if let Some(idx) = self.get_preset_idx(&name) {
            self.presets[idx] = preset.clone();
            Change::Modified
        } else {
            self.presets.push(preset.clone());
            Change::Added
        };
        if save {
            self.changes.push((name, change));
        }
    }
    pub fn remove_preset(&mut self, name: &str) {
        let idx = self.get_preset_idx(name).unwrap();
        self.presets.remove(idx);
        self.changes.push((name.to_owned(), Change::Removed));
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
    pub fn cat_names(&self) -> Vec<String> {
        let mut cats: Vec<String> = Vec::new();
        for preset in &self.presets {
            if cats.iter().find(|cat| **cat == preset.cat).is_none() {
                cats.push(preset.cat.clone());
            }
        }
        cats
    }
    pub fn cat_presets(&self, cat: &str) -> Vec<String> {
        let mut presets = Vec::new();
        for preset in &self.presets {
            if preset.cat.as_str() == cat {
                presets.push(preset.name.clone());
            }
        }
        presets
    }

    pub fn search_cats(&self, field: &str) -> Option<String> {
        if field.is_empty() {
            return None;
        }
        let mut results = self.cat_names();
        results.sort_by(|a, b| {
            let a_ml = str_match_level(a, field);
            let b_ml = str_match_level(b, field);
            a_ml.cmp(&b_ml).reverse()
        });
        match results.first() {
            Some(result) => {
                // if the result has a match level of 0 (doesn't match at all), return None
                if str_match_level(result, field) == 0 {
                    None
                } else {
                    Some(result.clone())
                }
            }
            None => None,
        }
    }
    pub fn search_presets(&self, field: &str) -> Vec<String> {
        if field.is_empty() {
            return Vec::new();
        }
        let mut results: Vec<_> = self
            .presets
            .iter()
            .map(|preset| preset.name.clone())
            .collect();
        results.sort_by(|a, b| {
            let a_ml = str_match_level(a, field);
            let b_ml = str_match_level(b, field);
            a_ml.cmp(&b_ml).reverse()
        });
        // Remove all results that have a match level of 0 (meaning they don't match at all)
        while let Some(last) = results.last() && str_match_level(last, field) == 0 {
        	results.pop();
        }
        results
    }
}

/// Checks how much a query matches a string
pub fn str_match_level(s: &str, q: &str) -> u8 {
    let (s, q) = (s.to_lowercase(), q.to_lowercase());
    match (s, q) {
        (s, q) if s == q => 200,
        (s, q) if s.starts_with(&q) => 100,
        (s, q) if s.contains(&q) => 50,
        _ => 0,
    }
}

fn default_presets() -> [DevicePreset; 2] {
    [
        DevicePreset {
            name: String::from("And"),
            cat: String::from("Basic"),
            color: [255, 0, 0, 255],
            data: PresetData::CombGate(CombGatePreset {
                inputs: [""; 2].map(str::to_owned).to_vec(),
                outputs: [""; 1].map(str::to_owned).to_vec(),
                table: TruthTable {
                    num_inputs: 2,
                    num_outputs: 1,
                    map: vec![0, 0, 0, 1],
                },
            }),
            src: PresetSource::Default,
        },
        DevicePreset {
            name: String::from("Not"),
            cat: String::from("Basic"),
            color: [0, 255, 0, 255],
            data: PresetData::CombGate(CombGatePreset {
                inputs: [""; 1].map(str::to_owned).to_vec(),
                outputs: [""; 1].map(str::to_owned).to_vec(),
                table: TruthTable {
                    num_inputs: 1,
                    num_outputs: 1,
                    map: vec![1, 0],
                },
            }),
            src: PresetSource::Default,
        },
    ]
}
