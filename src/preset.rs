pub mod chip;

use crate::{BitField, SimId, TruthTable};
use eframe::egui::Color32;
use std::collections::HashMap;

#[derive(Default, Clone, Debug)]
pub struct IoLabel {
    pub name: String,
    pub implicit: bool,
}
impl IoLabel {
    #[inline(always)]
    pub fn implicit_input() -> Self {
        Self {
            name: String::from("input"),
            implicit: true,
        }
    }
    #[inline(always)]
    pub fn implicit_output() -> Self {
        Self {
            name: String::from("output"),
            implicit: true,
        }
    }

    #[inline(always)]
    pub fn implicit(name: &str) -> Self {
        Self {
            name: String::from(name),
            implicit: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CombGate {
    pub name: String,
    pub color: Color32,
    pub inputs: Vec<IoLabel>,
    pub outputs: Vec<IoLabel>,
    pub table: TruthTable,
}
impl<'a> crate::IoAccess<()> for CombGate {
    #[inline(always)]
    fn num_inputs(&self) -> usize {
        self.table.num_inputs
    }
    #[inline(always)]
    fn num_outputs(&self) -> usize {
        self.table.num_outputs
    }

    #[inline(always)]
    fn get_input(&self, input: usize) -> () {
        assert!(input < self.inputs.len());
        ()
    }
    #[inline(always)]
    fn get_output(&self, output: usize) -> () {
        assert!(output < self.outputs.len());
        ()
    }
}
impl CombGate {
    #[inline(always)]
    pub fn get_input_label(&self, input: usize) -> &IoLabel {
        &self.inputs[input]
    }
    #[inline(always)]
    pub fn get_output_label(&self, output: usize) -> &IoLabel {
        &self.outputs[output]
    }
}

pub type DeviceData = crate::DeviceData<(), chip::Chip, CombGate>;
impl DeviceData {
    pub fn size(&self) -> (f32, f32) {
        use crate::graphics::{DEVICE_IO_SIZE, DEVICE_IO_SP};

        let width = 40.0;
        let height = std::cmp::max(self.num_inputs(), self.num_outputs()) as f32
            * (DEVICE_IO_SIZE.y + DEVICE_IO_SP)
            + DEVICE_IO_SP;

        (width, height)
    }

    pub fn name(&self) -> &str {
        match self {
            Self::CombGate(e) => &e.name,
            Self::Chip(e) => &e.name,
            Self::Light(_) => "Light",
            Self::Switch(_) => "Switch",
        }
    }

    pub fn color(&self) -> Option<Color32> {
        match self {
            Self::CombGate(e) => Some(e.color.clone()),
            Self::Chip(e) => Some(e.color.clone()),
            Self::Light(_) => None,
            Self::Switch(_) => None,
        }
    }
}
impl DeviceData {
    pub fn get_input_label(&self, input: usize) -> IoLabel {
        match self {
            Self::CombGate(e) => e.get_input_label(input).clone(),
            Self::Chip(e) => e.inputs[input].label.clone(),
            Self::Light(_) => {
                assert_eq!(input, 0);
                IoLabel::implicit_input()
            }
            Self::Switch(_) => panic!("a switch doesnt have an input"),
        }
    }
    pub fn get_output_label(&self, output: usize) -> IoLabel {
        match self {
            Self::CombGate(e) => e.get_output_label(output).clone(),
            Self::Chip(e) => e.outputs[output].label.clone(),
            Self::Switch(_) => {
                assert_eq!(output, 0);
                IoLabel::implicit_output()
            }
            Self::Light(_) => panic!("a light doesnt have an output"),
        }
    }
}

pub struct Preset {
    pub category: SimId,
    pub device: DeviceData,
}
pub struct Presets {
    categories: HashMap<SimId, String>,
    next_category_id: SimId,

    presets: HashMap<SimId, Preset>,
    next_preset_id: SimId,
}
impl Presets {
    pub fn new() -> Self {
        Self {
            categories: HashMap::new(),
            next_category_id: SimId(0),

            presets: HashMap::new(),
            next_preset_id: SimId(0),
        }
    }
    pub fn defaults() -> Self {
        let mut new = Self::new();
        new.add_defaults();
        new
    }

    pub fn get_preset(&self, id: SimId) -> Option<&DeviceData> {
        self.presets.get(&id).map(|e| &e.device)
    }
    pub fn get_cat(&self, id: SimId) -> Option<&str> {
        self.categories.get(&id).map(String::as_str)
    }

    pub fn add_preset(&mut self, category: SimId, device: DeviceData) -> SimId {
        let id = self.next_preset_id;
        self.next_preset_id = SimId(id.0 + 1);
        self.presets.insert(id, Preset { category, device });
        id
    }
    pub fn add_presets(&mut self, category: SimId, devices: &[DeviceData]) -> Vec<SimId> {
        let mut ids = Vec::with_capacity(devices.len());
        for device in devices.iter().cloned() {
            let id = self.next_preset_id;
            self.next_preset_id = SimId(id.0 + 1);
            self.presets.insert(id, Preset { category, device });
            ids.push(id);
        }
        ids
    }

    pub fn remove_cat(&mut self, category: SimId) {
        self.categories.remove(&category);
        let mut remove_presets = Vec::new();
        for (preset_id, preset) in &self.presets {
            if preset.category == category {
                remove_presets.push(*preset_id);
            }
        }
        for preset in remove_presets {
            self.presets.remove(&preset);
        }
    }
    pub fn remove_preset(&mut self, preset: SimId) {
        self.presets.remove(&preset);
    }
    pub fn add_category(&mut self, category: String) -> SimId {
        let id = self.next_category_id;
        self.next_category_id = SimId(id.0 + 1);
        self.categories.insert(id, category);
        id
    }

    pub fn add_defaults(&mut self) {
        let cat_id = self.add_category("Basic".to_owned());
        self.add_presets(
            cat_id,
            &[
                DeviceData::CombGate(and_gate()),
                DeviceData::CombGate(not_gate()),
                DeviceData::CombGate(nand_gate()),
                DeviceData::CombGate(nor_gate()),
                DeviceData::CombGate(or_gate()),
            ],
        );
    }

    #[inline(always)]
    pub fn get_categories(&self) -> Vec<(SimId, String)> {
        let mut cats: Vec<_> = self
            .categories
            .iter()
            .map(|(id, name)| (*id, name.clone()))
            .collect();
        cats.sort_by(|(a_id, _), (b_id, _)| a_id.cmp(&b_id));
        cats
    }
    pub fn get_cat_presets(&self, cat: SimId) -> Vec<(SimId, &DeviceData)> {
        let mut result = Vec::new();
        for (preset_id, preset) in &self.presets {
            if preset.category == cat {
                result.push((*preset_id, &preset.device));
            }
        }
        result
    }
}

pub fn and_gate() -> CombGate {
    CombGate {
        name: String::from("And"),
        color: Color32::BLUE,
        inputs: vec![IoLabel::implicit("a"), IoLabel::implicit("b")],
        outputs: vec![IoLabel::implicit_output()],
        table: TruthTable {
            num_inputs: 2,
            num_outputs: 1,
            map: vec![
                BitField(0), // 00
                BitField(0), // 01
                BitField(0), // 10
                BitField(1), // 11
            ],
        },
    }
}
pub fn not_gate() -> CombGate {
    CombGate {
        name: String::from("Not"),
        color: Color32::GREEN,
        inputs: vec![IoLabel::implicit_input()],
        outputs: vec![IoLabel::implicit_output()],
        table: TruthTable {
            num_inputs: 1,
            num_outputs: 1,
            map: vec![
                BitField(1), // 0
                BitField(0), // 1
            ],
        },
    }
}

pub fn nand_gate() -> CombGate {
    CombGate {
        name: String::from("Nand"),
        color: Color32::TEMPORARY_COLOR,
        inputs: vec![IoLabel::implicit("a"), IoLabel::implicit("b")],
        outputs: vec![IoLabel::implicit_output()],
        table: TruthTable {
            num_inputs: 2,
            num_outputs: 1,
            map: vec![
                BitField(1), // 00
                BitField(1), // 01
                BitField(1), // 10
                BitField(0), // 11
            ],
        },
    }
}
pub fn nor_gate() -> CombGate {
    CombGate {
        name: String::from("Nor"),
        color: Color32::YELLOW,
        inputs: vec![IoLabel::implicit("a"), IoLabel::implicit("b")],
        outputs: vec![IoLabel::implicit_output()],
        table: TruthTable {
            num_inputs: 2,
            num_outputs: 1,
            map: vec![
                BitField(1), // 00
                BitField(0), // 01
                BitField(0), // 10
                BitField(0), // 11
            ],
        },
    }
}
pub fn or_gate() -> CombGate {
    CombGate {
        name: String::from("Or"),
        color: Color32::RED,
        inputs: vec![IoLabel::implicit("a"), IoLabel::implicit("b")],
        outputs: vec![IoLabel::implicit_output()],
        table: TruthTable {
            num_inputs: 2,
            num_outputs: 1,
            map: vec![
                BitField(0), // 00
                BitField(1), // 01
                BitField(1), // 10
                BitField(1), // 11
            ],
        },
    }
}

use crate::debug::{Fmtter, GoodDebug};
impl GoodDebug for Preset {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("Preset\n");
        f.indent();
        f.push_field("category", &self.category);
        f.push_field("device", &self.device);
        f.unindent();
    }
}
impl GoodDebug for Presets {
    fn good_debug(&self, f: &mut Fmtter) {
        f.push_str("Presets\n");
        f.indent();
        f.push_field("categories", &self.categories);
        f.push_field("next_category_id", &self.next_category_id);
        f.push_field("presets", &self.presets);
        f.push_field("next_preset_id", &self.next_preset_id);
        f.unindent();
    }
}
